//! Desktop bridges for timeline open/jump-latest through `Core::command`.
//!
//! The shell attaches a `NativeTimelineOwner` that already holds the view-delta
//! emit sink. React still invokes `matrix_timeline_open` and
//! `matrix_timeline_jump_latest`.

use synara_core::app::timeline::{NativeTimelineOpenPosition, NativeTimelineOpenReadback};
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const TIMELINE_OPEN_COMMAND: &str = "matrix_timeline_open";
const TIMELINE_JUMP_LATEST_COMMAND: &str = "matrix_timeline_jump_latest";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn timeline_open(
    core: &Core,
    room_id: String,
    position: NativeTimelineOpenPosition,
) -> Result<NativeTimelineOpenReadback, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: TIMELINE_OPEN_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "roomId": room_id,
                "position": position,
            }),
        })
        .await
        .map_err(map_timeline_open_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| timeline_open_response_error())
}

pub(crate) async fn timeline_jump_latest(
    core: &Core,
    stream_id: String,
) -> Result<NativeTimelineOpenReadback, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: TIMELINE_JUMP_LATEST_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({ "streamId": stream_id }),
        })
        .await
        .map_err(map_timeline_open_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| timeline_open_response_error())
}

fn map_timeline_open_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.3-timeline-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => match error.diagnostic_id.as_deref() {
            Some("v-timeline-view-not-open") => MatrixAuthCommandError::new(
                "InvalidRequest",
                "The native Matrix timeline request is invalid.",
                "v-timeline-view-not-open",
            ),
            Some("v-timeline-normal-room-not-found") => MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix timeline is not available.",
                "v-timeline-normal-room-not-found",
            ),
            Some("d0.3-timeline-room-not-found") => MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix timeline is not available.",
                "d0.3-timeline-room-not-found",
            ),
            Some("v-timeline-thread-room-not-found") => MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix timeline is not available.",
                "v-timeline-thread-room-not-found",
            ),
            Some("v-timeline-thread-root-invalid") => MatrixAuthCommandError::new(
                "InvalidRequest",
                "The native Matrix timeline request is invalid.",
                "v-timeline-thread-root-invalid",
            ),
            Some("v-timeline-thread-open-failed") => MatrixAuthCommandError::new(
                "Unknown",
                "The native Matrix timeline is unavailable.",
                "v-timeline-thread-open-failed",
            ),
            _ => MatrixAuthCommandError::new(
                "InvalidRequest",
                "The native Matrix timeline request is invalid.",
                "d0.3-timeline-invalid-room-id",
            ),
        },
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix timeline is unavailable.",
            "d0.3-timeline-open-failed",
        ),
    }
}

fn timeline_open_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix timeline is unavailable.",
        "d0.3-timeline-open-failed",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_room_keeps_not_found_for_both_open_diagnostics() {
        for diagnostic in [
            "v-timeline-normal-room-not-found",
            "d0.3-timeline-room-not-found",
        ] {
            let error = map_timeline_open_core_error(
                MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                    .with_diagnostic(diagnostic),
            );
            assert_eq!(error.code, "NotFound");
            assert_eq!(error.diagnostic_id, diagnostic);
        }
    }

    #[test]
    fn thread_open_diagnostics_stay_privacy_safe() {
        let invalid = map_timeline_open_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                .with_diagnostic("v-timeline-thread-root-invalid"),
        );
        assert_eq!(invalid.code, "InvalidRequest");
        assert_eq!(invalid.diagnostic_id, "v-timeline-thread-root-invalid");

        let missing = map_timeline_open_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                .with_diagnostic("v-timeline-thread-room-not-found"),
        );
        assert_eq!(missing.code, "NotFound");
        assert_eq!(missing.diagnostic_id, "v-timeline-thread-room-not-found");

        let failed = map_timeline_open_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                .with_diagnostic("v-timeline-thread-open-failed"),
        );
        assert_eq!(failed.code, "Unknown");
        assert_eq!(failed.diagnostic_id, "v-timeline-thread-open-failed");
    }
}
