//! Pure protocol helpers: generation checks, sequence ordering, gap/dup, bounds.

use super::error::{MatrixIpcError, MatrixIpcErrorCategory};
use super::stream::{ResyncReason, ResyncRequiredPayload, StreamLifecycleState};
use super::version::{
    MATRIX_IPC_PROTOCOL_VERSION, MAX_ENVELOPE_PAYLOAD_JSON_BYTES, MAX_OPEN_STREAMS_PER_SESSION,
    MAX_STREAM_QUEUE_DEPTH,
};
use super::wire_counter::{checked_next_wire_counter, is_valid_wire_counter};

/// Outcome of applying an incoming stream sequence number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceOutcome {
    /// First message after subscribe, or sequence == last + 1.
    Accept { next_last_applied: u64 },
    /// Sequence already applied (duplicate-delta idempotence).
    Duplicate { last_applied: u64 },
    /// Sequence > last + 1 — gap; require snapshot resubscription.
    Gap { last_applied: u64, observed: u64 },
    /// Sequence before the applied baseline without being an exact duplicate
    /// of the last message (should not happen with monotonic streams).
    Behind { last_applied: u64, observed: u64 },
}

/// Check an incoming sequence against the last applied sequence on a stream.
///
/// Contract (R0.3 / REV-004):
/// - Counters must be wire-safe (`<= MAX_WIRE_COUNTER`); out-of-range → gap.
/// - After a snapshot is applied, `last_applied` is set to the snapshot sequence.
/// - Deltas must arrive as checked `last+1` (never overflow/wrap).
/// - Exact equal sequence → duplicate (idempotent ignore).
/// - Greater than expected next → gap → emit `resync_required`.
pub fn check_sequence(last_applied: Option<u64>, incoming: u64) -> SequenceOutcome {
    // Reject counters that cannot cross the JS boundary losslessly.
    if !is_valid_wire_counter(incoming) {
        return SequenceOutcome::Gap {
            last_applied: last_applied.unwrap_or(0),
            observed: incoming,
        };
    }
    match last_applied {
        None => SequenceOutcome::Accept {
            next_last_applied: incoming,
        },
        Some(last) if !is_valid_wire_counter(last) => SequenceOutcome::Gap {
            last_applied: last,
            observed: incoming,
        },
        Some(last) if incoming == last => SequenceOutcome::Duplicate { last_applied: last },
        Some(last) => match checked_next_wire_counter(last) {
            Some(expected) if incoming == expected => SequenceOutcome::Accept {
                next_last_applied: incoming,
            },
            Some(expected) if incoming > expected => SequenceOutcome::Gap {
                last_applied: last,
                observed: incoming,
            },
            Some(_) => SequenceOutcome::Behind {
                last_applied: last,
                observed: incoming,
            },
            // last is already MAX_WIRE_COUNTER: only exact duplicate is safe.
            None => SequenceOutcome::Gap {
                last_applied: last,
                observed: incoming,
            },
        },
    }
}

/// Reject envelopes whose session generation does not match the live session.
pub fn check_session_generation(
    live_generation: u64,
    envelope_generation: u64,
) -> Result<(), MatrixIpcError> {
    if live_generation == envelope_generation {
        Ok(())
    } else {
        Err(
            MatrixIpcError::new(MatrixIpcErrorCategory::StaleSessionGeneration).with_diagnostic(
                format!("stale_gen:live={live_generation}:msg={envelope_generation}"),
            ),
        )
    }
}

/// Reject envelopes with an unsupported protocol version.
pub fn check_protocol_version(protocol_version: u32) -> Result<(), MatrixIpcError> {
    if protocol_version == MATRIX_IPC_PROTOCOL_VERSION {
        Ok(())
    } else {
        Err(
            MatrixIpcError::new(MatrixIpcErrorCategory::UnsupportedCapability).with_diagnostic(
                format!(
                    "protocol_version:got={protocol_version}:want={MATRIX_IPC_PROTOCOL_VERSION}"
                ),
            ),
        )
    }
}

/// Build a `resync_required` payload for a detected sequence gap.
pub fn resync_payload_for_gap(
    stream_id: impl Into<String>,
    last_applied: u64,
    observed: u64,
) -> ResyncRequiredPayload {
    ResyncRequiredPayload {
        stream_id: Some(stream_id.into()),
        reason: ResyncReason::SequenceGap,
        last_applied_sequence: Some(last_applied),
        observed_sequence: Some(observed),
    }
}

/// Build a `resync_required` payload for stale session generation.
pub fn resync_payload_for_stale_generation(stream_id: Option<String>) -> ResyncRequiredPayload {
    ResyncRequiredPayload {
        stream_id,
        reason: ResyncReason::StaleSessionGeneration,
        last_applied_sequence: None,
        observed_sequence: None,
    }
}

/// Pure lifecycle transition helper (no live supervisor).
///
/// Returns `None` if the transition is not allowed.
pub fn transition_stream_lifecycle(
    current: StreamLifecycleState,
    event: StreamLifecycleEvent,
) -> Option<StreamLifecycleState> {
    use StreamLifecycleEvent as E;
    use StreamLifecycleState as S;

    match (current, event) {
        (S::Idle, E::SubscribeRequested) => Some(S::Subscribing),
        (S::Subscribing, E::SubscribedAck) => Some(S::SnapshotPending),
        (S::Subscribing, E::Failed) => Some(S::Failed),
        (S::SnapshotPending, E::SnapshotApplied) => Some(S::Live),
        (S::SnapshotPending, E::ResyncNeeded) => Some(S::ResyncRequired),
        (S::SnapshotPending, E::Failed) => Some(S::Failed),
        (S::Live, E::DeltaApplied) => Some(S::Live),
        (S::Live, E::DuplicateDelta) => Some(S::Live),
        (S::Live, E::ResyncNeeded) => Some(S::ResyncRequired),
        (S::Live, E::UnsubscribeRequested) => Some(S::Unsubscribing),
        (S::Live, E::Failed) => Some(S::Failed),
        (S::ResyncRequired, E::SubscribeRequested) => Some(S::Subscribing),
        (S::ResyncRequired, E::UnsubscribeRequested) => Some(S::Unsubscribing),
        (S::Unsubscribing, E::UnsubscribedAck) => Some(S::Closed),
        (S::Unsubscribing, E::ResourcesReleased) => Some(S::Closed),
        (S::Failed, E::SubscribeRequested) => Some(S::Subscribing),
        (S::Closed, E::SubscribeRequested) => Some(S::Subscribing),
        // Idempotent terminal hooks
        (S::Closed, E::ResourcesReleased) => Some(S::Closed),
        (S::Failed, E::ResourcesReleased) => Some(S::Failed),
        _ => None,
    }
}

/// Lifecycle input events for pure transition tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamLifecycleEvent {
    SubscribeRequested,
    SubscribedAck,
    SnapshotApplied,
    DeltaApplied,
    DuplicateDelta,
    ResyncNeeded,
    UnsubscribeRequested,
    UnsubscribedAck,
    ResourcesReleased,
    Failed,
}

/// Apply sequence check to lifecycle: gaps force resync.
pub fn apply_delta_sequence(
    last_applied: Option<u64>,
    incoming: u64,
) -> (SequenceOutcome, Option<StreamLifecycleEvent>) {
    let outcome = check_sequence(last_applied, incoming);
    let event = match outcome {
        SequenceOutcome::Accept { .. } => Some(StreamLifecycleEvent::DeltaApplied),
        SequenceOutcome::Duplicate { .. } => Some(StreamLifecycleEvent::DuplicateDelta),
        SequenceOutcome::Gap { .. } | SequenceOutcome::Behind { .. } => {
            Some(StreamLifecycleEvent::ResyncNeeded)
        }
    };
    (outcome, event)
}

/// Reject JSON envelope payload bodies that exceed the soft size bound.
///
/// Contract tests and future supervisors share this check. Oversized bodies
/// must use chunking / out-of-band handles (never media bytes over JSON IPC).
pub fn check_payload_json_bounds(byte_len: usize) -> Result<(), MatrixIpcError> {
    if byte_len <= MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        Ok(())
    } else {
        Err(
            MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant).with_diagnostic(format!(
                "payload_too_large:got={byte_len}:max={MAX_ENVELOPE_PAYLOAD_JSON_BYTES}"
            )),
        )
    }
}

/// Reject stream queues that would exceed the retained-depth bound.
pub fn check_stream_queue_depth(depth: usize) -> Result<(), MatrixIpcError> {
    if depth <= MAX_STREAM_QUEUE_DEPTH {
        Ok(())
    } else {
        Err(
            MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant).with_diagnostic(format!(
                "stream_queue_depth:got={depth}:max={MAX_STREAM_QUEUE_DEPTH}"
            )),
        )
    }
}

/// Reject opening more concurrent streams than the per-session bound allows.
pub fn check_open_streams(count: usize) -> Result<(), MatrixIpcError> {
    if count <= MAX_OPEN_STREAMS_PER_SESSION {
        Ok(())
    } else {
        Err(
            MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant).with_diagnostic(format!(
                "open_streams:got={count}:max={MAX_OPEN_STREAMS_PER_SESSION}"
            )),
        )
    }
}
