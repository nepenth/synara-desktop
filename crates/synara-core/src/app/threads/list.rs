//! Product projection of SDK thread-list items onto [`ThreadSummary`].
//!
//! IPC snapshots carry ids, counts, and timestamps only — no raw SDK items
//! and no pagination tokens.

use serde::{Deserialize, Serialize};

use crate::dto::ThreadSummary;

use super::{ThreadError, ThreadIndex, MAX_THREADS_PER_ROOM};

/// Version of the bounded native thread-list snapshot contract.
pub const NATIVE_THREAD_LIST_SCHEMA_VERSION: u32 = 1;

/// Ids-and-counts projection of one SDK `ThreadListItem`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadListItemProjection {
    pub root_event_id: String,
    pub reply_count: u32,
    pub latest_event_id: Option<String>,
    pub latest_origin_server_ts: Option<u64>,
    pub participated: bool,
}

/// Desktop/Core payload for `matrix_thread_list`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeThreadListRequest {
    pub room_id: String,
    pub action: String,
}

/// Native thread-list snapshot for one joined room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeThreadListSnapshot {
    pub schema_version: u32,
    pub room_id: String,
    pub threads: Vec<ThreadSummary>,
    pub end_reached: bool,
    /// True when the SDK page exceeded [`MAX_THREADS_PER_ROOM`] and overflow
    /// roots were dropped instead of panicking.
    pub truncated: bool,
}

impl NativeThreadListSnapshot {
    pub fn empty(room_id: String) -> Self {
        Self {
            schema_version: NATIVE_THREAD_LIST_SCHEMA_VERSION,
            room_id,
            threads: Vec::new(),
            end_reached: true,
            truncated: false,
        }
    }
}

/// Map one list item onto [`ThreadSummary`]. Invalid ids fail closed.
pub fn thread_summary_from_list_item(
    room_id: &str,
    item: ThreadListItemProjection,
) -> Result<ThreadSummary, ThreadError> {
    let summary = ThreadSummary {
        room_id: room_id.to_owned(),
        root_event_id: item.root_event_id,
        reply_count: item.reply_count,
        latest_event_id: item.latest_event_id,
        latest_origin_server_ts: item.latest_origin_server_ts,
        participated: item.participated,
        unread_count: None,
    };
    let mut index = ThreadIndex::new(0);
    index.upsert(summary.clone())?;
    Ok(summary)
}

/// Replace a room's index entries from the current SDK list, honoring the 256
/// cap by truncating overflow instead of panicking.
pub fn rebuild_thread_index(
    index: &mut ThreadIndex,
    room_id: &str,
    items: impl IntoIterator<Item = ThreadListItemProjection>,
) -> (Vec<ThreadSummary>, bool) {
    index.clear_room(room_id);
    let mut truncated = false;
    for item in items {
        match thread_summary_from_list_item(room_id, item) {
            Ok(summary) => {
                if index.upsert(summary).is_err() {
                    truncated = true;
                    break;
                }
            }
            Err(_) => continue,
        }
    }
    debug_assert!(index.list_room(room_id).len() <= MAX_THREADS_PER_ROOM);
    (
        index.list_room(room_id).into_iter().cloned().collect(),
        truncated,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(
        root: &str,
        replies: u32,
        latest: Option<&str>,
        ts: Option<u64>,
        own: bool,
    ) -> ThreadListItemProjection {
        ThreadListItemProjection {
            root_event_id: root.into(),
            reply_count: replies,
            latest_event_id: latest.map(str::to_owned),
            latest_origin_server_ts: ts,
            participated: own,
        }
    }

    #[test]
    fn thread_list_item_maps_to_summary_ids_and_counts() {
        let summary = thread_summary_from_list_item(
            "!r:example.org",
            item("$root", 4, Some("$latest"), Some(99), true),
        )
        .unwrap();
        assert_eq!(summary.room_id, "!r:example.org");
        assert_eq!(summary.root_event_id, "$root");
        assert_eq!(summary.reply_count, 4);
        assert_eq!(summary.latest_event_id.as_deref(), Some("$latest"));
        assert_eq!(summary.latest_origin_server_ts, Some(99));
        assert!(summary.participated);
        assert_eq!(summary.unread_count, None);
    }

    #[test]
    fn invalid_root_fails_closed_without_echoing_the_id() {
        let err = thread_summary_from_list_item(
            "!r:example.org",
            item("not-an-id", 0, None, None, false),
        )
        .unwrap_err();
        assert_eq!(err.diagnostic_id(), "p5.8-invalid-root-event-id");
        assert!(!format!("{err:?}").contains("not-an-id"));
    }

    #[test]
    fn rebuild_truncates_at_the_room_cap() {
        let mut index = ThreadIndex::new(1);
        let items = (0..=MAX_THREADS_PER_ROOM)
            .map(|i| item(&format!("${i}"), 0, None, Some(i as u64), false));
        let (threads, truncated) = rebuild_thread_index(&mut index, "!r:example.org", items);
        assert!(truncated);
        assert_eq!(threads.len(), MAX_THREADS_PER_ROOM);
        assert_eq!(
            threads[0].root_event_id,
            format!("${}", MAX_THREADS_PER_ROOM - 1)
        );
    }
}
