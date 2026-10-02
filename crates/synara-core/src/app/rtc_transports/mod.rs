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
