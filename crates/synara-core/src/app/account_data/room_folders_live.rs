//! Live `in.synara.room_folders` read-modify-write owned by the shared core.
//!
//! [`RoomFoldersState`] lives on the session's `SyncServiceOwner`, so the
//! room-list snapshot and the folder commands share one projection:
//!
//! - Mutations are serialized by an async lock and read the server copy
//!   (`GET /account_data`) before writing, because a successful PUT does not
//!   reach matrix-sdk's store until it syncs back.
//! - After a successful write the written document is the visible projection
//!   until its sync echo arrives (or a short fail-safe expiry), so the room
//!   list updates at once and an older echo cannot flip it back. This is the
//!   room-notes pattern (`image_packs_live.rs`).
//! - A write wakes the room-list owner through [`RoomFoldersState::changed`].
//!
//! Matrix account data has no conditional write: two devices writing at the
//! same moment resolve as whole-document last-writer-wins.

use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use matrix_sdk::{
    ruma::{
        events::{AnyGlobalAccountDataEventContent, GlobalAccountDataEventType},
        serde::Raw,
        RoomId, TransactionId,
    },
    Client, RoomState,
};
use serde::Deserialize;
use serde_json::value::{to_raw_value, RawValue as RawJsonValue};
use tokio::sync::{Mutex as AsyncMutex, Notify};

use super::{
    create_room_folder, delete_room_folder, is_valid_room_folder_room_id, move_room_folder,
    parse_room_folders_json, prune_room_folders, rename_room_folder, set_room_folder,
    validate_room_folder_name, validate_room_folders_size, NativeRoomFolderCreateResult,
    NativeRoomFoldersSnapshot, RoomFoldersDocument, RoomFoldersError, MAX_ROOM_FOLDERS_READ_BYTES,
    ROOM_FOLDERS_EVENT_TYPE,
};

/// How long a local write stays visible without its sync echo, and how long
/// an observed sync event is preferred over the SDK store.
pub const ROOM_FOLDERS_PENDING_PROJECTION_TTL: Duration = Duration::from_secs(30);

fn room_folders_event_type() -> GlobalAccountDataEventType {
    GlobalAccountDataEventType::from(ROOM_FOLDERS_EVENT_TYPE)
}

fn parse_raw(
    raw: Option<Raw<AnyGlobalAccountDataEventContent>>,
) -> Result<RoomFoldersDocument, RoomFoldersError> {
    match raw {
        Some(raw) => parse_room_folders_json(raw.json().get()),
        None => Ok(RoomFoldersDocument::default()),
    }
}

#[derive(Deserialize)]
struct RawRoomFoldersSyncEvent {
    content: Box<RawJsonValue>,
}

/// Parse a global account-data sync event of type
/// [`ROOM_FOLDERS_EVENT_TYPE`], bounding the envelope before parsing.
pub fn parse_room_folders_sync_event(
    raw_event: &RawJsonValue,
) -> Result<RoomFoldersDocument, RoomFoldersError> {
    const MAX_SYNC_EVENT_OVERHEAD_BYTES: usize = 256;
    if raw_event.get().len() > MAX_ROOM_FOLDERS_READ_BYTES + MAX_SYNC_EVENT_OVERHEAD_BYTES {
        return Err(RoomFoldersError::ContentTooLarge);
    }
    let event: RawRoomFoldersSyncEvent =
        serde_json::from_str(raw_event.get()).map_err(|_| RoomFoldersError::DeserializeFailed)?;
    parse_room_folders_json(event.content.get())
}

#[derive(Default)]
struct Projection {
    mutation_in_flight: bool,
    pending: Option<(RoomFoldersDocument, Instant)>,
    synchronized: Option<(Result<RoomFoldersDocument, RoomFoldersError>, Instant)>,
}

impl Projection {
    fn finish_mutation(&mut self, written: Option<RoomFoldersDocument>, now: Instant) {
        self.mutation_in_flight = false;
        if let Some(written) = written {
            self.pending = Some((written, now));
        }
    }

    fn observe(&mut self, content: Result<RoomFoldersDocument, RoomFoldersError>, now: Instant) {
        // A sync that lands while the RMW runs may predate the PUT.
        if self.mutation_in_flight {
            return;
        }
        // While a local write waits for its echo, any other event is
        // ambiguous (a later writer, or a delayed pre-PUT sync). Keep the
        // local write until its echo or the expiry; after the expiry the SDK
        // store decides.
        if let Some((pending, _)) = self.pending.as_ref() {
            if !content.as_ref().is_ok_and(|next| next == pending) {
                return;
            }
        }
        self.synchronized = Some((content, now));
        self.pending = None;
    }

    fn project(
        &mut self,
        stored: Result<RoomFoldersDocument, RoomFoldersError>,
        now: Instant,
    ) -> Result<RoomFoldersDocument, RoomFoldersError> {
        if let Some((pending, at)) = self.pending.as_ref() {
            if now.saturating_duration_since(*at) < ROOM_FOLDERS_PENDING_PROJECTION_TTL {
                return Ok(pending.clone());
            }
            self.pending = None;
        }
        if let Some((observed, at)) = self.synchronized.as_ref() {
            if now.saturating_duration_since(*at) < ROOM_FOLDERS_PENDING_PROJECTION_TTL {
                return observed.clone();
            }
            self.synchronized = None;
        }
        stored
    }
}

/// Clears the in-flight flag if a mutation future is dropped mid-way.
struct InFlight<'a> {
    state: &'a RoomFoldersState,
    finished: bool,
}

impl InFlight<'_> {
    fn finish(mut self, written: Option<RoomFoldersDocument>) {
        self.state
            .projection()
            .finish_mutation(written, Instant::now());
        self.finished = true;
    }
}

impl Drop for InFlight<'_> {
    fn drop(&mut self) {
        if !self.finished {
            self.state.projection().mutation_in_flight = false;
        }
    }
}

/// One session's room-folders projection and mutation lock.
#[derive(Default)]
pub struct RoomFoldersState {
    // Matrix global account data has no conditional write; keep this
    // client's RMW single-writer.
    mutation: AsyncMutex<()>,
    projection: Mutex<Projection>,
    changed: Notify,
}

impl std::fmt::Debug for RoomFoldersState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoomFoldersState").finish_non_exhaustive()
    }
}

/// What a room can be filed as, read from the client's room state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomFolderEligibility {
    Eligible,
    NotJoined,
    Direct,
    Space,
}

impl RoomFolderEligibility {
    fn check(self) -> Result<(), RoomFoldersError> {
        match self {
            Self::Eligible => Ok(()),
            Self::NotJoined => Err(RoomFoldersError::RoomNotJoined),
            Self::Direct => Err(RoomFoldersError::RoomIsDirect),
            Self::Space => Err(RoomFoldersError::RoomIsSpace),
        }
    }
}

/// Whether `room_id` may be filed: joined, not a DM, not a space.
pub async fn room_folder_eligibility(
    client: &Client,
    room_id: &str,
) -> Result<RoomFolderEligibility, RoomFoldersError> {
    if !is_valid_room_folder_room_id(room_id) {
        return Err(RoomFoldersError::InvalidRoom);
    }
    let room_id = RoomId::parse(room_id).map_err(|_| RoomFoldersError::InvalidRoom)?;
    let Some(room) = client.get_room(&room_id) else {
        return Ok(RoomFolderEligibility::NotJoined);
    };
    if room.state() != RoomState::Joined {
        return Ok(RoomFolderEligibility::NotJoined);
    }
    if room.is_space() {
        return Ok(RoomFolderEligibility::Space);
    }
    if room.is_direct().await.unwrap_or(false) {
        return Ok(RoomFolderEligibility::Direct);
    }
    Ok(RoomFolderEligibility::Eligible)
}

/// Keep a filed room unless the client knows it and the user is no longer
/// joined. Rooms the store does not know yet (before the first sync) stay.
fn still_joined(client: &Client, room_id: &str) -> bool {
    let Ok(room_id) = RoomId::parse(room_id) else {
        return false;
    };
    client
        .get_room(&room_id)
        .is_none_or(|room| room.state() == RoomState::Joined)
}

/// A new opaque folder id.
pub fn new_room_folder_id() -> String {
    TransactionId::new().to_string()
}

impl RoomFoldersState {
    fn projection(&self) -> MutexGuard<'_, Projection> {
        self.projection
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Resolves after the next local folder write (one stored wake-up if
    /// nobody is waiting). The room-list owner selects on it.
    pub async fn changed(&self) {
        self.changed.notified().await;
    }

    /// Feed a synchronized `in.synara.room_folders` event (raw sync JSON).
    pub fn observe_sync_event(&self, raw_event: &RawJsonValue) {
        let content = parse_room_folders_sync_event(raw_event);
        self.projection().observe(content, Instant::now());
    }

    /// The folders as this client should show them now: a pending local
    /// write, else a recent sync event, else the SDK store.
    pub async fn current(&self, client: &Client) -> Result<RoomFoldersDocument, RoomFoldersError> {
        let stored = client
            .account()
            .account_data_raw(room_folders_event_type())
            .await
            .map_err(|_| RoomFoldersError::FetchFailed)
            .and_then(parse_raw);
        self.projection().project(stored, Instant::now())
    }

    pub async fn snapshot(
        &self,
        client: &Client,
        session_generation: u64,
    ) -> Result<NativeRoomFoldersSnapshot, RoomFoldersError> {
        Ok(self.current(client).await?.snapshot(session_generation))
    }

    async fn fetch_fresh(client: &Client) -> Result<RoomFoldersDocument, RoomFoldersError> {
        let raw = client
            .account()
            .fetch_account_data(room_folders_event_type())
            .await
            .map_err(|_| RoomFoldersError::FetchFailed)?;
        parse_raw(raw)
    }

    async fn store(
        client: &Client,
        document: &RoomFoldersDocument,
    ) -> Result<(), RoomFoldersError> {
        validate_room_folders_size(document)?;
        let raw_value =
            to_raw_value(&document.to_json()).map_err(|_| RoomFoldersError::SerializeFailed)?;
        client
            .account()
            .set_account_data_raw(room_folders_event_type(), Raw::from_json(raw_value))
            .await
            .map_err(|_| RoomFoldersError::SetFailed)?;
        Ok(())
    }

    /// Serialized RMW: fetch the server copy, apply `mutate`, prune rooms the
    /// user left, write, and project the result. A mutation that changes
    /// nothing writes nothing (and so never writes only to prune).
    async fn mutate<F>(
        &self,
        client: &Client,
        mutate: F,
    ) -> Result<RoomFoldersDocument, RoomFoldersError>
    where
        F: FnOnce(RoomFoldersDocument) -> Result<RoomFoldersDocument, RoomFoldersError>,
    {
        let _serialized = self.mutation.lock().await;
        self.projection().mutation_in_flight = true;
        let in_flight = InFlight {
            state: self,
            finished: false,
        };
        let fresh = Self::fetch_fresh(client).await?;
        let next = mutate(fresh.clone())?;
        if next == fresh {
            in_flight.finish(None);
            return Ok(fresh);
        }
        let next = prune_room_folders(next, |room_id| still_joined(client, room_id));
        Self::store(client, &next).await?;
        in_flight.finish(Some(next.clone()));
        self.changed.notify_one();
        Ok(next)
    }

    async fn require_eligible(client: &Client, room_id: &str) -> Result<(), RoomFoldersError> {
        room_folder_eligibility(client, room_id).await?.check()
    }

    /// Append a folder named `name`; with `room_id`, file that room in it in
    /// the same write.
    pub async fn create(
        &self,
        client: &Client,
        session_generation: u64,
        name: &str,
        room_id: Option<&str>,
    ) -> Result<NativeRoomFolderCreateResult, RoomFoldersError> {
        validate_room_folder_name(name)?;
        if let Some(room_id) = room_id {
            Self::require_eligible(client, room_id).await?;
        }
        let folder_id = new_room_folder_id();
        let document = self
            .mutate(client, |document| {
                let document = create_room_folder(document, &folder_id, name)?;
                match room_id {
                    Some(room_id) => set_room_folder(document, room_id, Some(&folder_id)),
                    None => Ok(document),
                }
            })
            .await?;
        Ok(NativeRoomFolderCreateResult {
            folder_id,
            snapshot: document.snapshot(session_generation),
        })
    }

    pub async fn rename(
        &self,
        client: &Client,
        session_generation: u64,
        folder_id: &str,
        name: &str,
    ) -> Result<NativeRoomFoldersSnapshot, RoomFoldersError> {
        validate_room_folder_name(name)?;
        let document = self
            .mutate(client, |document| {
                rename_room_folder(document, folder_id, name)
            })
            .await?;
        Ok(document.snapshot(session_generation))
    }

    pub async fn delete(
        &self,
        client: &Client,
        session_generation: u64,
        folder_id: &str,
    ) -> Result<NativeRoomFoldersSnapshot, RoomFoldersError> {
        let document = self
            .mutate(client, |document| delete_room_folder(document, folder_id))
            .await?;
        Ok(document.snapshot(session_generation))
    }

    pub async fn move_to(
        &self,
        client: &Client,
        session_generation: u64,
        folder_id: &str,
        to_index: usize,
    ) -> Result<NativeRoomFoldersSnapshot, RoomFoldersError> {
        let document = self
            .mutate(client, |document| {
                move_room_folder(document, folder_id, to_index)
            })
            .await?;
        Ok(document.snapshot(session_generation))
    }

    /// File `room_id` in `folder_id`, or unfile it with `None`. Filing needs
    /// a joined room that is neither a DM nor a space; unfiling any valid id
    /// is allowed so stale entries can be removed.
    pub async fn set_room(
        &self,
        client: &Client,
        session_generation: u64,
        room_id: &str,
        folder_id: Option<&str>,
    ) -> Result<NativeRoomFoldersSnapshot, RoomFoldersError> {
        if !is_valid_room_folder_room_id(room_id) {
            return Err(RoomFoldersError::InvalidRoom);
        }
        if folder_id.is_some() {
            Self::require_eligible(client, room_id).await?;
        }
        let document = self
            .mutate(client, |document| {
                set_room_folder(document, room_id, folder_id)
            })
            .await?;
        Ok(document.snapshot(session_generation))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::app::account_data::normalize_room_folders_content;

    fn doc(name: &str) -> RoomFoldersDocument {
        normalize_room_folders_content(Some(&json!({
            "version": 1,
            "folders": [{ "id": "a", "name": name, "rooms": ["!r:s"] }]
        })))
    }

    fn event(document: &RoomFoldersDocument) -> Box<RawJsonValue> {
        RawJsonValue::from_string(
            json!({ "type": ROOM_FOLDERS_EVENT_TYPE, "content": document.to_json() }).to_string(),
        )
        .unwrap()
    }

    #[test]
    fn sync_events_are_bounded_and_parsed() {
        let parsed = parse_room_folders_sync_event(&event(&doc("A"))).unwrap();
        assert_eq!(parsed, doc("A"));
        let huge = RawJsonValue::from_string(format!(
            "{{\"type\":\"x\",\"content\":{{\"pad\":\"{}\"}}}}",
            "p".repeat(MAX_ROOM_FOLDERS_READ_BYTES + 300)
        ))
        .unwrap();
        assert_eq!(
            parse_room_folders_sync_event(&huge),
            Err(RoomFoldersError::ContentTooLarge)
        );
    }

    #[test]
    fn a_local_write_wins_until_its_echo_and_older_echoes_do_not_flicker() {
        let now = Instant::now();
        let mut projection = Projection::default();
        let store = || Ok(doc("store"));
        assert_eq!(projection.project(store(), now), Ok(doc("store")));

        // Write A, then write B before A's echo.
        projection.mutation_in_flight = true;
        projection.finish_mutation(Some(doc("A")), now);
        projection.mutation_in_flight = true;
        // A sync that lands mid-RMW is ignored.
        projection.observe(Ok(doc("before")), now);
        projection.finish_mutation(Some(doc("B")), now);
        assert_eq!(projection.project(store(), now), Ok(doc("B")));

        // A's echo arrives: B stays.
        projection.observe(Ok(doc("A")), now);
        assert_eq!(projection.project(store(), now), Ok(doc("B")));
        // B's echo acknowledges the write; it stays visible while the store
        // may still hold the older copy.
        projection.observe(Ok(doc("B")), now);
        assert_eq!(projection.project(store(), now), Ok(doc("B")));
        // Another device's later write is shown at once.
        projection.observe(Ok(doc("C")), now);
        assert_eq!(projection.project(store(), now), Ok(doc("C")));
        // After the window the SDK store decides.
        let later = now + ROOM_FOLDERS_PENDING_PROJECTION_TTL;
        assert_eq!(projection.project(store(), later), Ok(doc("store")));
    }

    #[test]
    fn an_unacknowledged_write_expires_into_the_store() {
        let now = Instant::now();
        let mut projection = Projection::default();
        projection.finish_mutation(Some(doc("A")), now);
        // A different writer's event while A waits for its echo is ambiguous.
        projection.observe(Ok(doc("other")), now);
        assert_eq!(projection.project(Ok(doc("store")), now), Ok(doc("A")));
        let later = now + ROOM_FOLDERS_PENDING_PROJECTION_TTL;
        assert_eq!(
            projection.project(Ok(doc("store")), later),
            Ok(doc("store"))
        );
    }

    #[test]
    fn a_dropped_mutation_clears_the_in_flight_flag() {
        let state = RoomFoldersState::default();
        {
            state.projection().mutation_in_flight = true;
            let _in_flight = InFlight {
                state: &state,
                finished: false,
            };
        }
        assert!(!state.projection().mutation_in_flight);
    }

    #[test]
    fn folder_ids_are_valid_and_distinct() {
        let first = new_room_folder_id();
        let second = new_room_folder_id();
        assert!(crate::app::account_data::is_valid_room_folder_id(&first));
        assert_ne!(first, second);
    }

    #[tokio::test]
    async fn invalid_requests_fail_before_network_io() {
        let client = Client::builder()
            .homeserver_url("http://127.0.0.1:9")
            .build()
            .await
            .expect("build isolated client");
        let state = RoomFoldersState::default();
        assert_eq!(
            state.create(&client, 1, "  ", None).await,
            Err(RoomFoldersError::InvalidName)
        );
        assert_eq!(
            state.rename(&client, 1, "a", "").await,
            Err(RoomFoldersError::InvalidName)
        );
        assert_eq!(
            state.set_room(&client, 1, "not-a-room", None).await,
            Err(RoomFoldersError::InvalidRoom)
        );
        // An unknown room cannot be filed.
        assert_eq!(
            state.set_room(&client, 1, "!unknown:s", Some("a")).await,
            Err(RoomFoldersError::RoomNotJoined)
        );
        assert_eq!(
            state.create(&client, 1, "New", Some("!unknown:s")).await,
            Err(RoomFoldersError::RoomNotJoined)
        );
    }
}
