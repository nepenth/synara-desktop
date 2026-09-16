use super::*;
use crate::matrix::rtc_transports::NativeRtcTransportsSnapshot;

#[tauri::command]
pub async fn matrix_rtc_transports_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeRtcTransportsSnapshot, MatrixAuthCommandError> {
    crate::bridge::rtc_transports::rtc_transports_snapshot(core.inner().as_ref()).await
}

#[tauri::command]
pub async fn matrix_rtc_transports_refresh(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeRtcTransportsSnapshot, MatrixAuthCommandError> {
    crate::bridge::rtc_transports::rtc_transports_refresh(core.inner().as_ref()).await
}

#[cfg(test)]
fn map_rtc_transports_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "p2-rtc-transports-snapshot-no-session" | "p2-rtc-transports-refresh-no-session" => (
            "Forbidden",
            "No native Matrix session is active.",
        ),
        "p2-rtc-transports-snapshot-invalid-payload"
        | "p2-rtc-transports-refresh-invalid-payload" => (
            "InvalidRequest",
            "The native MatrixRTC transport request is invalid.",
        ),
        _ => ("Unknown", "Native MatrixRTC transports are unavailable."),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rtc_transport_diagnostics_use_privacy_safe_categories() {
        let cases = [
            ("p2-rtc-transports-snapshot-no-session", "Forbidden"),
            ("p2-rtc-transports-refresh-no-session", "Forbidden"),
            ("p2-rtc-transports-snapshot-invalid-payload", "InvalidRequest"),
            ("p2-rtc-transports-refresh-invalid-payload", "InvalidRequest"),
            ("p2-rtc-transports-snapshot-serialization-failed", "Unknown"),
        ];
        for (diagnostic_id, expected_code) in cases {
            let error = map_rtc_transports_error(diagnostic_id);
            assert_eq!(error.code, expected_code);
            assert_eq!(error.diagnostic_id, diagnostic_id);
            assert!(!error.message.contains("@"));
            assert!(!error.message.contains("token"));
            assert!(!error.message.contains("widget"));
        }
    }
}
