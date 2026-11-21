//! Bounded history backfill for a room-list row whose preview fell behind.
//!
//! The SDK computes a room's latest event from the events its event cache
//! holds in memory. After a gappy sync that is only the new window: five
//! events for a room without a subscription (`ROOM_LIST_TIMELINE_LIMIT`).
//! When none of them can be a preview (edits of an older message, reactions,
//! redactions), the SDK keeps the previous latest event and asks for older
//! events through its automatic back-pagination, which this client keeps off
//! (`apply_event_cache_policy`: at login it paginated every room).
//!
//! So the row kept its old preview, time and place in the list while the room
//! was active. An agent that streams its reply by editing one message does
//! this on every catch-up, for example when the iOS app returns from the
//! background. Opening the room paginated it, which is why the row was right
//! afterwards.
//!
//! This task does the SDK's backfill for that case only: a joined room that
//! already has a latest event, whose loaded events are all newer than it. It
//! loads a few pages, one room at a time, once per newest event. A room with
//! no latest event at all (most rooms on a fresh login) is left alone, so a
//! login still sends no `/messages`.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use matrix_sdk::latest_events::LatestEventValue;
use matrix_sdk::ruma::{OwnedEventId, OwnedRoomId, RoomId};
use matrix_sdk::{Client, Room, RoomState};
use tokio::sync::broadcast::error::{RecvError, TryRecvError};

use crate::app::sync::{spawn_supervised, RestartPolicy, SupervisedTask};

/// Events asked for per page.
const BACKFILL_BATCH: u16 = 20;

/// Pages loaded for one room and one newest event before giving up.
pub(super) const BACKFILL_MAX_BATCHES: usize = 3;

/// Time for the SDK to recompute a latest event after the event cache
/// changed, and for a burst of updates to gather.
const BACKFILL_SETTLE: Duration = Duration::from_millis(400);

/// Rooms remembered as already tried; cleared when full.
const BACKFILL_MAX_TRACKED: usize = 4096;

/// The newest loaded event of `room` when its latest event is behind it and
/// is not among the loaded events. `None` when the preview can be trusted or
/// nothing is known to be newer.
pub(super) async fn stale_preview_marker(room: &Room) -> Option<OwnedEventId> {
    if room.state() != RoomState::Joined {
        return None;
    }
    // A local echo is the newest thing there is; no latest event at all is
    // the login case this must not paginate.
    let LatestEventValue::Remote(latest) = room.latest_event() else {
        return None;
    };
    let latest_id = latest.event_id()?.to_owned();
    let latest_ts = room.latest_event_timestamp()?;
    let (cache, _drop_handles) = room.event_cache().await.ok()?;
    let events = cache.events().await.ok()?;
    if events
        .iter()
        .any(|event| event.event_id().is_some_and(|id| id == latest_id))
    {
        // The SDK saw the newer events and still chose this one.
        return None;
    }
    let (newest_id, newest_ts) = events
        .iter()
        .rev()
        .find_map(|event| Some((event.event_id()?.to_owned(), event.timestamp()?)))?;
    (newest_ts > latest_ts).then_some(newest_id)
}

/// Load older events of `room_id` until its latest event is among the loaded
/// events, the room's start is reached or [`BACKFILL_MAX_BATCHES`] pages were
/// loaded. Each page makes the SDK recompute the latest event, which reaches
/// the room list as a normal room update. Returns the pages loaded.
pub(super) async fn backfill_stale_preview(
    client: &Client,
    room_id: &RoomId,
    attempted: &mut HashMap<OwnedRoomId, OwnedEventId>,
) -> usize {
    let Some(room) = client.get_room(room_id) else {
        return 0;
    };
    let Some(marker) = stale_preview_marker(&room).await else {
        return 0;
    };
    if attempted.get(room_id) == Some(&marker) {
        return 0;
    }
    if attempted.len() >= BACKFILL_MAX_TRACKED {
        attempted.clear();
    }
    attempted.insert(room_id.to_owned(), marker);
    let Ok((cache, _drop_handles)) = room.event_cache().await else {
        return 0;
    };
    let mut batches = 0;
    while batches < BACKFILL_MAX_BATCHES {
        let Ok(outcome) = cache.pagination().run_backwards_once(BACKFILL_BATCH).await else {
            break;
        };
        batches += 1;
        if outcome.reached_start {
            break;
        }
        tokio::time::sleep(BACKFILL_SETTLE).await;
        if stale_preview_marker(&room).await.is_none() {
            break;
        }
    }
    batches
}

/// Watch the event cache and backfill rooms whose preview fell behind.
pub(super) fn spawn_latest_event_backfill(client: &Client) -> SupervisedTask {
    let client = client.clone();
    // Subscribed before spawning so the first sync response is not missed.
    let mut first = Some(client.event_cache().subscribe_to_room_generic_updates());
    spawn_supervised(RestartPolicy::default(), move || {
        let client = client.clone();
        let mut updates = first
            .take()
            .unwrap_or_else(|| client.event_cache().subscribe_to_room_generic_updates());
        async move {
            let mut attempted = HashMap::new();
            let mut pending: HashSet<OwnedRoomId> = HashSet::new();
            loop {
                match updates.recv().await {
                    Ok(update) => {
                        pending.insert(update.room_id);
                    }
                    // Which rooms changed is lost; their next update or the
                    // room being opened corrects them.
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => return,
                }
                tokio::time::sleep(BACKFILL_SETTLE).await;
                loop {
                    match updates.try_recv() {
                        Ok(update) => {
                            pending.insert(update.room_id);
                        }
                        Err(TryRecvError::Lagged(_)) => continue,
                        Err(TryRecvError::Empty | TryRecvError::Closed) => break,
                    }
                }
                for room_id in pending.drain() {
                    backfill_stale_preview(&client, &room_id, &mut attempted).await;
                }
            }
        }
    })
}
