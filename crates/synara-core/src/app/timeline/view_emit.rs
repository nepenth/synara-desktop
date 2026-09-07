//! Timeline view-delta emit sink (P1 4B first slice).
//!
//! Builds [`TimelineViewDeltaBatch`] and hands it to a shell callback.
//! Desktop maps that onto the existing `matrix-timeline-view-updated` Tauri
//! event. This is not [`crate::platform::Platform::emit`] (IPC envelopes).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::{
    TimelinePaginationState, TimelineReadState, TimelineViewDeltaBatch, TimelineViewDeltaOp,
    TIMELINE_VIEW_SCHEMA_VERSION,
};

/// Shell-supplied sink for timeline view-delta batches.
pub type TimelineViewUpdateEmit = Arc<dyn Fn(TimelineViewDeltaBatch) + Send + Sync>;

/// Increments a per-stream revision and emits a privacy-safe view-delta batch.
pub struct ViewDeltaEmitter {
    emit: TimelineViewUpdateEmit,
    session_generation: u64,
    stream_id: String,
    room_id: String,
    revision: Arc<AtomicU64>,
}

impl ViewDeltaEmitter {
    pub fn new(
        emit: TimelineViewUpdateEmit,
        session_generation: u64,
        stream_id: String,
        room_id: String,
        revision: Arc<AtomicU64>,
    ) -> Self {
        Self {
            emit,
            session_generation,
            stream_id,
            room_id,
            revision,
        }
    }

    pub fn emit(
        &self,
        ops: Vec<TimelineViewDeltaOp>,
        read_state: Option<TimelineReadState>,
        pagination: Option<TimelinePaginationState>,
        pinned_event_ids: Option<Vec<String>>,
    ) {
        if ops.is_empty()
            && read_state.is_none()
            && pagination.is_none()
            && pinned_event_ids.is_none()
        {
            return;
        }
        let next_revision = self
            .revision
            .fetch_add(1, Ordering::AcqRel)
            .saturating_add(1);
        (self.emit)(TimelineViewDeltaBatch {
            schema_version: TIMELINE_VIEW_SCHEMA_VERSION,
            session_generation: self.session_generation,
            stream_id: self.stream_id.clone(),
            room_id: self.room_id.clone(),
            revision: next_revision,
            ops,
            read_state,
            pagination,
            pinned_event_ids,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn empty_payload_does_not_emit() {
        let seen = Arc::new(Mutex::new(0_u32));
        let seen_for_emit = seen.clone();
        let emitter = ViewDeltaEmitter::new(
            Arc::new(move |_| {
                *seen_for_emit.lock().expect("lock") += 1;
            }),
            3,
            "stream-1".into(),
            "!room:example.org".into(),
            Arc::new(AtomicU64::new(0)),
        );
        emitter.emit(Vec::new(), None, None, None);
        assert_eq!(*seen.lock().expect("lock"), 0);
    }

    #[test]
    fn revision_increments_and_payload_is_privacy_safe() {
        let batches = Arc::new(Mutex::new(Vec::new()));
        let batches_for_emit = batches.clone();
        let emitter = ViewDeltaEmitter::new(
            Arc::new(move |batch| {
                batches_for_emit.lock().expect("lock").push(batch);
            }),
            7,
            "stream-2".into(),
            "!room:example.org".into(),
            Arc::new(AtomicU64::new(4)),
        );
        emitter.emit(
            Vec::new(),
            Some(TimelineReadState {
                visible_tail_event_id: None,
                receipt_tail_event_id: None,
                own_read_event_id: None,
                unread_anchor_event_id: None,
                is_marked_unread: false,
            }),
            None,
            None,
        );
        let batches = batches.lock().expect("lock");
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].revision, 5);
        assert_eq!(batches[0].session_generation, 7);
        let json = serde_json::to_string(&batches[0]).expect("serialize");
        assert!(!json.contains("access_token"));
        assert!(!json.contains("syt_"));
    }
}
