//! Application-domain modules shared across Synara platforms (SNC-P1-5
//! domain chunks: sync, then room_list, then timeline).
//!
//! These are pure product-domain modules moved out of the Tauri desktop shell
//! via `git mv` + path corrections; behavior is identical to the pre-move
//! src-tauri/`crate::matrix::*` surfaces, which re-export this module.

pub mod auth;
pub mod room_list;
pub mod sync;
pub mod timeline;
pub mod utd_recovery;
