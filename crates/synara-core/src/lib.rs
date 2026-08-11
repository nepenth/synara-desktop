//! # synara-core
//!
//! Transport-agnostic shared native core for Synara (desktop via Tauri, iOS via
//! uniffi). This crate is intentionally EMPTY for the P1-1 workspace-scaffolding
//! slice; later P1 slices move `matrix/dto`, `matrix/ipc`, `matrix/tasks`, and
//! the domain modules here by `git mv` + path updates only (no behavior change).

pub mod dto;
