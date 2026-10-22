//! Desktop bridge for room create through `Core::command`.

use synara_core::app::room_ops::MatrixRoomCreateRequest;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn room_create(
    core: &Core,
    request: MatrixRoomCreateRequest,
) -> Result<String, MatrixAuthCommandError> {
    core.room_create(request)
        .await
        .map_err(map_room_create_core_error)
        .and_then(|room_id| {
            (!room_id.is_empty())
                .then_some(room_id)
                .ok_or_else(room_create_response_error)
        })
}

fn map_room_create_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error
        .diagnostic_id
        .as_deref()
        .unwrap_or("v-rooms-room-create-failed");
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix room create request is invalid.",
            diagnostic,
        ),
        // `v-rooms-room-create-rate-limited`, with the server's retry hint.
        MatrixIpcErrorCategory::RateLimited => MatrixAuthCommandError::new(
            "RateLimited",
            "The homeserver is limiting room creation.",
            diagnostic,
        )
        .with_retry_after_ms(error.retry_after_ms),
        // Includes `v-rooms-room-create-forbidden` (the server refused).
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix room could not be created.",
            diagnostic,
        ),
    }
}

fn room_create_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix room could not be created.",
        "v-rooms-room-create-failed",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Core's classification of the server's `createRoom` error, run here
    /// with the desktop feature set (the Core unit test covers it too).
    #[test]
    fn core_classifies_room_create_rate_limits_and_refusals() {
        use matrix_sdk::ruma::api::error::{ErrorKind, LimitExceededErrorData, RetryAfter};
        use synara_core::app::room_ops::room_create_failure;

        let mut limited = LimitExceededErrorData::new();
        limited.retry_after = Some(RetryAfter::Delay(std::time::Duration::from_secs(4)));
        let failure = room_create_failure(Some(&ErrorKind::LimitExceeded(limited)));
        assert_eq!(failure.diagnostic_id, "v-rooms-room-create-rate-limited");
        assert_eq!(failure.retry_after_ms, Some(4_000));
        assert_eq!(
            room_create_failure(Some(&ErrorKind::Forbidden)).diagnostic_id,
            "v-rooms-room-create-forbidden"
        );
        assert_eq!(
            room_create_failure(None).diagnostic_id,
            "v-rooms-room-create-failed"
        );
    }

    #[test]
    fn rate_limits_and_refusals_keep_their_own_diagnostics() {
        let limited = map_room_create_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::RateLimited)
                .with_diagnostic("v-rooms-room-create-rate-limited")
                .with_retry_after_ms(3_000),
        );
        assert_eq!(limited.code, "RateLimited");
        assert_eq!(limited.diagnostic_id, "v-rooms-room-create-rate-limited");
        assert_eq!(limited.retry_after_ms, Some(3_000));
        let wire = serde_json::to_value(&limited).expect("error json");
        assert_eq!(wire["retryAfterMs"], 3_000);
        assert_eq!(wire["diagnosticId"], "v-rooms-room-create-rate-limited");

        let refused = map_room_create_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::Unknown)
                .with_diagnostic("v-rooms-room-create-forbidden"),
        );
        assert_eq!(refused.diagnostic_id, "v-rooms-room-create-forbidden");
        assert_eq!(refused.retry_after_ms, None);
        assert!(serde_json::to_value(&refused).expect("error json")["retryAfterMs"].is_null());

        // No session is still reported as such.
        let no_session = map_room_create_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-create-no-session"),
        );
        assert_eq!(no_session.diagnostic_id, "d0.4-send-requires-session");

        let other =
            map_room_create_core_error(MatrixIpcError::new(MatrixIpcErrorCategory::Unknown));
        assert_eq!(other.diagnostic_id, "v-rooms-room-create-failed");
    }
}
