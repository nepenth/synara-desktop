//! V-CRYPTO.4 live secret-storage bootstrap, unlock, import, and reset.
//!
//! Recovery material is handled only by the managed Rust client. A generated
//! recovery key is returned once for the desktop UI to display. It is not
//! written to Downloads.

pub mod live;
