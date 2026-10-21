//! Desktop bridges for timeline open/jump-latest through `Core::command`.
//!
//! The shell attaches a `NativeTimelineOwner` that already holds the view-delta
//! emit sink. React still invokes `matrix_timeline_open` and
//! `matrix_timeline_jump_latest`.

use synara_core::app::timeline::{NativeTimelineOpenPosition, NativeTimelineOpenReadback};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn timeline_open(
    core: &Core,
    room_id: String,
    position: NativeTimelineOpenPosition,
) -> Result<NativeTimelineOpenReadback, MatrixAuthCommandError> {
    let response = core
        .timeline_open(synara_core::core_api::MatrixTimelineOpenRequest { room_id, position })
        .await
        .map_err(map_timeline_open_core_error)?;
    Ok(response)
}

pub(crate) async fn timeline_jump_latest(
    core: &Core,
    stream_id: String,
) -> Result<NativeTimelineOpenReadback, MatrixAuthCommandError> {
    let response = core
        .timeline_jump_latest(synara_core::core_api::MatrixTimelineJumpLatestRequest { stream_id })
        .await
        .map_err(map_timeline_open_core_error)?;
    Ok(response)
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
            // Any other Core diagnostic (`v-timeline-focused-open-failed`,
            // pagination or decoration failures, …) passes through unchanged
            // so the renderer can name what failed; Core diagnostics are
            // static, privacy-safe codes.
            Some(diagnostic) => MatrixAuthCommandError::new(
                "Unknown",
                "The native Matrix timeline is unavailable.",
                diagnostic,
            ),
            None => MatrixAuthCommandError::new(
                "InvalidRequest",
                "The native Matrix timeline request is invalid.",
                "d0.3-timeline-invalid-room-id",
            ),
        },
        // Core files `v-timeline-focused-open-failed` and other open
        // failures under Unknown; keep their id instead of a generic one.
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix timeline is unavailable.",
            error
                .diagnostic_id
                .unwrap_or_else(|| "d0.3-timeline-open-failed".to_owned()),
        ),
    }
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
    fn unknown_core_diagnostics_pass_through_instead_of_reading_as_invalid_room() {
        // As Core files it (`timeline_open_owner_error`): category Unknown.
        let focused = map_timeline_open_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::Unknown)
                .with_diagnostic("v-timeline-focused-open-failed"),
        );
        assert_eq!(focused.diagnostic_id, "v-timeline-focused-open-failed");
        assert_eq!(focused.code, "Unknown");
        let invariant = map_timeline_open_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                .with_diagnostic("v-timeline-focused-open-failed"),
        );
        assert_eq!(invariant.diagnostic_id, "v-timeline-focused-open-failed");
        let unexplained =
            map_timeline_open_core_error(MatrixIpcError::new(MatrixIpcErrorCategory::Unknown));
        assert_eq!(unexplained.diagnostic_id, "d0.3-timeline-open-failed");

        let other = map_timeline_open_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                .with_diagnostic("v-timeline-some-future-failure"),
        );
        assert_eq!(other.diagnostic_id, "v-timeline-some-future-failure");

        // Only a rejection without any diagnostic keeps the generic id.
        let bare =
            map_timeline_open_core_error(MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant));
        assert_eq!(bare.diagnostic_id, "d0.3-timeline-invalid-room-id");
        assert_eq!(bare.code, "InvalidRequest");
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
