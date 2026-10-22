//! Mention badges from the homeserver's highlight notifications.
//!
//! Synapse's simplified sliding sync sends `highlight_count: 0` for every
//! room, and the SDK's client-side mention count only covers the events the
//! room-list window delivered (automatic back-pagination is off). A mention
//! older than that window had no badge after a fresh login. One bounded read
//! of `GET /notifications?only=highlight` ([`HIGHLIGHT_PAGE_LIMIT`] per page,
//! at most [`HIGHLIGHT_MAX_PAGES`] pages) fills the gap. It runs once the
//! first room-list response has settled, and again after a resume or a
//! sync that skipped events in a room the client already knew, at most once
//! per [`HIGHLIGHT_REFRESH_MIN_INTERVAL`].
//!
//! The row's mention count is the larger of the client count and the
//! server highlights still unread here ([`unread_highlight_count`]). Reading
//! a room on this device hides its server highlights at once through the
//! user's own read receipt; the next read refreshes the rest.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::{Stream, StreamExt};
use matrix_sdk::ruma::api::client::push::get_notifications;
use matrix_sdk::ruma::events::receipt::{ReceiptThread, ReceiptType};
use matrix_sdk::ruma::{MilliSecondsSinceUnixEpoch, OwnedRoomId, RoomId, UInt};
use matrix_sdk::sync::RoomUpdates;
use matrix_sdk::{Client, Room};
use tokio::sync::broadcast;

/// Notifications per page.
pub const HIGHLIGHT_PAGE_LIMIT: u32 = 50;
/// Pages per read; older highlights stay with the client count.
pub const HIGHLIGHT_MAX_PAGES: usize = 2;
/// Shortest gap between two reads.
pub const HIGHLIGHT_REFRESH_MIN_INTERVAL: Duration = Duration::from_secs(30);
/// Quiet time after a sync response before reading, so the read sees the
/// state that response produced and its gaps need no second read.
pub const HIGHLIGHT_SETTLE: Duration = Duration::from_secs(1);
/// Longest a pending read waits for sync responses to go quiet.
pub const HIGHLIGHT_MAX_DEFER: Duration = Duration::from_secs(5);

/// One highlight notification, reduced to what the count needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlightRecord {
    /// The notified event's `origin_server_ts` in milliseconds.
    pub event_ts: u64,
    /// The server's `read` flag: the user's receipt was at or after it when
    /// the notification was read.
    pub read: bool,
}

/// The last read's highlights per room, shared by the room-list owner (which
/// refreshes it) and room-list snapshots (which count from it).
#[derive(Debug, Default)]
pub struct ServerHighlights {
    by_room: Mutex<HashMap<OwnedRoomId, Vec<HighlightRecord>>>,
}

impl ServerHighlights {
    pub fn replace(&self, by_room: HashMap<OwnedRoomId, Vec<HighlightRecord>>) {
        if let Ok(mut current) = self.by_room.lock() {
            *current = by_room;
        }
    }

    pub fn records(&self, room_id: &RoomId) -> Vec<HighlightRecord> {
        self.by_room
            .lock()
            .ok()
            .and_then(|by_room| by_room.get(room_id).cloned())
            .unwrap_or_default()
    }
}

/// Highlights still unread: not marked read by the server, and newer than
/// the user's latest read receipt known here (`read_up_to_ts`, the receipt's
/// own timestamp), so reading the room on any device clears them.
pub fn unread_highlight_count(records: &[HighlightRecord], read_up_to_ts: Option<u64>) -> u64 {
    records
        .iter()
        .filter(|record| !record.read && read_up_to_ts.is_none_or(|read| record.event_ts > read))
        .count() as u64
}

/// The user's latest unthreaded or main-thread read receipt (public or
/// private) in `room`, as the receipt's timestamp.
pub async fn own_read_up_to_ts(room: &Room) -> Option<u64> {
    let own = room.own_user_id();
    let mut latest: Option<u64> = None;
    for receipt_type in [ReceiptType::Read, ReceiptType::ReadPrivate] {
        for thread in [ReceiptThread::Unthreaded, ReceiptThread::Main] {
            if let Ok(Some((_, receipt))) = room
                .load_user_receipt(receipt_type.clone(), &thread, own)
                .await
            {
                if let Some(ts) = receipt.ts {
                    let ts: u64 = ts.get().into();
                    latest = Some(latest.map_or(ts, |current| current.max(ts)));
                }
            }
        }
    }
    latest
}

/// Read the newest highlight notifications, at most [`HIGHLIGHT_MAX_PAGES`]
/// pages of [`HIGHLIGHT_PAGE_LIMIT`].
pub async fn fetch_server_highlights(
    client: &Client,
) -> Result<HashMap<OwnedRoomId, Vec<HighlightRecord>>, &'static str> {
    let mut by_room: HashMap<OwnedRoomId, Vec<HighlightRecord>> = HashMap::new();
    let mut from = None;
    for _ in 0..HIGHLIGHT_MAX_PAGES {
        let mut request = get_notifications::v3::Request::new();
        request.only = Some("highlight".to_owned());
        request.limit = Some(UInt::from(HIGHLIGHT_PAGE_LIMIT));
        request.from = from.take();
        let response = client
            .send(request)
            .await
            .map_err(|_| "room-list-highlights-read-failed")?;
        // Synapse sends a `next_token` with every non-empty page; a short
        // page is the last one.
        let full_page = response.notifications.len() >= HIGHLIGHT_PAGE_LIMIT as usize;
        for notification in response.notifications {
            let event_ts = notification
                .event
                .get_field::<MilliSecondsSinceUnixEpoch>("origin_server_ts")
                .ok()
                .flatten()
                .unwrap_or(notification.ts);
            by_room
                .entry(notification.room_id)
                .or_default()
                .push(HighlightRecord {
                    event_ts: event_ts.get().into(),
                    read: notification.read,
                });
        }
        match response.next_token {
            Some(token) if full_page => from = Some(token),
            _ => break,
        }
    }
    Ok(by_room)
}

/// True when the sync skipped events (a limited timeline) in a joined room
/// the client already knew. A room's first appearance is always limited and
/// is not a gap.
pub fn update_has_gap(update: &RoomUpdates, known: &HashSet<OwnedRoomId>) -> bool {
    update
        .joined
        .iter()
        .any(|(room_id, room)| room.timeline.limited && known.contains(room_id))
}

/// When a pending read may start: after the last response has been quiet for
/// [`HIGHLIGHT_SETTLE`] (or [`HIGHLIGHT_MAX_DEFER`] after it became pending)
/// and [`HIGHLIGHT_REFRESH_MIN_INTERVAL`] after the previous read.
pub fn next_read_at(
    pending_since: tokio::time::Instant,
    last_response: tokio::time::Instant,
    last_read: Option<tokio::time::Instant>,
) -> tokio::time::Instant {
    let settled = (last_response + HIGHLIGHT_SETTLE).min(pending_since + HIGHLIGHT_MAX_DEFER);
    match last_read {
        Some(read) => settled.max(read + HIGHLIGHT_REFRESH_MIN_INTERVAL),
        None => settled,
    }
}

/// Keep [`ServerHighlights`] fresh for the session.
///
/// - `responses` yields once per successful room-list sync response (after
///   the value current at subscription).
/// - `running` yields whether the sync service runs; a change to running
///   after it had stopped is a resume.
///
/// The first read waits for the first response, so it reflects this
/// session; every trigger before a read starts is covered by it. Calls
/// `emit` after each successful read so the shells re-read the room list.
pub async fn run_highlight_refresh<R, S, Fetch, FetchFuture, Emit>(
    mut room_updates: broadcast::Receiver<RoomUpdates>,
    responses: R,
    running: S,
    mut known: HashSet<OwnedRoomId>,
    fetch: Fetch,
    store: Arc<ServerHighlights>,
    emit: Emit,
) where
    R: Stream<Item = ()>,
    S: Stream<Item = bool>,
    Fetch: Fn() -> FetchFuture,
    FetchFuture: std::future::Future<
        Output = Result<HashMap<OwnedRoomId, Vec<HighlightRecord>>, &'static str>,
    >,
    Emit: Fn(),
{
    futures_util::pin_mut!(responses);
    futures_util::pin_mut!(running);
    let started = tokio::time::Instant::now();
    let mut pending_since: Option<tokio::time::Instant> = Some(started);
    let mut last_response: Option<tokio::time::Instant> = None;
    let mut last_read: Option<tokio::time::Instant> = None;
    let mut was_running: Option<bool> = None;
    let mut updates_open = true;
    let mut responses_open = true;
    let mut running_open = true;
    loop {
        let read_at = match (pending_since, last_response) {
            (Some(pending), Some(response)) => Some(next_read_at(pending, response, last_read)),
            _ => None,
        };
        tokio::select! {
            update = room_updates.recv(), if updates_open => {
                let gap = match update {
                    Ok(update) => {
                        let gap = update_has_gap(&update, &known);
                        known.extend(update.joined.keys().cloned());
                        gap
                    }
                    // Missed updates may have held a gap.
                    Err(broadcast::error::RecvError::Lagged(_)) => true,
                    Err(broadcast::error::RecvError::Closed) => {
                        updates_open = false;
                        false
                    }
                };
                if gap {
                    pending_since.get_or_insert_with(tokio::time::Instant::now);
                }
            }
            response = responses.next(), if responses_open => match response {
                Some(()) => last_response = Some(tokio::time::Instant::now()),
                None => responses_open = false,
            },
            state = running.next(), if running_open => match state {
                Some(state) => {
                    if state && was_running == Some(false) {
                        pending_since.get_or_insert_with(tokio::time::Instant::now);
                    }
                    was_running = Some(state);
                }
                None => running_open = false,
            },
            () = sleep_until_or_forever(read_at) => {
                pending_since = None;
                last_read = Some(tokio::time::Instant::now());
                if let Ok(by_room) = fetch().await {
                    store.replace(by_room);
                    emit();
                }
            }
        }
    }
}

async fn sleep_until_or_forever(deadline: Option<tokio::time::Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(event_ts: u64, read: bool) -> HighlightRecord {
        HighlightRecord { event_ts, read }
    }

    #[test]
    fn only_highlights_after_the_read_position_count() {
        let records = [record(100, false), record(200, false), record(300, true)];
        assert_eq!(
            unread_highlight_count(&records, None),
            2,
            "server read flag"
        );
        assert_eq!(
            unread_highlight_count(&records, Some(150)),
            1,
            "a receipt newer than a highlight hides it"
        );
        assert_eq!(unread_highlight_count(&records, Some(200)), 0);
        assert_eq!(unread_highlight_count(&[], None), 0);
    }

    #[test]
    fn gaps_count_only_in_rooms_the_client_already_knew() {
        let known_room: OwnedRoomId = "!known:example.org".try_into().unwrap();
        let new_room: OwnedRoomId = "!new:example.org".try_into().unwrap();
        let known = HashSet::from([known_room.clone()]);
        let mut update = RoomUpdates::default();
        let mut limited = matrix_sdk::sync::JoinedRoomUpdate::default();
        limited.timeline.limited = true;
        update.joined.insert(new_room, limited.clone());
        assert!(
            !update_has_gap(&update, &known),
            "a first appearance is no gap"
        );
        update.joined.insert(known_room.clone(), Default::default());
        assert!(
            !update_has_gap(&update, &known),
            "an unlimited update is no gap"
        );
        update.joined.insert(known_room, limited);
        assert!(update_has_gap(&update, &known));
    }

    #[test]
    fn reads_wait_for_quiet_responses_and_the_minimum_interval() {
        let now = tokio::time::Instant::now();
        assert_eq!(next_read_at(now, now, None), now + HIGHLIGHT_SETTLE);
        // Responses that keep coming cannot defer a read forever.
        assert_eq!(
            next_read_at(now, now + Duration::from_secs(60), None),
            now + HIGHLIGHT_MAX_DEFER
        );
        assert_eq!(
            next_read_at(now, now, Some(now)),
            now + HIGHLIGHT_REFRESH_MIN_INTERVAL
        );
    }
}
