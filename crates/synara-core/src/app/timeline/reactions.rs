//! Recover remote `m.reaction` annotation event ids for native projection.
//!
//! 0.19 `ReactionInfo` exposes `send_state` only for our local echoes. Remote
//! reactions (`send_state: None`) omit the annotation id. The public recovery
//! order is: `EventSendState::Sent`, then in-memory room event cache, then
//! (viewer open / redact only) stored cache relations and `Room::relations`.
//! Timeline deltas never call `/relations`.

use std::collections::{HashMap, HashSet};

use matrix_sdk::{
    deserialized_responses::TimelineEvent,
    room::{IncludeRelations, RelationsOptions},
    ruma::{
        events::{
            relation::RelationType, AnySyncMessageLikeEvent, AnySyncTimelineEvent, TimelineEventType,
        },
        uint, EventId, OwnedEventId,
    },
    Room,
};
use matrix_sdk_ui::timeline::EventSendState;

use super::native::{NativeTimelineItem, NativeTimelineReaction, NativeTimelineReactionSender};
use super::view::{TimelineReaction, TimelineReactionSender, TimelineViewDeltaOp, TimelineViewRow};

/// `(target_event_id, sender, key) → annotation event id`.
pub type AnnotationIdMap = HashMap<(String, String, String), String>;

/// One recovered `m.reaction` annotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredAnnotation {
    pub reaction_event_id: String,
    pub sender: String,
    pub target_event_id: String,
    pub key: String,
}

/// Map 0.19 `ReactionInfo.send_state` onto a public annotation id.
///
/// Remote reactions (`None`) and unsent/failed local echoes stay `None`.
pub fn reaction_event_id_from_send_state(send_state: &Option<EventSendState>) -> Option<String> {
    match send_state {
        Some(EventSendState::Sent { event_id }) => Some(event_id.to_string()),
        _ => None,
    }
}

/// Parse a timeline/cache event as an `m.reaction` annotation.
pub fn annotation_from_timeline_event(event: &TimelineEvent) -> Option<RecoveredAnnotation> {
    let reaction_event_id = event.event_id()?.to_string();
    let AnySyncTimelineEvent::MessageLike(AnySyncMessageLikeEvent::Reaction(reaction)) =
        event.raw().deserialize().ok()?
    else {
        return None;
    };
    let original = reaction.as_original()?;
    Some(RecoveredAnnotation {
        reaction_event_id,
        sender: original.sender.to_string(),
        target_event_id: original.content.relates_to.event_id.to_string(),
        key: original.content.relates_to.key.clone(),
    })
}

/// Fill a missing sender id from a recovered map. Existing ids are kept.
pub fn fill_sender_event_id(
    target_event_id: &str,
    key: &str,
    user_id: &str,
    reaction_event_id: &mut Option<String>,
    recovered: &AnnotationIdMap,
) {
    if reaction_event_id.is_some() {
        return;
    }
    if let Some(id) = recovered.get(&(
        target_event_id.to_owned(),
        user_id.to_owned(),
        key.to_owned(),
    )) {
        *reaction_event_id = Some(id.clone());
    }
}

pub fn fill_native_items_reaction_ids(items: &mut [NativeTimelineItem], recovered: &AnnotationIdMap) {
    for item in items {
        fill_native_reactions(&item.event_id, &mut item.reactions, recovered);
    }
}

pub fn fill_native_reactions(
    target_event_id: &str,
    reactions: &mut [NativeTimelineReaction],
    recovered: &AnnotationIdMap,
) {
    for reaction in reactions {
        for sender in &mut reaction.senders {
            fill_sender_event_id(
                target_event_id,
                &reaction.key,
                &sender.user_id,
                &mut sender.reaction_event_id,
                recovered,
            );
        }
    }
}

pub fn fill_view_rows_reaction_ids(rows: &mut [TimelineViewRow], recovered: &AnnotationIdMap) {
    for row in rows {
        fill_view_row_reaction_ids(row, recovered);
    }
}

pub fn fill_view_delta_ops_reaction_ids(
    ops: &mut [TimelineViewDeltaOp],
    recovered: &AnnotationIdMap,
) {
    for op in ops {
        match op {
            TimelineViewDeltaOp::Append { rows } | TimelineViewDeltaOp::Reset { rows } => {
                fill_view_rows_reaction_ids(rows, recovered);
            }
            TimelineViewDeltaOp::PushFront { row }
            | TimelineViewDeltaOp::PushBack { row }
            | TimelineViewDeltaOp::Insert { row, .. }
            | TimelineViewDeltaOp::Set { row, .. } => {
                fill_view_row_reaction_ids(row, recovered);
            }
            _ => {}
        }
    }
}

fn fill_view_row_reaction_ids(row: &mut TimelineViewRow, recovered: &AnnotationIdMap) {
    match row {
        TimelineViewRow::Message(row) => {
            let Some(target) = row.event.event_id.clone() else {
                return;
            };
            fill_view_reactions(&target, &mut row.reactions, recovered);
        }
        TimelineViewRow::Poll(row) => {
            let Some(target) = row.event.event_id.clone() else {
                return;
            };
            fill_view_reactions(&target, &mut row.reactions, recovered);
        }
        TimelineViewRow::Sticker {
            event, reactions, ..
        } => {
            let Some(target) = event.event_id.clone() else {
                return;
            };
            fill_view_reactions(&target, reactions, recovered);
        }
        _ => {}
    }
}

fn fill_view_reactions(
    target_event_id: &str,
    reactions: &mut [TimelineReaction],
    recovered: &AnnotationIdMap,
) {
    for reaction in reactions {
        for sender in &mut reaction.senders {
            fill_sender_event_id(
                target_event_id,
                &reaction.key,
                &sender.user_id,
                &mut sender.reaction_event_id,
                recovered,
            );
        }
    }
}

pub fn native_items_reaction_targets(items: &[NativeTimelineItem]) -> HashSet<String> {
    items
        .iter()
        .filter(|item| {
            item.reactions.iter().any(|reaction| {
                reaction
                    .senders
                    .iter()
                    .any(|sender| sender.reaction_event_id.is_none())
            })
        })
        .map(|item| item.event_id.clone())
        .collect()
}

pub fn view_rows_reaction_targets(rows: &[TimelineViewRow]) -> HashSet<String> {
    let mut targets = HashSet::new();
    collect_view_row_targets(rows, &mut targets);
    targets
}

pub fn view_delta_ops_reaction_targets(ops: &[TimelineViewDeltaOp]) -> HashSet<String> {
    let mut targets = HashSet::new();
    for op in ops {
        match op {
            TimelineViewDeltaOp::Append { rows } | TimelineViewDeltaOp::Reset { rows } => {
                collect_view_row_targets(rows, &mut targets);
            }
            TimelineViewDeltaOp::PushFront { row }
            | TimelineViewDeltaOp::PushBack { row }
            | TimelineViewDeltaOp::Insert { row, .. }
            | TimelineViewDeltaOp::Set { row, .. } => {
                collect_view_row_targets(std::slice::from_ref(row), &mut targets);
            }
            _ => {}
        }
    }
    targets
}

fn collect_view_row_targets(rows: &[TimelineViewRow], targets: &mut HashSet<String>) {
    for row in rows {
        let (event_id, reactions) = match row {
            TimelineViewRow::Message(row) => (row.event.event_id.as_deref(), row.reactions.as_slice()),
            TimelineViewRow::Poll(row) => (row.event.event_id.as_deref(), row.reactions.as_slice()),
            TimelineViewRow::Sticker {
                event, reactions, ..
            } => (event.event_id.as_deref(), reactions.as_slice()),
            _ => continue,
        };
        let Some(event_id) = event_id else {
            continue;
        };
        if reactions.iter().any(|reaction| {
            reaction
                .senders
                .iter()
                .any(|sender| sender.reaction_event_id.is_none())
        }) {
            targets.insert(event_id.to_owned());
        }
    }
}

/// In-memory cache scan. Does not page storage or call `/relations`.
pub async fn recover_annotation_ids_in_memory(
    room: &Room,
    targets: &HashSet<String>,
) -> AnnotationIdMap {
    let mut recovered = AnnotationIdMap::new();
    if targets.is_empty() {
        return recovered;
    }
    let Ok((cache, _handles)) = room.event_cache().await else {
        return recovered;
    };
    let mut visited = 0usize;
    let _ = cache
        .rfind_map_event_in_memory_by(|event| {
            visited += 1;
            if let Some(annotation) = annotation_from_timeline_event(event) {
                if targets.contains(&annotation.target_event_id) {
                    recovered.insert(
                        (
                            annotation.target_event_id,
                            annotation.sender,
                            annotation.key,
                        ),
                        annotation.reaction_event_id,
                    );
                }
            }
            (visited >= 2_000).then_some(())
        })
        .await;
    recovered
}

/// Stored + loaded related events. Still not a `/relations` network fetch.
pub async fn recover_annotation_ids_from_room_cache(
    room: &Room,
    target: &EventId,
) -> AnnotationIdMap {
    let mut recovered = AnnotationIdMap::new();
    let Ok((cache, _handles)) = room.event_cache().await else {
        return recovered;
    };
    let Ok(related) = cache
        .find_event_relations(target, Some(vec![RelationType::Annotation]))
        .await
    else {
        return recovered;
    };
    extend_recovered_from_events(&mut recovered, related);
    recovered
}

/// Lazy `/relations` used only on viewer open / redact.
pub async fn recover_annotation_ids_from_relations(
    room: &Room,
    target: OwnedEventId,
) -> AnnotationIdMap {
    let mut recovered = AnnotationIdMap::new();
    let mut opts = RelationsOptions {
        include_relations: IncludeRelations::RelationsOfTypeAndEventType(
            RelationType::Annotation,
            TimelineEventType::Reaction,
        ),
        limit: Some(uint!(100)),
        recurse: false,
        ..Default::default()
    };
    for _ in 0..5 {
        let Ok(page) = room.relations(target.clone(), opts.clone()).await else {
            break;
        };
        extend_recovered_from_events(&mut recovered, page.chunk);
        match page.prev_batch_token {
            Some(from) if recovered.len() < 500 => opts.from = Some(from),
            _ => break,
        }
    }
    recovered
}

fn extend_recovered_from_events(
    recovered: &mut AnnotationIdMap,
    events: impl IntoIterator<Item = TimelineEvent>,
) {
    for event in events {
        if let Some(annotation) = annotation_from_timeline_event(&event) {
            recovered.insert(
                (
                    annotation.target_event_id,
                    annotation.sender,
                    annotation.key,
                ),
                annotation.reaction_event_id,
            );
        }
    }
}

/// Recover missing annotation ids. `network` enables stored-cache relations
/// and `/relations` after the in-memory pass.
pub async fn recover_missing_annotation_ids(
    room: &Room,
    targets: &HashSet<String>,
    network: bool,
) -> AnnotationIdMap {
    let mut recovered = recover_annotation_ids_in_memory(room, targets).await;
    if !network {
        return recovered;
    }
    for target in targets {
        let Ok(event_id) = OwnedEventId::try_from(target.as_str()) else {
            continue;
        };
        let stored = recover_annotation_ids_from_room_cache(room, &event_id).await;
        recovered.extend(stored);
        let networked = recover_annotation_ids_from_relations(room, event_id).await;
        recovered.extend(networked);
    }
    recovered
}

pub async fn enrich_native_items(
    room: &Room,
    items: &mut [NativeTimelineItem],
    network: bool,
) {
    let targets = native_items_reaction_targets(items);
    if targets.is_empty() {
        return;
    }
    let recovered = recover_missing_annotation_ids(room, &targets, network).await;
    fill_native_items_reaction_ids(items, &recovered);
}

pub async fn enrich_native_reactions(
    room: &Room,
    target_event_id: &str,
    reactions: &mut [NativeTimelineReaction],
    network: bool,
) {
    if !reactions.iter().any(|reaction| {
        reaction
            .senders
            .iter()
            .any(|sender| sender.reaction_event_id.is_none())
    }) {
        return;
    }
    let mut targets = HashSet::new();
    targets.insert(target_event_id.to_owned());
    let recovered = recover_missing_annotation_ids(room, &targets, network).await;
    fill_native_reactions(target_event_id, reactions, &recovered);
}

pub async fn enrich_view_rows(room: &Room, rows: &mut [TimelineViewRow]) {
    let targets = view_rows_reaction_targets(rows);
    if targets.is_empty() {
        return;
    }
    let recovered = recover_missing_annotation_ids(room, &targets, false).await;
    fill_view_rows_reaction_ids(rows, &recovered);
}

pub async fn enrich_view_delta_ops(room: &Room, ops: &mut [TimelineViewDeltaOp]) {
    let targets = view_delta_ops_reaction_targets(ops);
    if targets.is_empty() {
        return;
    }
    let recovered = recover_missing_annotation_ids(room, &targets, false).await;
    fill_view_delta_ops_reaction_ids(ops, &recovered);
}

pub fn project_view_reaction_senders<'a, U>(
    by_sender: impl IntoIterator<Item = (&'a U, &'a matrix_sdk_ui::timeline::ReactionInfo)>,
) -> Vec<TimelineReactionSender>
where
    U: std::fmt::Display + 'a,
{
    by_sender
        .into_iter()
        .map(|(user_id, info)| TimelineReactionSender {
            user_id: user_id.to_string(),
            reaction_event_id: reaction_event_id_from_send_state(&info.send_state),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use matrix_sdk::ruma::{
        serde::Raw, MilliSecondsSinceUnixEpoch, OwnedEventId, UInt,
    };
    use matrix_sdk_ui::timeline::{EventSendState, ReactionInfo};
    use serde_json::json;

    use super::*;
    use crate::app::timeline::native::NativeTimelineReactionSender;

    fn sent_id(id: &str) -> EventSendState {
        EventSendState::Sent {
            event_id: OwnedEventId::try_from(id).unwrap(),
        }
    }

    #[test]
    fn sent_send_state_maps_the_annotation_id() {
        assert_eq!(
            reaction_event_id_from_send_state(&Some(sent_id("$reaction:example.org"))),
            Some("$reaction:example.org".into())
        );
    }

    #[test]
    fn remote_and_unsent_send_states_do_not_invent_ids() {
        assert_eq!(reaction_event_id_from_send_state(&None), None);
        assert_eq!(
            reaction_event_id_from_send_state(&Some(EventSendState::NotSentYet { progress: None })),
            None
        );
    }

    #[test]
    fn recovered_ids_fill_only_missing_remote_senders() {
        let mut senders = vec![
            NativeTimelineReactionSender {
                user_id: "@alice:example.org".into(),
                reaction_event_id: Some("$local-sent:example.org".into()),
            },
            NativeTimelineReactionSender {
                user_id: "@bob:example.org".into(),
                reaction_event_id: None,
            },
        ];
        let mut recovered = AnnotationIdMap::new();
        recovered.insert(
            (
                "$target:example.org".into(),
                "@bob:example.org".into(),
                "👍".into(),
            ),
            "$bob-reaction:example.org".into(),
        );
        recovered.insert(
            (
                "$target:example.org".into(),
                "@alice:example.org".into(),
                "👍".into(),
            ),
            "$should-not-replace:example.org".into(),
        );
        for sender in &mut senders {
            fill_sender_event_id(
                "$target:example.org",
                "👍",
                &sender.user_id,
                &mut sender.reaction_event_id,
                &recovered,
            );
        }
        assert_eq!(
            senders[0].reaction_event_id.as_deref(),
            Some("$local-sent:example.org")
        );
        assert_eq!(
            senders[1].reaction_event_id.as_deref(),
            Some("$bob-reaction:example.org")
        );
    }

    #[test]
    fn annotation_parse_reads_sender_key_and_target() {
        let raw = Raw::from_json_string(
            json!({
                "event_id": "$reaction:example.org",
                "sender": "@alice:example.org",
                "origin_server_ts": 1,
                "type": "m.reaction",
                "content": {
                    "m.relates_to": {
                        "rel_type": "m.annotation",
                        "event_id": "$target:example.org",
                        "key": "👍"
                    }
                }
            })
            .to_string(),
        )
        .unwrap();
        let event = TimelineEvent::from_plaintext(raw);
        let parsed = annotation_from_timeline_event(&event).expect("reaction");
        assert_eq!(parsed.reaction_event_id, "$reaction:example.org");
        assert_eq!(parsed.sender, "@alice:example.org");
        assert_eq!(parsed.target_event_id, "$target:example.org");
        assert_eq!(parsed.key, "👍");
    }

    #[test]
    fn reaction_info_send_state_matches_projection_contract() {
        let sent = ReactionInfo {
            timestamp: MilliSecondsSinceUnixEpoch(UInt::from(1u32)),
            send_state: Some(sent_id("$r:example.org")),
        };
        let remote = ReactionInfo {
            timestamp: MilliSecondsSinceUnixEpoch(UInt::from(1u32)),
            send_state: None,
        };
        assert_eq!(
            reaction_event_id_from_send_state(&sent.send_state).as_deref(),
            Some("$r:example.org")
        );
        assert_eq!(reaction_event_id_from_send_state(&remote.send_state), None);
    }

    #[test]
    fn view_and_live_projection_keep_sent_ids_and_fill_remote_none() {
        let alice = matrix_sdk::ruma::user_id!("@alice:example.org").to_owned();
        let bob = matrix_sdk::ruma::user_id!("@bob:example.org").to_owned();
        let sent = ReactionInfo {
            timestamp: MilliSecondsSinceUnixEpoch(UInt::from(1u32)),
            send_state: Some(sent_id("$alice-reaction:example.org")),
        };
        let remote = ReactionInfo {
            timestamp: MilliSecondsSinceUnixEpoch(UInt::from(1u32)),
            send_state: None,
        };
        let mut senders = project_view_reaction_senders([(&alice, &sent), (&bob, &remote)]);
        assert_eq!(
            senders[0].reaction_event_id.as_deref(),
            Some("$alice-reaction:example.org")
        );
        assert_eq!(senders[1].reaction_event_id, None);

        let mut recovered = AnnotationIdMap::new();
        recovered.insert(
            (
                "$target:example.org".into(),
                "@bob:example.org".into(),
                "👍".into(),
            ),
            "$bob-reaction:example.org".into(),
        );
        for sender in &mut senders {
            fill_sender_event_id(
                "$target:example.org",
                "👍",
                &sender.user_id,
                &mut sender.reaction_event_id,
                &recovered,
            );
        }
        assert_eq!(
            senders[0].reaction_event_id.as_deref(),
            Some("$alice-reaction:example.org")
        );
        assert_eq!(
            senders[1].reaction_event_id.as_deref(),
            Some("$bob-reaction:example.org")
        );
    }
}
