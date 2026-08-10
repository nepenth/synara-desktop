//! V-CRYPTO.7 — live own-account device list, trust, and action ownership.
//!
//! The WebView receives a bounded presentation projection. Device keys,
//! access tokens, UIAA internals, and raw SDK errors remain in the Rust host.

pub mod live;

pub use live::{
    NativeDeviceDeleteAuthentication, NativeDeviceDeleteChallenge, NativeDeviceDeleteResult,
    NativeDeviceOwner, NativeDeviceSnapshot, PendingDeviceDeletion,
};

pub const MATRIX_DEVICES_MARKER: &str = "matrix-devices-v-crypto-7";

pub fn matrix_devices_markers() -> &'static str {
    MATRIX_DEVICES_MARKER
}
