//! Per-room projections reused between room-list snapshots.
//!
//! A snapshot used to project every joined room and rebuild the whole space
//! graph although a sync burst usually changes one room. The live owner
//! ([`super::NativeRoomListOwner`]) sees every change as an entries diff or a
//! wake and marks what it touched dirty here; a snapshot projects only dirty
//! or unknown rooms again and reuses the rest. Room identity and order still
//! come from the SDK list on every snapshot.
//!
//! While no owner task consumes diffs nothing marks rooms dirty, so the cache
//! is not trusted and every room is projected. The owner's safety pulse marks
//! everything dirty, so a change no stream reports heals within
//! [`super::ROOM_LIST_SAFETY_PULSE`].
//!
//! The cache lives on the session's `SyncServiceOwner`, so a new session
//! generation or a logout drops it with the owner.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use matrix_sdk::ruma::{OwnedRoomId, OwnedUserId, RoomId};

use crate::app::spaces::NativeSpaceParentEntry;
use crate::dto::{RoomEncryptionStatus, RoomSummary};

/// One room's projection as the last snapshot built it.
#[derive(Debug, Clone)]
pub(crate) struct CachedRoom {
    pub(crate) summary: RoomSummary,
    pub(crate) is_agent_room: bool,
}

impl CachedRoom {
    /// An unknown encryption state is probed again (throttled) on a later
    /// snapshot, and the answer can arrive without a room-list diff, so such
    /// a row is never reused.
    fn reusable(&self) -> bool {
        self.summary.encryption_status != RoomEncryptionStatus::Unknown
    }
}

#[derive(Debug)]
enum Dirty {
    All,
    Rooms(HashSet<OwnedRoomId>),
}

#[derive(Debug)]
struct CachedSpaceGraph {
    /// The listed rooms the graph was built for; a joined or left room
    /// changes the graph.
    rooms: HashSet<OwnedRoomId>,
    entries: Vec<NativeSpaceParentEntry>,
}

#[derive(Debug)]
struct CacheState {
    rooms: HashMap<OwnedRoomId, CachedRoom>,
    dirty: Dirty,
    space_graph: Option<CachedSpaceGraph>,
    space_graph_dirty: bool,
    /// The agent list `is_agent_room` was computed with.
    agents: Option<Vec<OwnedUserId>>,
}

impl Default for CacheState {
    fn default() -> Self {
        Self {
            rooms: HashMap::new(),
            dirty: Dirty::All,
            space_graph: None,
            space_graph_dirty: true,
            agents: None,
        }
    }
}

/// Room-list projection cache and change revision for one sync owner.
#[derive(Debug, Default)]
pub(crate) struct RoomListSnapshotCache {
    state: Mutex<CacheState>,
    /// One snapshot build at a time: a build consumes the dirty marks, so a
    /// second build running alongside could reuse a row the first is still
    /// projecting.
    build: tokio::sync::Mutex<()>,
    /// Coalesced changes the owner emitted so far. Lives here rather than in
    /// the owner so snapshot reads can compare against it.
    revision: AtomicU64,
    /// Owner tasks consuming entry diffs right now.
    trackers: AtomicUsize,
}

/// Rows and space graph a snapshot may reuse; everything else is projected.
#[derive(Debug, Default)]
pub(crate) struct SnapshotPlan {
    reusable: HashMap<OwnedRoomId, CachedRoom>,
    space_parents: Option<Vec<NativeSpaceParentEntry>>,
}

impl SnapshotPlan {
    pub(crate) fn take_room(&mut self, room_id: &RoomId) -> Option<CachedRoom> {
        self.reusable.remove(room_id)
    }

    pub(crate) fn take_space_parents(&mut self) -> Option<Vec<NativeSpaceParentEntry>> {
        self.space_parents.take()
    }
}

/// Counts an owner task as tracking changes until it is dropped.
#[derive(Debug)]
pub(crate) struct TrackingGuard {
    cache: Arc<RoomListSnapshotCache>,
}

impl Drop for TrackingGuard {
    fn drop(&mut self) {
        self.cache.trackers.fetch_sub(1, Ordering::AcqRel);
    }
}

impl RoomListSnapshotCache {
    fn lock(&self) -> Option<MutexGuard<'_, CacheState>> {
        self.state.lock().ok()
    }

    /// Coalesced changes emitted so far. Zero until the first change.
    pub(crate) fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }

    /// Record one emitted change and return its revision. Every dirty mark
    /// for that change is made before this call.
    pub(crate) fn next_revision(&self) -> u64 {
        self.revision.fetch_add(1, Ordering::AcqRel) + 1
    }

    /// Whether an owner task is marking changes, which the cache and an
    /// unchanged-revision answer both rely on.
    pub(crate) fn is_tracked(&self) -> bool {
        self.trackers.load(Ordering::Acquire) > 0
    }

    /// Start tracking for an owner task whose entries stream is open.
    /// Changes before it are unknown, so everything starts dirty.
    pub(crate) fn track(self: &Arc<Self>) -> TrackingGuard {
        self.invalidate_all();
        self.trackers.fetch_add(1, Ordering::AcqRel);
        TrackingGuard {
            cache: Arc::clone(self),
        }
    }

    pub(crate) fn invalidate_all(&self) {
        if let Some(mut state) = self.lock() {
            state.dirty = Dirty::All;
            state.space_graph_dirty = true;
        }
    }

    pub(crate) fn invalidate_rooms<'a>(&self, room_ids: impl IntoIterator<Item = &'a RoomId>) {
        let Some(mut state) = self.lock() else {
            return;
        };
        if let Dirty::Rooms(dirty) = &mut state.dirty {
            dirty.extend(room_ids.into_iter().map(ToOwned::to_owned));
        }
    }

    /// Drop rows for rooms that left the list.
    pub(crate) fn forget_rooms<'a>(&self, room_ids: impl IntoIterator<Item = &'a RoomId>) {
        let Some(mut state) = self.lock() else {
            return;
        };
        for room_id in room_ids {
            state.rooms.remove(room_id);
            if let Dirty::Rooms(dirty) = &mut state.dirty {
                dirty.remove(room_id);
            }
        }
    }

    pub(crate) fn invalidate_space_graph(&self) {
        if let Some(mut state) = self.lock() {
            state.space_graph_dirty = true;
        }
    }

    /// Serializes snapshot builds; hold it from [`Self::begin`] to
    /// [`Self::finish`].
    pub(crate) async fn build_gate(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.build.lock().await
    }

    /// Consume the dirty marks and say what this snapshot may reuse for the
    /// listed `room_ids`. Marks made after this call stay for the next one.
    pub(crate) fn begin(
        &self,
        room_ids: &HashSet<OwnedRoomId>,
        agents: &[OwnedUserId],
    ) -> SnapshotPlan {
        let tracked = self.is_tracked();
        let Some(mut state) = self.lock() else {
            return SnapshotPlan::default();
        };
        let dirty = std::mem::replace(&mut state.dirty, Dirty::Rooms(HashSet::new()));
        let space_graph_dirty = std::mem::take(&mut state.space_graph_dirty);
        let dirty = match dirty {
            Dirty::Rooms(dirty) if tracked && state.agents.as_deref() == Some(agents) => dirty,
            _ => return SnapshotPlan::default(),
        };
        let reusable = room_ids
            .iter()
            .filter(|room_id| !dirty.contains(*room_id))
            .filter_map(|room_id| {
                state
                    .rooms
                    .get(room_id)
                    .filter(|cached| cached.reusable())
                    .map(|cached| (room_id.clone(), cached.clone()))
            })
            .collect();
        let space_parents = state
            .space_graph
            .as_ref()
            .filter(|graph| !space_graph_dirty && graph.rooms == *room_ids)
            .map(|graph| graph.entries.clone());
        SnapshotPlan {
            reusable,
            space_parents,
        }
    }

    /// Keep this snapshot's rows for `room_ids` and its space graph (`None`
    /// when the graph read failed, so the next snapshot reads it again).
    pub(crate) fn finish(
        &self,
        room_ids: HashSet<OwnedRoomId>,
        fresh: Vec<(OwnedRoomId, CachedRoom)>,
        agents: &[OwnedUserId],
        space_parents: Option<Vec<NativeSpaceParentEntry>>,
    ) {
        let Some(mut state) = self.lock() else {
            return;
        };
        state.rooms.retain(|room_id, _| room_ids.contains(room_id));
        state.rooms.extend(fresh);
        state.agents = Some(agents.to_vec());
        state.space_graph = space_parents.map(|entries| CachedSpaceGraph {
            rooms: room_ids,
            entries,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::Membership;

    fn summary(room_id: &str, encryption_status: RoomEncryptionStatus) -> RoomSummary {
        RoomSummary {
            room_id: room_id.to_owned(),
            name: None,
            canonical_alias: None,
            avatar_url: None,
            membership: Membership::Join,
            is_direct: false,
            direct_user_id: None,
            is_space: false,
            is_call: false,
            has_active_call: false,
            active_call_participant_count: 0,
            is_favorite: false,
            is_low_priority: false,
            folder_id: None,
            encryption_status,
            state_encrypted: false,
            join_rule: None,
            unread_count: 0,
            highlight_count: 0,
            marked_unread: false,
            notification_mode: None,
            last_activity_ts: None,
            last_message_preview: None,
            last_message_is_agent_approval: false,
            heroes: None,
            tombstone_successor_room_id: None,
        }
    }

    fn room(id: &str) -> OwnedRoomId {
        OwnedRoomId::try_from(id).unwrap()
    }

    fn cached(id: &str) -> (OwnedRoomId, CachedRoom) {
        (
            room(id),
            CachedRoom {
                summary: summary(id, RoomEncryptionStatus::Encrypted),
                is_agent_room: false,
            },
        )
    }

    fn ids(list: &[&str]) -> HashSet<OwnedRoomId> {
        list.iter().map(|id| room(id)).collect()
    }

    fn graph() -> Vec<NativeSpaceParentEntry> {
        vec![NativeSpaceParentEntry {
            room_id: "!a:x".to_owned(),
            parent_ids: vec!["!s:x".to_owned()],
        }]
    }

    /// A tracked cache after one full snapshot of `!a` and `!b`.
    fn warm() -> (Arc<RoomListSnapshotCache>, TrackingGuard) {
        let cache = Arc::new(RoomListSnapshotCache::default());
        let guard = cache.track();
        let rooms = ids(&["!a:x", "!b:x"]);
        let plan = cache.begin(&rooms, &[]);
        assert!(plan.reusable.is_empty(), "a new tracker starts all dirty");
        assert!(plan.space_parents.is_none());
        cache.finish(
            rooms,
            vec![cached("!a:x"), cached("!b:x")],
            &[],
            Some(graph()),
        );
        (cache, guard)
    }

    #[test]
    fn clean_rooms_and_graph_are_reused_and_dirty_rooms_are_not() {
        let (cache, _guard) = warm();
        let rooms = ids(&["!a:x", "!b:x"]);
        let plan = cache.begin(&rooms, &[]);
        assert_eq!(plan.reusable.len(), 2);
        assert_eq!(plan.space_parents, Some(graph()));

        cache.invalidate_rooms([&*room("!b:x")]);
        let mut plan = cache.begin(&rooms, &[]);
        assert!(plan.take_room(&room("!a:x")).is_some());
        assert!(plan.take_room(&room("!b:x")).is_none());
        assert!(
            plan.space_parents.is_some(),
            "a room change keeps the graph"
        );
    }

    #[test]
    fn marks_made_during_a_build_survive_it() {
        let (cache, _guard) = warm();
        let rooms = ids(&["!a:x", "!b:x"]);
        let _plan = cache.begin(&rooms, &[]);
        cache.invalidate_rooms([&*room("!a:x")]);
        cache.invalidate_space_graph();
        cache.finish(rooms.clone(), vec![cached("!a:x")], &[], Some(graph()));
        let plan = cache.begin(&rooms, &[]);
        assert!(!plan.reusable.contains_key(&room("!a:x")));
        assert!(plan.space_parents.is_none());
    }

    #[test]
    fn untracked_or_all_dirty_or_new_agents_reuse_nothing() {
        let (cache, guard) = warm();
        let rooms = ids(&["!a:x", "!b:x"]);
        cache.invalidate_all();
        let plan = cache.begin(&rooms, &[]);
        assert!(plan.reusable.is_empty() && plan.space_parents.is_none());
        cache.finish(
            rooms.clone(),
            vec![cached("!a:x"), cached("!b:x")],
            &[],
            Some(graph()),
        );

        let agent = OwnedUserId::try_from("@agent:x").unwrap();
        let plan = cache.begin(&rooms, std::slice::from_ref(&agent));
        assert!(
            plan.reusable.is_empty(),
            "agent rooms depend on the agent list"
        );
        cache.finish(
            rooms.clone(),
            vec![cached("!a:x"), cached("!b:x")],
            &[],
            Some(graph()),
        );

        drop(guard);
        assert!(!cache.is_tracked());
        let plan = cache.begin(&rooms, &[]);
        assert!(plan.reusable.is_empty() && plan.space_parents.is_none());
    }

    #[test]
    fn a_changed_room_set_or_unknown_encryption_is_projected_again() {
        let (cache, _guard) = warm();
        let plan = cache.begin(&ids(&["!a:x", "!b:x", "!c:x"]), &[]);
        assert!(
            plan.space_parents.is_none(),
            "a joined room rebuilds the graph"
        );
        assert_eq!(plan.reusable.len(), 2);

        cache.forget_rooms([&*room("!b:x")]);
        let unknown = CachedRoom {
            summary: summary("!c:x", RoomEncryptionStatus::Unknown),
            is_agent_room: false,
        };
        cache.finish(
            ids(&["!a:x", "!c:x"]),
            vec![(room("!c:x"), unknown)],
            &[],
            None,
        );
        let plan = cache.begin(&ids(&["!a:x", "!b:x", "!c:x"]), &[]);
        assert_eq!(plan.reusable.len(), 1, "only !a is known and settled");
        assert!(plan.reusable.contains_key(&room("!a:x")));
        assert!(
            plan.space_parents.is_none(),
            "a failed graph read is not kept"
        );
    }
}
