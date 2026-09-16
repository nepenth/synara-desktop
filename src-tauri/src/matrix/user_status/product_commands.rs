use super::*;
use crate::matrix::user_status::{NativeUserStatusSnapshot, NativeUserStatusWriteResult};

#[tauri::command]
pub async fn matrix_user_status_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
    user_id: String,
) -> Result<NativeUserStatusSnapshot, MatrixAuthCommandError> {
    crate::bridge::user_status::user_status_snapshot(core.inner().as_ref(), user_id).await
}

#[tauri::command]
pub async fn matrix_user_status_set(
    core: State<'_, Arc<synara_core::Core>>,
    emoji: String,
    text: String,
) -> Result<NativeUserStatusWriteResult, MatrixAuthCommandError> {
    crate::bridge::user_status::user_status_set(core.inner().as_ref(), emoji, text).await
}

#[tauri::command]
pub async fn matrix_user_status_clear(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeUserStatusWriteResult, MatrixAuthCommandError> {
    crate::bridge::user_status::user_status_clear(core.inner().as_ref()).await
}

#[cfg(test)]
fn map_user_status_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "p2-user-status-snapshot-no-session"
        | "p2-user-status-set-no-session"
        | "p2-user-status-clear-no-session" => ("Forbidden", "No native Matrix session is active."),
        "v-user-status-unsupported" => (
            "InvalidRequest",
            "This homeserver does not support user status.",
        ),
        "v-user-status-emoji-cap"
        | "v-user-status-text-cap"
        | "v-user-status-invalid-user-id"
        | "p2-user-status-snapshot-invalid-payload"
        | "p2-user-status-set-invalid-payload"
        | "p2-user-status-clear-invalid-payload" => (
            "InvalidRequest",
            "The native user status request is invalid.",
        ),
        _ => ("Unknown", "Native user status is unavailable."),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_status_diagnostics_use_privacy_safe_categories() {
        let cases = [
            ("p2-user-status-snapshot-no-session", "Forbidden"),
            ("p2-user-status-set-no-session", "Forbidden"),
            ("p2-user-status-clear-no-session", "Forbidden"),
            ("v-user-status-unsupported", "InvalidRequest"),
            ("v-user-status-emoji-cap", "InvalidRequest"),
            ("v-user-status-text-cap", "InvalidRequest"),
            ("v-user-status-invalid-user-id", "InvalidRequest"),
            ("p2-user-status-set-invalid-payload", "InvalidRequest"),
            ("p2-user-status-snapshot-serialization-failed", "Unknown"),
        ];
        for (diagnostic_id, expected_code) in cases {
            let error = map_user_status_error(diagnostic_id);
            assert_eq!(error.code, expected_code);
            assert_eq!(error.diagnostic_id, diagnostic_id);
            assert!(!error.message.contains("@"));
            assert!(!error.message.contains("emoji"));
            assert!(!error.message.contains("token"));
            assert!(!error.message.contains("set_call"));
        }
    }
}
