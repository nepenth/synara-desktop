//! Desktop bridge for MatrixRTC transport snapshot/refresh through `Core::command`.
//!
//! Core owns `NativeRtcTransportsOwner` after the shell attaches it. React
//! still invokes `matrix_rtc_transports_snapshot` / `refresh`. This is not a
//! widget driver.

use synara_core::app::rtc_transports::NativeRtcTransportsSnapshot;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn rtc_transports_snapshot(
    core: &Core,
) -> Result<NativeRtcTransportsSnapshot, MatrixAuthCommandError> {
    core.rtc_transports_snapshot()
        .await
        .map_err(map_rtc_transports_core_error)
}

pub(crate) async fn rtc_transports_refresh(
    core: &Core,
) -> Result<NativeRtcTransportsSnapshot, MatrixAuthCommandError> {
    core.rtc_transports_refresh()
        .await
        .map_err(map_rtc_transports_core_error)
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
