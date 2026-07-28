//! P8.2 — Device list / trust projection foundation (harness).
//!
//! Pure index of product device summaries. **No device keys, tokens, or
//! recovery material.** No SDK crypto APIs, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p8.2-devices.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod index;

pub use error::DeviceError;
pub use index::{DeviceIndex, DeviceSummary, MAX_DEVICES};

/// Static marker for link / schema smoke.
pub const MATRIX_DEVICES_MARKER: &str = "matrix-devices-p8.2";

/// Touch device paths so they remain linked in non-test builds.
pub fn matrix_devices_markers() -> &'static str {
    let idx = DeviceIndex::new(0);
    debug_assert!(idx.is_empty());
    debug_assert_eq!(idx.len(), 0);
    debug_assert_eq!(MATRIX_DEVICES_MARKER, "matrix-devices-p8.2");
    MATRIX_DEVICES_MARKER
}

#[cfg(test)]
mod tests;
