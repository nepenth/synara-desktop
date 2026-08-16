//! Application-domain modules shared across Synara platforms (SNC-P1-5
//! domain chunks: sync, then room_list, then timeline).
//!
//! These are pure product-domain modules moved out of the Tauri desktop shell
//! via `git mv` + path corrections; behavior is identical to the pre-move
//! src-tauri/`crate::matrix::*` surfaces, which re-export this module.

pub mod account_data;
pub mod auth;
pub mod backup;
pub mod client_builder;
pub mod cross_signing;
pub mod crypto_store;
pub mod devices;
pub mod diagnostics;
pub mod legacy;
pub mod lifecycle;
pub mod media;
pub mod media_cache;
pub mod media_export;
pub mod members;
pub mod notifications;
pub mod polls;
pub mod presence;
pub mod raw_content;
pub mod receipts;
pub mod relations;
pub mod room_directory;
pub mod room_keys;
pub mod room_list;
pub mod room_ops;
pub mod room_profile;
pub mod routes;
pub mod search;
pub mod secret_storage;
pub mod security;
pub mod send;
pub mod spaces;
pub mod store;
pub mod supervisor;
pub mod sync;
pub mod threads;
pub mod timeline;
pub mod typing;
pub mod unread;
pub mod user_profile;
pub mod utd_recovery;
pub mod verification;
