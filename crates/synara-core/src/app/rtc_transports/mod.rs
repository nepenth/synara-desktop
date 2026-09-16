//! MatrixRTC transport discovery (MSC4143 / MSC4519).
//!
//! Product-owned, privacy-safe snapshot of `Client::discover_rtc_transports`.
//! This is not a widget driver, Element Call host, or join path.

mod live;
mod native;

pub use live::NativeRtcTransportsOwner;
pub use native::{
    map_discovered_rtc_transports, NativeRtcTransport, NativeRtcTransportKind,
    NativeRtcTransportsSnapshot, NativeRtcTransportsStatus, MAX_RTC_TRANSPORTS,
    RTC_TRANSPORTS_MARKER,
};

/// Touch discovery paths so they remain linked in non-test builds.
pub fn matrix_rtc_transports_markers() -> &'static str {
    debug_assert_eq!(RTC_TRANSPORTS_MARKER, "matrix-rtc-transports-msc4143");
    debug_assert_eq!(MAX_RTC_TRANSPORTS, 8);
    RTC_TRANSPORTS_MARKER
}
