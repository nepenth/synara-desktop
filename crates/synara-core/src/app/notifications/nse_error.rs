//! Closed, privacy-safe diagnostics for NSE notification resolution.
//! Never serialize SDK Display/Debug output: it can include server URLs, IDs or paths.
use matrix_sdk::ruma::api::error::ErrorKind;
use matrix_sdk::{Error as SdkError, HttpError};
use matrix_sdk_ui::notification_client::Error as NotificationError;

pub fn nse_notification_error_code(error: &NotificationError) -> &'static str {
    match error {
        NotificationError::BuildingLocalClient(_) => "p4-s11-nse-client-init-failed",
        // UnknownRoom may follow an unsuccessful sync. It does not prove the
        // account lacks membership; upstream can mask the preceding sync error.
        NotificationError::UnknownRoom | NotificationError::SlidingSyncEmptyRoom => {
            "p4-s11-nse-room-unavailable"
        }
        NotificationError::InvalidRumaEvent => "p4-s11-nse-invalid-event",
        NotificationError::ContextMissingEvent => "p4-s11-nse-context-missing-event",
        NotificationError::StoreError(_) => "p4-s11-nse-store-unavailable",
        NotificationError::SdkError(error) => sdk_code(error),
    }
}
/// Preserve useful init causes while keeping an unknown initialization failure
/// distinct from an unknown event-fetch failure.
pub fn nse_notification_initialization_error_code(error: &NotificationError) -> &'static str {
    match nse_notification_error_code(error) {
        "p4-s11-nse-event-fetch-failed" => "p4-s11-nse-client-init-failed",
        code => code,
    }
}
fn sdk_code(error: &SdkError) -> &'static str {
    match error {
        SdkError::AuthenticationRequired => "p4-s11-nse-session-rejected",
        SdkError::Http(error) => http_code(error, 0),
        SdkError::CrossProcessLockError(_) => "p4-s11-nse-store-lock-failed",
        SdkError::StateStore(_) | SdkError::EventCacheStore(_) | SdkError::Io(_) => {
            "p4-s11-nse-store-unavailable"
        }
        SdkError::BadCryptoStoreState
        | SdkError::NoOlmMachine
        | SdkError::CryptoStoreError(_)
        | SdkError::OlmError(_)
        | SdkError::MegolmError(_)
        | SdkError::DecryptorError(_) => "p4-s11-nse-crypto-unavailable",
        SdkError::SerdeJson(_) => "p4-s11-nse-invalid-response",
        SdkError::SlidingSync(error) => match error.as_ref() {
            matrix_sdk::sliding_sync::Error::VersionIsMissing => {
                "p4-s11-nse-sliding-sync-version-missing"
            }
            matrix_sdk::sliding_sync::Error::UnauthenticatedUser => "p4-s11-nse-session-rejected",
            matrix_sdk::sliding_sync::Error::BadResponse(_) => "p4-s11-nse-invalid-response",
            _ => "p4-s11-nse-event-fetch-failed",
        },
        _ => "p4-s11-nse-event-fetch-failed",
    }
}
fn http_code(error: &HttpError, depth: u8) -> &'static str {
    if depth > 4 {
        return "p4-s11-nse-event-fetch-failed";
    }
    match error {
        HttpError::Cached(error) => return http_code(error, depth + 1),
        HttpError::Reqwest(error) => {
            return if error.is_timeout() {
                "p4-s11-nse-network-timeout"
            } else {
                "p4-s11-nse-network-unavailable"
            }
        }
        HttpError::RefreshToken(_) => return "p4-s11-nse-session-rejected",
        _ => {}
    }
    match error.client_api_error_kind() {
        Some(ErrorKind::UnknownToken(_) | ErrorKind::MissingToken) => "p4-s11-nse-session-rejected",
        Some(ErrorKind::Forbidden | ErrorKind::GuestAccessForbidden) => "p4-s11-nse-access-denied",
        Some(ErrorKind::LimitExceeded(_)) => "p4-s11-nse-rate-limited",
        Some(ErrorKind::Unrecognized) => "p4-s11-nse-api-incompatible",
        Some(ErrorKind::NotFound) => "p4-s11-nse-context-missing-event",
        _ => "p4-s11-nse-event-fetch-failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nse_error_diagnostics_keep_causes_distinct_without_carrying_error_text() {
        for (error, code) in [
            (
                NotificationError::UnknownRoom,
                "p4-s11-nse-room-unavailable",
            ),
            (
                NotificationError::InvalidRumaEvent,
                "p4-s11-nse-invalid-event",
            ),
            (
                NotificationError::ContextMissingEvent,
                "p4-s11-nse-context-missing-event",
            ),
            (
                NotificationError::SdkError(SdkError::AuthenticationRequired),
                "p4-s11-nse-session-rejected",
            ),
            (
                NotificationError::SdkError(SdkError::NoOlmMachine),
                "p4-s11-nse-crypto-unavailable",
            ),
            (
                NotificationError::SdkError(
                    matrix_sdk::sliding_sync::Error::VersionIsMissing.into(),
                ),
                "p4-s11-nse-sliding-sync-version-missing",
            ),
            (
                NotificationError::SdkError(
                    matrix_sdk::sliding_sync::Error::BadResponse(
                        "https://private.example/user/@alice token=secret".into(),
                    )
                    .into(),
                ),
                "p4-s11-nse-invalid-response",
            ),
        ] {
            let actual = nse_notification_error_code(&error);
            assert_eq!(actual, code);
            assert!(!actual.contains("secret"));
            assert!(!actual.contains("alice"));
            let initialization = nse_notification_initialization_error_code(&error);
            assert!(!initialization.contains("secret"));
            assert!(!initialization.contains("alice"));
        }
    }

    /// Diagnostic codes the shipping NSE path (`synara-nse-core` →
    /// `app::nse_preview`) can return, read from the non-test source.
    fn shipping_nse_codes() -> std::collections::BTreeSet<String> {
        let sources = [
            include_str!("../nse_preview.rs"),
            include_str!("nse_error.rs"),
            include_str!("../../../../synara-nse-core/src/lib.rs"),
        ];
        let mut codes = std::collections::BTreeSet::new();
        for source in sources {
            let production = source
                .split("#[cfg(test)]\nmod tests")
                .next()
                .unwrap_or(source);
            for line in production
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
            {
                for literal in line.split('"').skip(1).step_by(2) {
                    let is_code = (literal.starts_with("p4-") || literal.starts_with("nse-"))
                        && literal.chars().all(|c| {
                            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.'
                        });
                    if is_code {
                        codes.insert(literal.to_owned());
                    }
                }
            }
        }
        codes
    }

    /// iOS maps each code to a fixed diagnostics stage in Swift
    /// (`previewFailureStage`); its XCTest checks the same table. A new or
    /// removed Core code fails here until the shared table is updated.
    #[test]
    fn every_shipping_nse_code_has_a_shared_ios_stage() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/support/notification-policy-vectors.json"
        ))
        .unwrap();
        let pinned = vectors["nsePreviewFailureStages"]["codes"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(shipping_nse_codes(), pinned);
    }
}
