//! Pure protocol helpers: generation checks, sequence ordering, gap/dup.

use super::error::{MatrixIpcError, MatrixIpcErrorCategory};
use super::stream::{ResyncReason, ResyncRequiredPayload, StreamLifecycleState};
use super::version::MATRIX_IPC_PROTOCOL_VERSION;

/// Outcome of applying an incoming stream sequence number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceOutcome {
    /// First message after subscribe, or sequence == last + 1.
    Accept { next_last_applied: u64 },
    /// Sequence already applied (duplicate-delta idempotence).
    Duplicate { last_applied: u64 },
    /// Sequence > last + 1 — gap; require snapshot resubscription.
    Gap {
        last_applied: u64,
        observed: u64,
    },
    /// Sequence before the applied baseline without being an exact duplicate
    /// of the last message (should not happen with monotonic streams).
    Behind { last_applied: u64, observed: u64 },
}

/// Check an incoming sequence against the last applied sequence on a stream.
///
/// Contract:
/// - After a snapshot is applied, `last_applied` is set to the snapshot sequence.
/// - Deltas must arrive as last+1.
/// - Exact equal sequence → duplicate (idempotent ignore).
/// - Greater than last+1 → gap → emit `resync_required`.
pub fn check_sequence(last_applied: Option<u64>, incoming: u64) -> SequenceOutcome {
    match last_applied {
        None => SequenceOutcome::Accept {
            next_last_applied: incoming,
        },
        Some(last) if incoming == last + 1 => SequenceOutcome::Accept {
            next_last_applied: incoming,
        },
        Some(last) if incoming == last => SequenceOutcome::Duplicate {
            last_applied: last,
        },
        Some(last) if incoming > last + 1 => SequenceOutcome::Gap {
            last_applied: last,
            observed: incoming,
        },
        Some(last) => SequenceOutcome::Behind {
            last_applied: last,
            observed: incoming,
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
        Err(MatrixIpcError::new(MatrixIpcErrorCategory::StaleSessionGeneration)
            .with_diagnostic(format!(
                "stale_gen:live={live_generation}:msg={envelope_generation}"
            )))
    }
}

/// Reject envelopes with an unsupported protocol version.
pub fn check_protocol_version(protocol_version: u32) -> Result<(), MatrixIpcError> {
    if protocol_version == MATRIX_IPC_PROTOCOL_VERSION {
        Ok(())
    } else {
        Err(MatrixIpcError::new(MatrixIpcErrorCategory::UnsupportedCapability)
            .with_diagnostic(format!(
                "protocol_version:got={protocol_version}:want={MATRIX_IPC_PROTOCOL_VERSION}"
            )))
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
pub fn resync_payload_for_stale_generation(
    stream_id: Option<String>,
) -> ResyncRequiredPayload {
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
