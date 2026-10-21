//! Live V-ROOMS.2 space ownership.
//!
//! - 2a: child→parents map for nav/unread rollup
//! - 2b: typed hierarchy-summary reads
//! - 2c: local joined-space child graph + m.space.child / restricted-join mutations

use std::collections::{BTreeMap, BTreeSet};

use futures_util::StreamExt;
use matrix_sdk::{
    deserialized_responses::RawSyncOrStrippedState,
    room::ParentSpace,
    ruma::{
        api::client::space::get_hierarchy,
        events::{
            room::join_rules::RoomJoinRulesEventContent,
            space::{child::SpaceChildEventContent, parent::SpaceParentEventContent},
            StateEventType,
        },
        room::JoinRuleSummary,
        room::{AllowRule, JoinRule},
        OwnedRoomId, OwnedServerName, SpaceChildOrder, UInt,
    },
    Client, Room, RoomState,
};
use serde_json::json;

use super::{
    would_introduce_cycle, NativeRestrictedJoinReparentResult, NativeSpaceChildEdge,
    NativeSpaceChildMutationResult, NativeSpaceChildrenSnapshot, NativeSpaceHierarchyRoom,
    NativeSpaceHierarchySnapshot, NativeSpaceParentEntry, NativeSpaceParentsSnapshot,
};

const HIERARCHY_PAGE_LIMIT: u64 = 100;
const HIERARCHY_MAX_PAGES: usize = 50;

/// Child to parent-space map for grouping and unread rollup.
///
/// Edges come from each joined space's `m.space.child` events and, as in
/// Element, from a joined room's own `m.space.parent` events that the SDK
/// (`Room::parent_spaces`, the spec algorithm) finds legitimate: the joined
/// parent lists the room, or the parent event's sender may add children to
/// that parent. The child list alone missed rooms that declare a parent the
/// space does not list.
pub async fn snapshot_space_parents(
    client: &Client,
    session_generation: u64,
) -> Result<NativeSpaceParentsSnapshot, &'static str> {
    let mut edges: Vec<(String, String)> = collect_valid_edges(client)
        .await?
        .into_iter()
        .map(|edge| (edge.parent_id, edge.child_id))
        .collect();
    edges.extend(declared_parent_edges(client).await);
    let mut child_to_parents: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for (parent_id, child_id) in edges {
        if child_id == parent_id {
            continue;
        }
        if child_to_parents
            .get(&child_id)
            .is_some_and(|parents| parents.contains(&parent_id))
        {
            continue;
        }
        if would_introduce_cycle(&child_to_parents, &parent_id, &child_id) {
            continue;
        }
        child_to_parents
            .entry(child_id)
            .or_default()
            .insert(parent_id);
    }

    let mut entries: Vec<NativeSpaceParentEntry> = child_to_parents
        .into_iter()
        .map(|(room_id, parents)| NativeSpaceParentEntry {
            room_id,
            parent_ids: parents.into_iter().collect(),
        })
        .collect();
    entries.sort_by(|a, b| a.room_id.cmp(&b.room_id));

    Ok(NativeSpaceParentsSnapshot {
        session_generation,
        entries,
    })
}

pub async fn snapshot_space_children(
    client: &Client,
    session_generation: u64,
) -> Result<NativeSpaceChildrenSnapshot, &'static str> {
    let mut edges = collect_valid_edges(client).await?;
    edges.sort_by(|a, b| {
        a.parent_id
            .cmp(&b.parent_id)
            .then_with(|| a.child_id.cmp(&b.child_id))
    });
    Ok(NativeSpaceChildrenSnapshot {
        session_generation,
        edges,
    })
}

pub async fn snapshot_space_hierarchy(
    client: &Client,
    session_generation: u64,
    room_id: &str,
) -> Result<NativeSpaceHierarchySnapshot, &'static str> {
    let room_id =
        OwnedRoomId::try_from(room_id).map_err(|_| "v-rooms.2b-space-hierarchy-invalid-room")?;
    let mut next_batch = None;
    let mut rooms = BTreeMap::new();

    for _ in 0..HIERARCHY_MAX_PAGES {
        let mut request = get_hierarchy::v1::Request::new(room_id.clone());
        request.from = next_batch.take();
        request.limit = UInt::new(HIERARCHY_PAGE_LIMIT);
        request.max_depth = UInt::new(1);
        let response = client
            .send(request)
            .await
            .map_err(|_| "v-rooms.2b-space-hierarchy-read-failed")?;

        for chunk in response.rooms {
            let summary = chunk.summary;
            let allowed_room_ids = match &summary.join_rule {
                JoinRuleSummary::Restricted(rule) | JoinRuleSummary::KnockRestricted(rule) => rule
                    .allowed_room_ids
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
                _ => Vec::new(),
            };
            let room = NativeSpaceHierarchyRoom {
                room_id: summary.room_id.to_string(),
                name: summary.name,
                canonical_alias: summary.canonical_alias.map(|value| value.to_string()),
                topic: summary.topic,
                avatar_url: summary.avatar_url.map(|value| value.to_string()),
                room_type: summary.room_type.map(|value| value.to_string()),
                num_joined_members: summary.num_joined_members.into(),
                join_rule: summary.join_rule.as_str().to_owned(),
                allowed_room_ids,
                world_readable: summary.world_readable,
                guest_can_join: summary.guest_can_join,
            };
            rooms.insert(room.room_id.clone(), room);
        }

        next_batch = response.next_batch;
        if next_batch.is_none() {
            return Ok(NativeSpaceHierarchySnapshot {
                session_generation,
                rooms: rooms.into_values().collect(),
            });
        }
    }

    Err("v-rooms.2b-space-hierarchy-page-limit")
}

pub async fn set_space_child(
    client: &Client,
    parent_id: &str,
    child_id: &str,
    via: &[String],
    order: Option<&str>,
    suggested: Option<bool>,
) -> Result<NativeSpaceChildMutationResult, &'static str> {
    let parent = parse_room_id(parent_id, "v-rooms.2c-invalid-parent")?;
    let child = parse_room_id(child_id, "v-rooms.2c-invalid-child")?;
    let room = joined_room(client, &parent)?;

    let mut servers = Vec::with_capacity(via.len());
    for server in via {
        let parsed = server
            .trim()
            .parse::<OwnedServerName>()
            .map_err(|_| "v-rooms.2c-invalid-via")?;
        servers.push(parsed);
    }

    let mut content = SpaceChildEventContent::new(servers);
    if let Some(order_raw) = order.map(str::trim).filter(|value| !value.is_empty()) {
        content.order = Some(
            SpaceChildOrder::parse(order_raw)
                .map_err(|_| "v-rooms.2c-invalid-order")?
                .to_owned(),
        );
    }
    if let Some(suggested) = suggested {
        content.suggested = suggested;
    }

    room.send_state_event_for_key(&child, content)
        .await
        .map_err(|_| "v-rooms.2c-space-child-set-failed")?;

    Ok(NativeSpaceChildMutationResult {
        parent_id: parent.to_string(),
        child_id: child.to_string(),
        status: crate::dto::MutationStatus::Updated,
    })
}

pub async fn remove_space_child(
    client: &Client,
    parent_id: &str,
    child_id: &str,
) -> Result<NativeSpaceChildMutationResult, &'static str> {
    let parent = parse_room_id(parent_id, "v-rooms.2c-invalid-parent")?;
    let child = parse_room_id(child_id, "v-rooms.2c-invalid-child")?;
    let room = joined_room(client, &parent)?;

    // Product remove path posts empty content so the edge fails isValidChild.
    room.send_state_event_raw("m.space.child", child.as_str(), json!({}))
        .await
        .map_err(|_| "v-rooms.2c-space-child-remove-failed")?;
    clear_space_parent(client, &child, &parent).await;

    Ok(NativeSpaceChildMutationResult {
        parent_id: parent.to_string(),
        child_id: child.to_string(),
        status: crate::dto::MutationStatus::Removed,
    })
}

/// Best effort, as the SDK's `SpaceService::remove_child_from_space` does:
/// empty the child's `m.space.parent` for this space when the user may, so a
/// stale parent link does not keep grouping the room under the space. The
/// child edge is already removed, so a failure here is not reported.
async fn clear_space_parent(client: &Client, child: &OwnedRoomId, parent: &OwnedRoomId) {
    let Some(child_room) = client.get_room(child) else {
        return;
    };
    let Some(user_id) = client.user_id() else {
        return;
    };
    let can_send = child_room
        .power_levels()
        .await
        .is_ok_and(|levels| levels.user_can_send_state(user_id, StateEventType::SpaceParent));
    let has_parent = matches!(
        child_room
            .get_state_event_static_for_key::<SpaceParentEventContent, _>(parent)
            .await,
        Ok(Some(_))
    );
    if can_send && has_parent {
        let _ = child_room
            .send_state_event_raw("m.space.parent", parent.as_str(), json!({}))
            .await;
    }
}

/// Replace a restricted/knock_restricted allow membership when a room is
/// dragged between spaces (product Lobby reorder coordination).
pub async fn reparent_restricted_join_allow(
    client: &Client,
    room_id: &str,
    remove_parent_id: Option<&str>,
    add_parent_id: &str,
) -> Result<NativeRestrictedJoinReparentResult, &'static str> {
    let room_id = parse_room_id(room_id, "v-rooms.2c-invalid-room")?;
    let add_parent = parse_room_id(add_parent_id, "v-rooms.2c-invalid-parent")?;
    let remove_parent = match remove_parent_id {
        Some(id) if !id.trim().is_empty() => Some(parse_room_id(id, "v-rooms.2c-invalid-parent")?),
        _ => None,
    };
    let room = joined_room(client, &room_id)?;

    let raw = room
        .get_state_event_static::<RoomJoinRulesEventContent>()
        .await
        .map_err(|_| "v-rooms.2c-join-rules-read-failed")?;
    let Some(raw) = raw else {
        return Ok(NativeRestrictedJoinReparentResult {
            room_id: room_id.to_string(),
            status: crate::dto::MutationStatus::Skipped,
        });
    };
    let event = match raw {
        RawSyncOrStrippedState::Sync(raw) => raw
            .deserialize()
            .map_err(|_| "v-rooms.2c-join-rules-deserialize-failed")?,
        RawSyncOrStrippedState::Stripped(_) => {
            return Ok(NativeRestrictedJoinReparentResult {
                room_id: room_id.to_string(),
                status: crate::dto::MutationStatus::Skipped,
            });
        }
    };
    let Some(original) = event.as_original() else {
        return Ok(NativeRestrictedJoinReparentResult {
            room_id: room_id.to_string(),
            status: crate::dto::MutationStatus::Skipped,
        });
    };

    let content = match &original.content.join_rule {
        JoinRule::Restricted(restricted) => RoomJoinRulesEventContent::restricted(
            reparent_allow_list(&restricted.allow, remove_parent.as_ref(), &add_parent),
        ),
        JoinRule::KnockRestricted(restricted) => RoomJoinRulesEventContent::knock_restricted(
            reparent_allow_list(&restricted.allow, remove_parent.as_ref(), &add_parent),
        ),
        _ => {
            return Ok(NativeRestrictedJoinReparentResult {
                room_id: room_id.to_string(),
                status: crate::dto::MutationStatus::Skipped,
            });
        }
    };

    room.send_state_event(content)
        .await
        .map_err(|_| "v-rooms.2c-join-rules-set-failed")?;

    Ok(NativeRestrictedJoinReparentResult {
        room_id: room_id.to_string(),
        status: crate::dto::MutationStatus::Updated,
    })
}

fn reparent_allow_list(
    allow: &[AllowRule],
    remove_parent: Option<&OwnedRoomId>,
    add_parent: &OwnedRoomId,
) -> Vec<AllowRule> {
    let mut next: Vec<AllowRule> = allow
        .iter()
        .filter(|rule| match (rule, remove_parent) {
            (AllowRule::RoomMembership(membership), Some(remove)) => membership.room_id != *remove,
            _ => true,
        })
        .cloned()
        .collect();
    if !next.iter().any(|rule| match rule {
        AllowRule::RoomMembership(membership) => membership.room_id == *add_parent,
        _ => false,
    }) {
        next.push(AllowRule::room_membership(add_parent.clone()));
    }
    next
}

/// `(parent, child)` for every joined room's `m.space.parent` that names a
/// joined space and that the SDK accepts. A reciprocal parent is already an
/// `m.space.child` edge; one the parent does not list counts only when the
/// event's sender has the power to add children there (`WithPowerlevel`).
/// Unverifiable and illegitimate parents are ignored. A failed read skips
/// that room rather than the whole graph.
async fn declared_parent_edges(client: &Client) -> Vec<(String, String)> {
    let mut edges = Vec::new();
    for room in client.joined_rooms() {
        let Ok(parents) = room.parent_spaces().await else {
            continue;
        };
        futures_util::pin_mut!(parents);
        while let Some(parent) = parents.next().await {
            let parent = match parent {
                Ok(ParentSpace::Reciprocal(parent)) => parent,
                // The parent never listed the room. An `m.space.child` it
                // emptied (Synara's and the SDK's remove path) is an explicit
                // removal that a stale `m.space.parent` must not undo.
                Ok(ParentSpace::WithPowerlevel(parent))
                    if !matches!(
                        parent
                            .get_state_event_static_for_key::<SpaceChildEventContent, _>(
                                room.room_id()
                            )
                            .await,
                        Ok(Some(_))
                    ) =>
                {
                    parent
                }
                _ => continue,
            };
            if parent.state() == RoomState::Joined && parent.is_space() {
                edges.push((parent.room_id().to_string(), room.room_id().to_string()));
            }
        }
    }
    edges
}

async fn collect_valid_edges(client: &Client) -> Result<Vec<NativeSpaceChildEdge>, &'static str> {
    let mut edges = Vec::new();
    for room in client.joined_rooms() {
        if room.state() != RoomState::Joined || !room.is_space() {
            continue;
        }
        edges.extend(space_child_edges(&room).await?);
    }
    Ok(edges)
}

async fn space_child_edges(room: &Room) -> Result<Vec<NativeSpaceChildEdge>, &'static str> {
    let parent_id = room.room_id().to_string();
    let events = room
        .get_state_events_static::<SpaceChildEventContent>()
        .await
        .map_err(|_| "v-rooms.2a-space-child-state-failed")?;
    let mut edges = Vec::new();
    for raw in events {
        if let Some(edge) = parse_valid_edge(&parent_id, &raw) {
            edges.push(edge);
        }
    }
    Ok(edges)
}

fn parse_valid_edge(
    parent_id: &str,
    raw: &RawSyncOrStrippedState<SpaceChildEventContent>,
) -> Option<NativeSpaceChildEdge> {
    match raw {
        RawSyncOrStrippedState::Sync(raw) => {
            let event = raw.deserialize().ok()?;
            let original = event.as_original()?;
            // Match Synara `isValidChild`: content must expose a `via` array.
            let via = original
                .content
                .via
                .iter()
                .map(|server| server.to_string())
                .collect::<Vec<_>>();
            Some(NativeSpaceChildEdge {
                parent_id: parent_id.to_owned(),
                child_id: original.state_key.to_string(),
                order: original
                    .content
                    .order
                    .as_ref()
                    .map(|value| value.to_string()),
                suggested: original.content.suggested,
                via,
                origin_server_ts: original.origin_server_ts.get().into(),
            })
        }
        RawSyncOrStrippedState::Stripped(raw) => {
            let event = raw.deserialize().ok()?;
            // Stripped validity uses ruma's via-present check.
            if !event.content.is_valid() {
                return None;
            }
            let via = event
                .content
                .via
                .as_ref()?
                .iter()
                .map(|server| server.to_string())
                .collect::<Vec<_>>();
            Some(NativeSpaceChildEdge {
                parent_id: parent_id.to_owned(),
                child_id: event.state_key.to_string(),
                order: event.content.order.as_ref().map(|value| value.to_string()),
                suggested: event.content.suggested,
                via,
                origin_server_ts: 0,
            })
        }
    }
}

fn parse_room_id(room_id: &str, diagnostic: &'static str) -> Result<OwnedRoomId, &'static str> {
    OwnedRoomId::try_from(room_id.trim()).map_err(|_| diagnostic)
}

fn joined_room(client: &Client, room_id: &OwnedRoomId) -> Result<Room, &'static str> {
    let room = client.get_room(room_id).ok_or("v-rooms.2c-room-missing")?;
    if room.state() != RoomState::Joined {
        return Err("v-rooms.2c-room-not-joined");
    }
    Ok(room)
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_sdk::ruma::owned_room_id;

    #[test]
    fn reparent_allow_replaces_old_parent_membership() {
        let old = owned_room_id!("!old:example.org");
        let new = owned_room_id!("!new:example.org");
        let keep = owned_room_id!("!keep:example.org");
        let allow = vec![
            AllowRule::room_membership(old.clone()),
            AllowRule::room_membership(keep.clone()),
        ];
        let next = reparent_allow_list(&allow, Some(&old), &new);
        assert_eq!(
            next,
            vec![
                AllowRule::room_membership(keep),
                AllowRule::room_membership(new),
            ]
        );
    }

    #[test]
    fn reparent_allow_dedupes_existing_new_parent() {
        let new = owned_room_id!("!new:example.org");
        let allow = vec![AllowRule::room_membership(new.clone())];
        let next = reparent_allow_list(&allow, None, &new);
        assert_eq!(next, vec![AllowRule::room_membership(new)]);
    }
}

#[cfg(test)]
mod parent_tests {
    use super::*;
    use matrix_sdk::ruma::{
        events::AnySyncStateEvent, room_id, serde::Raw, user_id, Int, RoomVersionId,
    };
    use matrix_sdk::test_utils::mocks::MatrixMockServer;
    use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder};
    use std::collections::BTreeMap;

    fn raw_state(value: serde_json::Value) -> Raw<AnySyncStateEvent> {
        Raw::from_json(serde_json::value::to_raw_value(&value).unwrap())
    }

    #[tokio::test]
    async fn rooms_declaring_a_legitimate_parent_are_grouped_under_it() {
        let server = MatrixMockServer::new().await;
        let client = server.client_builder().build().await;
        let own = user_id!("@example:localhost");
        let bob = user_id!("@bob:example.org");
        let carol = user_id!("@carol:example.org");
        let space = room_id!("!b5-space:example.org");
        let listed = room_id!("!b5-listed:example.org");
        let declared = room_id!("!b5-declared:example.org");
        let removed = room_id!("!b5-removed:example.org");
        let unauthorised = room_id!("!b5-unauthorised:example.org");

        let s = EventFactory::new().room(space);
        server
            .mock_get_members()
            .ok(vec![
                s.member(bob).into_raw(),
                s.member(carol).into_raw(),
                s.member(own).into_raw(),
            ])
            .mount()
            .await;
        server
            .sync_room(
                &client,
                JoinedRoomBuilder::new(space)
                    .add_state_event(s.create(bob, RoomVersionId::V11).with_space_type())
                    .add_state_event(s.member(bob))
                    .add_state_event(s.member(carol))
                    .add_state_event(s.member(own))
                    .add_state_event(
                        s.power_levels(&mut BTreeMap::from([(bob.to_owned(), Int::from(100))]))
                            .sender(bob),
                    )
                    .add_state_event(raw_state(serde_json::json!({
                        "type": "m.space.child",
                        "state_key": listed,
                        "sender": bob,
                        "event_id": "$child-listed",
                        "origin_server_ts": 1,
                        "content": { "via": ["example.org"] }
                    })))
                    // An emptied child event is how Synara and the SDK remove
                    // a child.
                    .add_state_event(raw_state(serde_json::json!({
                        "type": "m.space.child",
                        "state_key": removed,
                        "sender": bob,
                        "event_id": "$child-removed",
                        "origin_server_ts": 2,
                        "content": {}
                    }))),
            )
            .await;
        for (room_id, parent_sender) in [
            (listed, None),
            (declared, Some(bob)),
            (removed, Some(bob)),
            (unauthorised, Some(carol)),
        ] {
            let f = EventFactory::new().room(room_id);
            let mut builder = JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(bob, RoomVersionId::V11))
                .add_state_event(f.member(own));
            if let Some(sender) = parent_sender {
                builder = builder.add_state_event(raw_state(serde_json::json!({
                    "type": "m.space.parent",
                    "state_key": space,
                    "sender": sender,
                    "event_id": format!("$parent-{}", room_id.as_str().len()),
                    "origin_server_ts": 3,
                    "content": { "via": ["example.org"] }
                })));
            }
            server.sync_room(&client, builder).await;
        }

        let snapshot = snapshot_space_parents(&client, 1).await.expect("parents");
        let parents: BTreeMap<String, Vec<String>> = snapshot
            .entries
            .into_iter()
            .map(|entry| (entry.room_id, entry.parent_ids))
            .collect();
        let expected = vec![space.to_string()];
        assert_eq!(parents.get(listed.as_str()), Some(&expected));
        assert_eq!(
            parents.get(declared.as_str()),
            Some(&expected),
            "a parent declared by someone allowed to add children counts"
        );
        assert_eq!(
            parents.get(removed.as_str()),
            None,
            "an explicit child removal wins over a stale parent event"
        );
        assert_eq!(
            parents.get(unauthorised.as_str()),
            None,
            "a parent declared without power in the space is ignored"
        );
    }
}
