//! P4.7 — Presence stream index and V-PRESENCE.USER native owner.
//!
//! Pure projection of per-user presence state. Complements P6.3 typing.
//! The product owner consumes the managed SDK client's global presence stream.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p4.7-presence.md`

// Restored from the pre-split module so `live.rs` (Tauri/SDK owner) keeps the
// same clippy allowances it had as a child of the harness `mod.rs`.
#![allow(dead_code)]
#![allow(unused_imports)]

pub use synara_core::app::presence::*;

pub mod live;
pub use live::start as start_presence_owner;
