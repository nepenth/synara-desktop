//! A7 deterministic sequencing proof at the production SDK/Core boundary.
//!
//! These tests drive the pinned `matrix-sdk-ui` timeline with a mocked
//! homeserver, then project the SDK's real `VectorDiff` batches through
//! `project_timeline_diffs`. They prove deterministic adapter behavior, not
//! live homeserver or two-client interoperability.

use std::{collections::BTreeMap, io::Cursor, sync::Arc, time::Duration};

use eyeball_im::VectorDiff;
use futures_util::StreamExt;
use matrix_sdk::test_utils::mocks::{MatrixMockServer, RoomMessagesResponseTemplate};
use matrix_sdk_crypto::decrypt_room_key_export;
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, ALICE, BOB, CAROL};
use matrix_sdk_ui::timeline::{RoomExt, TimelineItem as SdkTimelineItem};
use ruma::{
    event_id,
    events::{
        room::encrypted::{
            EncryptedEventScheme, MegolmV1AesSha2ContentInit, RoomEncryptedEventContent,
        },
        StateEventType,
    },
    room_id, EventId, RoomVersionId,
};
use synara_core::app::timeline::{
    project_timeline_diffs, project_timeline_diffs_with_media, project_timeline_item,
    NativeTimelineOpenPosition, NativeTimelineOpenRequest, NativeTimelineOwner,
    TimelineMediaRegistry, TimelineMessageRow, TimelineRoomActionAuthority, TimelineViewDeltaBatch,
    TimelineViewDeltaOp, TimelineViewRow,
};
use tokio::time::timeout;

const WAIT: Duration = Duration::from_secs(5);

fn row_event_id(row: &TimelineViewRow) -> Option<&str> {
    match row {
        TimelineViewRow::Message(row) => row.event.event_id.as_deref(),
        TimelineViewRow::Sticker { event, .. }
        | TimelineViewRow::Membership(synara_core::app::timeline::TimelineMembershipRow {
            event,
            ..
        })
        | TimelineViewRow::State(synara_core::app::timeline::TimelineStateRow { event, .. })
        | TimelineViewRow::Call(synara_core::app::timeline::TimelineCallRow { event, .. })
        | TimelineViewRow::Redacted(synara_core::app::timeline::TimelineRedactedRow {
            event,
            ..
        })
        | TimelineViewRow::EncryptedUnavailable(
            synara_core::app::timeline::TimelineEncryptedUnavailableRow { event, .. },
        ) => event.event_id.as_deref(),
        TimelineViewRow::Poll(row) => row.event.event_id.as_deref(),
        TimelineViewRow::Other(row) => row.event_id.as_deref(),
        TimelineViewRow::DateSeparator { .. }
        | TimelineViewRow::ReadMarker { .. }
        | TimelineViewRow::UnreadMarker { .. }
        | TimelineViewRow::TimelineStart { .. }
        | TimelineViewRow::Pagination { .. } => None,
    }
}

fn row_item_id(row: &TimelineViewRow) -> &str {
    match row {
        TimelineViewRow::Message(row) => &row.event.item_id,
        TimelineViewRow::Sticker { event, .. }
        | TimelineViewRow::Membership(synara_core::app::timeline::TimelineMembershipRow {
            event,
            ..
        })
        | TimelineViewRow::State(synara_core::app::timeline::TimelineStateRow { event, .. })
        | TimelineViewRow::Call(synara_core::app::timeline::TimelineCallRow { event, .. })
        | TimelineViewRow::Redacted(synara_core::app::timeline::TimelineRedactedRow {
            event,
            ..
        })
        | TimelineViewRow::EncryptedUnavailable(
            synara_core::app::timeline::TimelineEncryptedUnavailableRow { event, .. },
        ) => &event.item_id,
        TimelineViewRow::Poll(row) => &row.event.item_id,
        TimelineViewRow::Other(row) => &row.item_id,
        TimelineViewRow::DateSeparator { item_id, .. }
        | TimelineViewRow::ReadMarker { item_id }
        | TimelineViewRow::UnreadMarker { item_id }
        | TimelineViewRow::TimelineStart { item_id }
        | TimelineViewRow::Pagination { item_id, .. } => item_id,
    }
}

fn op_row(op: &TimelineViewDeltaOp) -> Option<&TimelineViewRow> {
    match op {
        TimelineViewDeltaOp::PushFront { row }
        | TimelineViewDeltaOp::PushBack { row }
        | TimelineViewDeltaOp::Insert { row, .. }
        | TimelineViewDeltaOp::Set { row, .. } => Some(row),
        TimelineViewDeltaOp::Append { .. }
        | TimelineViewDeltaOp::Reset { .. }
        | TimelineViewDeltaOp::Clear
        | TimelineViewDeltaOp::PopFront
        | TimelineViewDeltaOp::PopBack
        | TimelineViewDeltaOp::Remove { .. }
        | TimelineViewDeltaOp::Truncate { .. } => None,
    }
}

fn projected_event<'a>(ops: &'a [TimelineViewDeltaOp], event_id: &EventId) -> &'a TimelineViewRow {
    ops.iter()
        .filter_map(op_row)
        .find(|row| row_event_id(row) == Some(event_id.as_str()))
        .unwrap_or_else(|| panic!("missing projected event {event_id}"))
}

async fn next_batch<S>(stream: &mut S) -> Vec<VectorDiff<std::sync::Arc<SdkTimelineItem>>>
where
    S: futures_util::Stream<Item = Vec<VectorDiff<std::sync::Arc<SdkTimelineItem>>>> + Unpin,
{
    timeout(WAIT, stream.next())
        .await
        .expect("timeline delta timed out")
        .expect("timeline delta stream ended")
}

fn event_capabilities(
    rows: &[TimelineViewRow],
    event_id: &EventId,
) -> synara_core::app::timeline::TimelineRowCapabilities {
    let row = rows
        .iter()
        .find(|row| row_event_id(row) == Some(event_id.as_str()))
        .unwrap_or_else(|| panic!("missing projected event {event_id}"));
    match row {
        TimelineViewRow::Message(row) => row.event.capabilities,
        TimelineViewRow::Sticker { event, .. }
        | TimelineViewRow::Membership(synara_core::app::timeline::TimelineMembershipRow {
            event,
            ..
        })
        | TimelineViewRow::State(synara_core::app::timeline::TimelineStateRow { event, .. })
        | TimelineViewRow::Call(synara_core::app::timeline::TimelineCallRow { event, .. })
        | TimelineViewRow::Redacted(synara_core::app::timeline::TimelineRedactedRow {
            event,
            ..
        })
        | TimelineViewRow::EncryptedUnavailable(
            synara_core::app::timeline::TimelineEncryptedUnavailableRow { event, .. },
        ) => event.capabilities,
        TimelineViewRow::Poll(row) => row.event.capabilities,
        TimelineViewRow::Other(row) => {
            row.event
                .as_ref()
                .expect("event row missing capability base")
                .capabilities
        }
        _ => panic!("event projected as a virtual row"),
    }
}

async fn next_authority_reset(
    receiver: &mut tokio::sync::mpsc::UnboundedReceiver<TimelineViewDeltaBatch>,
    event_id: &EventId,
) -> Vec<TimelineViewRow> {
    timeout(WAIT, async {
        loop {
            let batch = receiver.recv().await.expect("view update stream ended");
            if let Some(rows) = batch.ops.into_iter().find_map(|op| match op {
                TimelineViewDeltaOp::Reset { rows }
                    if rows
                        .iter()
                        .any(|row| row_event_id(row) == Some(event_id.as_str())) =>
                {
                    Some(rows)
                }
                _ => None,
            }) {
                break rows;
            }
        }
    })
    .await
    .expect("authority re-projection timed out")
}

#[tokio::test]
async fn redaction_replaces_the_existing_projected_row_without_duplicate_identity() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!a7-redaction:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;

    let timeline = room.timeline().await.unwrap();
    let (_, mut stream) = timeline.subscribe().await;
    let event_id = event_id!("$a7-redaction-target");
    let f = EventFactory::new().room(room_id);
    let mut media_registry = TimelineMediaRegistry::new(7, room_id.as_str());

    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_timeline_event(f.text_msg("remove me").sender(*ALICE).event_id(event_id)),
        )
        .await;
    let inserted_diffs = next_batch(&mut stream).await;
    let inserted = project_timeline_diffs(&inserted_diffs, client.user_id());
    let live_inserted = project_timeline_diffs_with_media(
        &inserted_diffs,
        client.user_id(),
        TimelineRoomActionAuthority::default(),
        &mut media_registry,
    );
    let original = projected_event(&inserted, event_id);
    let live_original = projected_event(&live_inserted, event_id);
    assert!(matches!(original, TimelineViewRow::Message(_)));
    let original_item_id = row_item_id(original).to_owned();
    assert_eq!(row_item_id(live_original), original_item_id);
    let original_index = timeline
        .subscribe()
        .await
        .0
        .iter()
        .position(|item| item.as_event().and_then(|event| event.event_id()) == Some(event_id))
        .expect("inserted event missing from SDK timeline");

    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_timeline_event(f.redaction(event_id).sender(*ALICE)),
        )
        .await;
    let replacement_diffs = next_batch(&mut stream).await;
    let replacement = project_timeline_diffs(&replacement_diffs, client.user_id());
    let live_replacement = project_timeline_diffs_with_media(
        &replacement_diffs,
        client.user_id(),
        TimelineRoomActionAuthority::default(),
        &mut media_registry,
    );
    let (replacement_index, replacement_row) = replacement
        .iter()
        .find_map(|op| match op {
            TimelineViewDeltaOp::Set { index, row }
                if row_event_id(row) == Some(event_id.as_str()) =>
            {
                Some((*index, row))
            }
            _ => None,
        })
        .expect("redaction must project as an in-place Set");

    assert_eq!(replacement_index, original_index);
    assert!(matches!(replacement_row, TimelineViewRow::Redacted(_)));
    assert_eq!(row_item_id(replacement_row), original_item_id);
    let (live_replacement_index, live_replacement_row) = live_replacement
        .iter()
        .find_map(|op| match op {
            TimelineViewDeltaOp::Set { index, row }
                if row_event_id(row) == Some(event_id.as_str()) =>
            {
                Some((*index, row))
            }
            _ => None,
        })
        .expect("live redaction projector must preserve the in-place Set");
    assert_eq!(live_replacement_index, original_index);
    assert!(matches!(live_replacement_row, TimelineViewRow::Redacted(_)));
    assert_eq!(row_item_id(live_replacement_row), original_item_id);
    assert!(replacement.iter().all(|op| match op_row(op) {
        Some(row) if row_event_id(row) == Some(event_id.as_str()) => {
            matches!(op, TimelineViewDeltaOp::Set { .. })
                && matches!(row, TimelineViewRow::Redacted(_))
                && row_item_id(row) == original_item_id
        }
        _ => true,
    }));
}

#[tokio::test]
async fn late_decryption_replaces_utd_with_plaintext_at_the_same_projected_identity() {
    const SESSION_ID: &str = "gM8i47Xhu0q52xLfgUXzanCMpLinoyVyH7R58cBuVBU";
    const SESSION_KEY: &[u8] = b"\
        -----BEGIN MEGOLM SESSION DATA-----\n\
        ASKcWoiAVUM97482UAi83Avce62hSLce7i5JhsqoF6xeAAAACqt2Cg3nyJPRWTTMXxXH7TXnkfdlmBXbQtq5\
        bpHo3LRijcq2Gc6TXilESCmJN14pIsfKRJrWjZ0squ/XsoTFytuVLWwkNaW3QF6obeg2IoVtJXLMPdw3b2vO\
        vgwGY3OMP0XafH13j1vcb6YLzvgLkZQLnYvd47hv3yK/9GmKS9tokuaQ7dCVYckYcIOS09EDTs70YdxUd5WG\
        rQynATCLFP1p/NAGv70r9MK7Cy/mNpjD0r4qC7UEDIoi1kOWzHgnLo19wtvwsb8Fg8ATxcs3Wmtj8hIUYpDx\
        ia4sM10zbytUuaPUAfCDf42IyxdmOnGe1CueXhgI71y+RW0s0argNqUt7jB70JT0o9CyX6UBGRaqLk2MPY9T\
        hUu5J8X3UgIa6rcbWigzohzWm9rdbEHFrSWqjpfQYMaAKQQgETrjSy4XTrp2RhC2oNqG/hylI4ab+F4X6fpH\
        DYP1NqNMP5g36xNu7LhDnrUB5qsPjYOmWORxGLfudpF3oLYCSlr3DgHqEIB6HjQblLZ3KQuPBse3zxyROTnS\
        AhdPH4a/z1wioFtKNVph3hecsiKEdqnz4Y2coSIdhz58mJ9JWNQoFAENE5CSsoEZAGvafYZVpW4C75YY2zq1\
        wIeiFi1dT43/jLAUGkslsi1VvnyfUu8qO404RxYO3XHoGLMFoFLOO+lZ+VGci2Vz10AhxJhEBHxRKxw4k2uB\
        HztoSJUr/2Y\n\
        -----END MEGOLM SESSION DATA-----";

    let server = MatrixMockServer::new().await;
    let client = server.client_builder().logged_in_with_oauth().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!DovneieKSTkdHKpIXy:morpheus.localhost");
    let f = EventFactory::new().room(room_id);
    let room = server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.member(client.user_id().unwrap()).display_name("Alice")),
        )
        .await;
    let timeline = room.timeline().await.unwrap();
    let (_, mut stream) = timeline.subscribe().await;
    let event_id = event_id!("$a7-late-decryption");
    let mut media_registry = TimelineMediaRegistry::new(7, room_id.as_str());
    let encrypted = f
        .event(RoomEncryptedEventContent::new(
            EncryptedEventScheme::MegolmV1AesSha2(
                MegolmV1AesSha2ContentInit {
                    ciphertext: "AwgAEtABPRMavuZMDJrPo6pGQP4qVmpcuapuXtzKXJyi3YpEsjSWdzuRKIgJzD4PcSqJM1A8kzxecTQNJsC5q22+KSFEPxPnI4ltpm7GFowSoPSW9+bFdnlfUzEP1jPqYevHAsMJp2fRKkzQQbPordrUk1gNqEpGl4BYFeRqKl9GPdKFwy45huvQCLNNueqlCFZVoYMuhxrfyMiJJAVNTofkr2um2mKjDTlajHtr39pTG8k0eOjSXkLOSdZvNOMzhGhSaFNeERSA2G2YbeknOvU7MvjiO0AKuxaAe1CaVhAI14FCgzrJ8g0y5nly+n7xQzL2G2Dn8EoXM5Iqj8W99iokQoVsSrUEnaQ1WnSIfewvDDt4LCaD/w7PGETMCQ".to_owned(),
                    sender_key: "DeHIg4gwhClxzFYcmNntPNF9YtsdZbmMy8+3kzCMXHA".to_owned(),
                    device_id: "NLAZCWIOCO".into(),
                    session_id: SESSION_ID.into(),
                }
                .into(),
            ),
            None,
        ))
        .sender(*BOB)
        .event_id(event_id);

    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id).add_timeline_event(encrypted),
        )
        .await;
    let utd_diffs = next_batch(&mut stream).await;
    let utd_ops = project_timeline_diffs(&utd_diffs, client.user_id());
    let live_utd_ops = project_timeline_diffs_with_media(
        &utd_diffs,
        client.user_id(),
        TimelineRoomActionAuthority::default(),
        &mut media_registry,
    );
    let utd_row = projected_event(&utd_ops, event_id);
    let live_utd_row = projected_event(&live_utd_ops, event_id);
    assert!(matches!(utd_row, TimelineViewRow::EncryptedUnavailable(_)));
    let original_item_id = row_item_id(utd_row).to_owned();
    assert_eq!(row_item_id(live_utd_row), original_item_id);

    let exported_keys = decrypt_room_key_export(Cursor::new(SESSION_KEY), "1234").unwrap();
    client
        .olm_machine_for_testing()
        .await
        .as_ref()
        .unwrap()
        .store()
        .import_exported_room_keys(exported_keys, |_, _| {})
        .await
        .unwrap();

    let decrypted_diffs = next_batch(&mut stream).await;
    let decrypted_ops = project_timeline_diffs(&decrypted_diffs, client.user_id());
    let live_decrypted_ops = project_timeline_diffs_with_media(
        &decrypted_diffs,
        client.user_id(),
        TimelineRoomActionAuthority::default(),
        &mut media_registry,
    );
    let (replacement_index, decrypted_row) = decrypted_ops
        .iter()
        .find_map(|op| match op {
            TimelineViewDeltaOp::Set { index, row }
                if row_event_id(row) == Some(event_id.as_str()) =>
            {
                Some((*index, row))
            }
            _ => None,
        })
        .expect("late decryption must project as an in-place Set");
    let TimelineViewRow::Message(message) = decrypted_row else {
        panic!("late-decrypted event was not projected as a message")
    };
    assert_eq!(replacement_index, 1);
    assert_eq!(message.body, "It's a secret to everybody");
    assert_eq!(row_item_id(decrypted_row), original_item_id);
    let (live_replacement_index, live_decrypted_row) = live_decrypted_ops
        .iter()
        .find_map(|op| match op {
            TimelineViewDeltaOp::Set { index, row }
                if row_event_id(row) == Some(event_id.as_str()) =>
            {
                Some((*index, row))
            }
            _ => None,
        })
        .expect("live late-decrypt projector must preserve the in-place Set");
    let TimelineViewRow::Message(live_message) = live_decrypted_row else {
        panic!("live late-decrypted event was not projected as a message")
    };
    assert_eq!(live_replacement_index, 1);
    assert_eq!(live_message.body, "It's a secret to everybody");
    assert_eq!(row_item_id(live_decrypted_row), original_item_id);
}

#[tokio::test]
async fn pagination_overlap_keeps_one_projected_identity_per_event() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!a7-pagination:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    let timeline = room.timeline().await.unwrap();
    let (_, mut stream) = timeline.subscribe().await;
    let overlap_id = event_id!("$a7-overlap");
    let older_id = event_id!("$a7-older");
    let f = EventFactory::new().room(room_id);

    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .set_timeline_limited()
                .set_timeline_prev_batch("a7-prev")
                .add_timeline_event(
                    f.text_msg("already live")
                        .sender(*ALICE)
                        .event_id(overlap_id),
                ),
        )
        .await;
    let live_ops = project_timeline_diffs(&next_batch(&mut stream).await, client.user_id());
    let overlap_item_id = row_item_id(projected_event(&live_ops, overlap_id)).to_owned();
    assert_eq!(
        live_ops
            .iter()
            .filter_map(op_row)
            .filter(|row| row_event_id(row) == Some(overlap_id.as_str()))
            .count(),
        1
    );

    let overlap = f
        .text_msg("already live")
        .sender(*ALICE)
        .event_id(overlap_id)
        .into_raw_timeline();
    let older = f
        .text_msg("older history")
        .sender(*BOB)
        .event_id(older_id)
        .into_raw_timeline();
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default().events(vec![overlap, older]))
        .mock_once()
        .mount()
        .await;

    assert!(timeline.paginate_backwards(10).await.unwrap());
    let pagination_ops = project_timeline_diffs(&next_batch(&mut stream).await, client.user_id());
    assert_eq!(
        pagination_ops
            .iter()
            .filter_map(op_row)
            .filter(|row| row_event_id(row) == Some(older_id.as_str()))
            .count(),
        1,
        "pagination must introduce the older event exactly once"
    );
    let projected_overlap_rows: Vec<_> = pagination_ops
        .iter()
        .filter_map(op_row)
        .filter(|row| row_event_id(row) == Some(overlap_id.as_str()))
        .collect();
    assert_eq!(
        projected_overlap_rows.len(),
        1,
        "the SDK may move an overlapping event, but must not project two copies"
    );
    assert_eq!(
        row_item_id(projected_overlap_rows[0]),
        overlap_item_id,
        "deduplication must preserve the SDK/Core row identity while moving it"
    );

    let (items, _) = timeline.subscribe().await;
    let rows: Vec<_> = items
        .iter()
        .map(|item| project_timeline_item(item, client.user_id()))
        .collect();
    assert_eq!(
        rows.iter()
            .filter(|row| row_event_id(row) == Some(overlap_id.as_str()))
            .count(),
        1
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row_event_id(row) == Some(older_id.as_str()))
            .count(),
        1
    );
}

#[tokio::test]
async fn relation_before_parent_replaces_fallback_reply_with_ready_preview_in_place() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let room_id = room_id!("!a7-relation:example.org");
    let room = server.sync_joined_room(&client, room_id).await;
    server.mock_room_state_encryption().plain().mount().await;
    let timeline = room.timeline().await.unwrap();
    let (_, mut stream) = timeline.subscribe().await;
    let parent_id = event_id!("$a7-parent");
    let reply_id = event_id!("$a7-reply");
    let f = EventFactory::new().room(room_id);

    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id).add_timeline_event(
                f.text_msg("reply arrived first")
                    .reply_to(parent_id)
                    .sender(*BOB)
                    .event_id(reply_id),
            ),
        )
        .await;
    let initial_ops = project_timeline_diffs(&next_batch(&mut stream).await, client.user_id());
    let initial = projected_event(&initial_ops, reply_id);
    let TimelineViewRow::Message(initial) = initial else {
        panic!("reply was not projected as a message")
    };
    let initial_item_id = initial.event.item_id.clone();
    let fallback = initial.reply.as_ref().expect("reply metadata missing");
    assert_eq!(fallback.event_id, parent_id.as_str());
    assert_eq!(fallback.sender_id, None);
    assert_eq!(fallback.body, "Jump to original");

    server
        .mock_room_event()
        .match_event_id()
        .ok(f
            .text_msg("authoritative parent")
            .sender(*CAROL)
            .event_id(parent_id)
            .into())
        .mock_once()
        .mount()
        .await;
    timeline.fetch_details_for_event(reply_id).await.unwrap();

    let replacement_ops = project_timeline_diffs(&next_batch(&mut stream).await, client.user_id());
    let replacements: Vec<(usize, &TimelineMessageRow)> = replacement_ops
        .iter()
        .filter_map(|op| match op {
            TimelineViewDeltaOp::Set {
                index,
                row: TimelineViewRow::Message(row),
            } if row.event.event_id.as_deref() == Some(reply_id.as_str()) => {
                Some((*index, row.as_ref()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        replacements.len(),
        2,
        "pending and ready must both be ordered Sets"
    );
    assert_eq!(replacements[0].0, replacements[1].0);
    assert!(replacements
        .iter()
        .all(|(_, row)| row.event.item_id == initial_item_id));
    let pending = replacements[0].1.reply.as_ref().unwrap();
    assert_eq!(pending.sender_id, None);
    assert_eq!(pending.body, "Jump to original");
    let ready = replacements[1].1.reply.as_ref().unwrap();
    assert_eq!(ready.event_id, parent_id.as_str());
    assert_eq!(ready.sender_id.as_deref(), Some(CAROL.as_str()));
    assert_eq!(ready.body, "authoritative parent");
}

#[tokio::test]
async fn room_power_grant_and_revoke_reset_existing_row_capabilities_and_redact_preflight() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!a7-authority:example.org");
    let event_id = event_id!("$a7-authority-target");
    let f = EventFactory::new().room(room_id);
    let own_user_id = client.user_id().unwrap().to_owned();
    let mut ordinary_member = BTreeMap::from([(own_user_id.clone(), 0.into())]);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(&own_user_id, RoomVersionId::V11))
                .add_state_event(
                    f.power_levels(&mut ordinary_member)
                        .sender(&own_user_id)
                        .state_key(""),
                )
                .add_timeline_event(f.text_msg("remote message").sender(*BOB).event_id(event_id)),
        )
        .await;
    server.mock_room_state_encryption().plain().mount().await;

    let (updates, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let owner = NativeTimelineOwner::new(
        &client,
        Arc::new(move |batch| {
            let _ = updates.send(batch);
        }),
        7,
    );
    let opened = owner
        .open_at(NativeTimelineOpenRequest {
            room_id: room_id.to_string(),
            position: NativeTimelineOpenPosition::LiveBottom,
        })
        .await
        .unwrap();
    let ordinary = event_capabilities(&opened.snapshot.rows, event_id);
    assert!(!ordinary.pin);
    assert!(!ordinary.redact);
    assert_eq!(
        owner
            .redact_event(room_id.as_str(), event_id.as_str(), None)
            .await,
        Err("v-timeline-redact-permission-denied")
    );

    let mut moderator = BTreeMap::from([(own_user_id.clone(), 100.into())]);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id).add_state_event(
                f.power_levels(&mut moderator)
                    .sender(&own_user_id)
                    .state_key(""),
            ),
        )
        .await;
    let granted_levels = client
        .get_room(room_id)
        .unwrap()
        .power_levels()
        .await
        .unwrap();
    assert!(granted_levels.user_can_redact_event_of_other(&own_user_id));
    assert!(granted_levels.user_can_send_state(&own_user_id, StateEventType::RoomPinnedEvents));
    let granted_rows = next_authority_reset(&mut receiver, event_id).await;
    let granted = event_capabilities(&granted_rows, event_id);
    assert!(granted.pin);
    assert!(granted.redact);

    server
        .mock_room_redact()
        .ok(event_id!("$a7-redaction-ack"))
        .mock_once()
        .mount()
        .await;
    let readback = owner
        .redact_event(room_id.as_str(), event_id.as_str(), Some("moderated"))
        .await
        .unwrap();
    assert_eq!(readback.event_id, event_id.as_str());
    assert_eq!(readback.status, "redacted");

    let mut revoked = BTreeMap::from([(own_user_id.clone(), 0.into())]);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id).add_state_event(
                f.power_levels(&mut revoked)
                    .sender(&own_user_id)
                    .state_key(""),
            ),
        )
        .await;
    let revoked_levels = client
        .get_room(room_id)
        .unwrap()
        .power_levels()
        .await
        .unwrap();
    assert!(!revoked_levels.user_can_redact_event_of_other(&own_user_id));
    assert!(!revoked_levels.user_can_send_state(&own_user_id, StateEventType::RoomPinnedEvents));
    let revoked_rows = next_authority_reset(&mut receiver, event_id).await;
    let revoked = event_capabilities(&revoked_rows, event_id);
    assert!(!revoked.pin);
    assert!(!revoked.redact);
    assert_eq!(
        owner
            .redact_event(room_id.as_str(), event_id.as_str(), None)
            .await,
        Err("v-timeline-redact-permission-denied")
    );
}
