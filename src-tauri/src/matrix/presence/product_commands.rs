use super::*;
use crate::matrix::presence::{NativePresenceSnapshotResult, NativePresenceSubscription};

#[tauri::command]
pub async fn matrix_presence_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
    user_id: String,
) -> Result<NativePresenceSnapshotResult, MatrixAuthCommandError> {
    crate::bridge::presence_snapshot::presence_snapshot(core.inner().as_ref(), user_id).await
}

#[tauri::command]
pub async fn matrix_presence_subscribe(
    core: State<'_, Arc<synara_core::Core>>,
    user_id: String,
) -> Result<NativePresenceSubscription, MatrixAuthCommandError> {
    crate::bridge::presence_subscriptions::presence_subscribe(core.inner().as_ref(), user_id).await
}

#[tauri::command]
pub async fn matrix_presence_unsubscribe(
    core: State<'_, Arc<synara_core::Core>>,
    subscription_id: String,
) -> Result<(), MatrixAuthCommandError> {
    crate::bridge::presence_subscriptions::presence_unsubscribe(
        core.inner().as_ref(),
        subscription_id,
    )
    .await
}

pub(super) fn map_presence_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "v-presence-invalid-user-id" | "v-presence-invalid-subscription-id" => (
            "InvalidRequest",
            "The native Matrix presence request is invalid.",
        ),
        "v-presence-user-owner-missing" => ("Forbidden", "No native Matrix session is active."),
        "v-presence-session-not-live" => (
            "Forbidden",
            "The native Matrix presence session is no longer live.",
        ),
        "v-presence-stale-session-generation" => (
            "StaleSessionGeneration",
            "The native Matrix presence session changed.",
        ),
        _ => ("Unknown", "Native Matrix presence is unavailable."),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_and_failure_diagnostics_use_privacy_safe_categories() {
        let cases = [
            ("v-presence-invalid-user-id", "InvalidRequest"),
            ("v-presence-invalid-subscription-id", "InvalidRequest"),
            ("v-presence-user-owner-missing", "Forbidden"),
            ("v-presence-session-not-live", "Forbidden"),
            (
                "v-presence-stale-session-generation",
                "StaleSessionGeneration",
            ),
            ("v-presence-store-read-failed", "Unknown"),
            ("v-presence-event-deserialize-failed", "Unknown"),
            ("v-presence-state-unsupported", "Unknown"),
            ("p4.7-status-msg-cap", "Unknown"),
            ("p4.7-last-active-ts-invalid", "Unknown"),
        ];

        for (diagnostic_id, expected_code) in cases {
            let error = map_presence_error(diagnostic_id);
            assert_eq!(error.code, expected_code);
            assert_eq!(error.diagnostic_id, diagnostic_id);
            assert!(!error.message.contains("@"));
            assert!(!error.message.contains("status"));
            assert!(!error.message.contains("token"));
        }
    }
}
