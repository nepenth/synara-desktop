//! Desktop bridge for `matrix_timeline_set_read_state` through `Core::command`.

use serde::Serialize;
use synara_core::app::timeline::{
    NativeTimelineReadAction, NativeTimelineReadIntent, TimelineViewMetadata,
};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

/// A read action's outcome and the stream's metadata after it. The read
/// state also reaches the renderer as a delta batch, and the rows never
/// change with it, so none cross IPC here.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineReadStateAck {
    pub action: NativeTimelineReadAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_sent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acknowledged_event_id: Option<String>,
    pub view: TimelineViewMetadata,
}

pub(crate) async fn timeline_set_read_state(
    core: &Core,
    stream_id: String,
    action: NativeTimelineReadAction,
    intent: NativeTimelineReadIntent,
    observed_live_tail_event_id: Option<String>,
    public_receipt: bool,
) -> Result<TimelineReadStateAck, MatrixAuthCommandError> {
    let response = core
        .timeline_set_read_state(synara_core::core_api::MatrixTimelineSetReadStateRequest {
            stream_id,
            action,
            intent,
            observed_live_tail_event_id,
            public_receipt,
        })
        .await
        .map_err(map_timeline_set_read_state_core_error)?;
    Ok(TimelineReadStateAck {
        action: response.action,
        receipt_sent: response.receipt_sent,
        acknowledged_event_id: response.acknowledged_event_id,
        view: response.snapshot.into(),
    })
}

fn map_timeline_set_read_state_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.3-timeline-requires-session",
        ),
        // Core's diagnostic goes through one-to-one: only a stream that is
        // really gone reports `v-timeline-view-not-open`.
        MatrixIpcErrorCategory::SdkInvariant => {
            let diagnostic = error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-timeline-view-read-state-invalid");
            let (code, message) = match diagnostic {
                "v-timeline-view-not-open" => (
                    "InvalidRequest",
                    "The native Matrix timeline view is not open.",
                ),
                "v-timeline-read-requires-live-view" => (
                    "InvalidRequest",
                    "The native Matrix timeline view is not at the live bottom.",
                ),
                "v-timeline-read-observed-tail-required"
                | "v-timeline-read-observed-tail-invalid"
                | "v-timeline-read-observed-tail-unexpected"
                | "v-timeline-read-target-unresolved" => (
                    "InvalidRequest",
                    "The native Matrix timeline read target is not current.",
                ),
                "v-timeline-send-thread-receipt-failed" | "v-timeline-send-read-markers-failed" => {
                    ("Unknown", "The read receipt could not be sent.")
                }
                _ => (
                    "InvalidRequest",
                    "The native Matrix timeline read request is invalid.",
                ),
            };
            MatrixAuthCommandError::new(code, message, diagnostic)
        }
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix timeline is unavailable.",
            "v-timeline-view-read-state-failed",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sdk_invariant(diagnostic: &str) -> MatrixAuthCommandError {
        map_timeline_set_read_state_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant).with_diagnostic(diagnostic),
        )
    }

    #[test]
    fn only_a_closed_view_reports_view_not_open() {
        let closed = sdk_invariant("v-timeline-view-not-open");
        assert_eq!(closed.code, "InvalidRequest");
        assert_eq!(closed.diagnostic_id, "v-timeline-view-not-open");

        // Every diagnostic Core classifies as `SdkInvariant` for a read write
        // (`timeline_view_owner_error`) keeps its own id.
        for diagnostic in [
            "v-timeline-read-observed-tail-required",
            "v-timeline-read-observed-tail-invalid",
            "v-timeline-read-observed-tail-unexpected",
            "v-timeline-read-requires-live-view",
            "v-timeline-read-mark-unread-requires-explicit-intent",
            "v-timeline-send-thread-receipt-failed",
            "v-timeline-send-read-markers-failed",
            "v-timeline-read-target-unresolved",
        ] {
            let error = sdk_invariant(diagnostic);
            assert_eq!(error.diagnostic_id, diagnostic);
            assert_ne!(error.message, closed.message, "{diagnostic}");
        }
    }

    #[test]
    fn a_failed_receipt_is_not_reported_as_an_invalid_request() {
        let error = sdk_invariant("v-timeline-send-thread-receipt-failed");
        assert_eq!(error.code, "Unknown");
        assert_eq!(error.message, "The read receipt could not be sent.");
    }

    #[test]
    fn an_invariant_without_a_diagnostic_does_not_claim_the_view_closed() {
        let error = map_timeline_set_read_state_core_error(MatrixIpcError::new(
            MatrixIpcErrorCategory::SdkInvariant,
        ));
        assert_eq!(error.diagnostic_id, "v-timeline-view-read-state-invalid");
    }

    #[test]
    fn session_and_unknown_failures_keep_their_static_codes() {
        let no_session = map_timeline_set_read_state_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-set-read-state-no-session"),
        );
        assert_eq!(no_session.diagnostic_id, "d0.3-timeline-requires-session");
        let unknown = map_timeline_set_read_state_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::Unknown)
                .with_diagnostic("v-timeline-view-mark-unread-failed"),
        );
        assert_eq!(unknown.diagnostic_id, "v-timeline-view-read-state-failed");
    }
}
