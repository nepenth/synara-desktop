//! Desktop re-export of Core MatrixRTC transport discovery.
//!
//! Not a widget driver, Element Call host, or join path. Product code must
//! call `Client::discover_rtc_transports` via the Core owner.

pub use synara_core::app::rtc_transports::*;
