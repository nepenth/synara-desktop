//! Desktop bridge for MatrixRTC transport snapshot/refresh through `Core::command`.
//!
//! Core owns `NativeRtcTransportsOwner` after the shell attaches it. React
//! still invokes `matrix_rtc_transports_snapshot` / `refresh`. This is not a
//! widget driver.

use synara_core::app::rtc_transports::NativeRtcTransportsSnapshot;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const RTC_TRANSPORTS_SNAPSHOT_COMMAND: &str = "matrix_rtc_transports_snapshot";
const RTC_TRANSPORTS_REFRESH_COMMAND: &str = "matrix_rtc_transports_refresh";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn rtc_transports_snapshot(
    core: &Core,
) -> Result<NativeRtcTransportsSnapshot, MatrixAuthCommandError> {
    dispatch(core, RTC_TRANSPORTS_SNAPSHOT_COMMAND).await
}

pub(crate) async fn rtc_transports_refresh(
    core: &Core,
) -> Result<NativeRtcTransportsSnapshot, MatrixAuthCommandError> {
    dispatch(core, RTC_TRANSPORTS_REFRESH_COMMAND).await
}

async fn dispatch(
    core: &Core,
    command: &'static str,
) -> Result<NativeRtcTransportsSnapshot, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: command.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .map_err(map_rtc_transports_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| rtc_transports_response_error())
}

fn map_rtc_transports_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "p2-rtc-transports-snapshot-no-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native MatrixRTC transport request is invalid.",
            "p2-rtc-transports-snapshot-invalid-payload",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "Native MatrixRTC transports are unavailable.",
            "p2-rtc-transports-snapshot-serialization-failed",
        ),
    }
}

fn rtc_transports_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native MatrixRTC transports are unavailable.",
        "p2-rtc-transports-snapshot-serialization-failed",
    )
}
