//! V-CRYPTO.4 live secret-storage bootstrap, unlock, import, and reset.
//!
//! Recovery material is handled only by the managed Rust client. Generated
//! recovery keys are written directly to a private host file and never cross
//! the Tauri IPC boundary.

pub mod live;
