//! # synara-core
//!
//! Transport-agnostic shared native core for Synara (desktop via Tauri, iOS via
//! uniffi). Domain modules move here by `git mv` + path updates only, keeping
//! behavior identical (P1 slices: dto, transport/ipc, task, then the app/
//! domain chunks).

pub mod dto;

pub mod task;

pub mod transport;
