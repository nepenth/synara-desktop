//! Synara-owned Matrix domain DTOs (P1.4).
//!
//! Product-oriented projections for Matrix IPC snapshot/delta bodies.
//! **No** `matrix_sdk` / Ruma types. **No** access/refresh tokens on the wire.
//! **No** large media byte arrays — handles/URIs/paths only.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p1.4-domain-dtos.md`
//! Shared fixtures: `docs/matrix-rust-sdk/dto/fixtures/`
//!
//! Independent of `matrix::ipc` transport envelopes (P1.3). Domain bodies may
//! later compose into snapshot/delta payloads; P1.3 envelopes remain stable.

#![allow(dead_code)]
#![allow(unused_imports)]

mod ids;
mod media;
mod member;
mod notification;
mod receipt;
mod relation;
mod room;
mod search;
mod security;
mod session;
mod space;
mod thread;
mod timeline;
mod typing;
mod upload;

pub use ids::*;
pub use media::*;
pub use member::*;
pub use notification::*;
pub use receipt::*;
pub use relation::*;
pub use room::*;
pub use search::*;
pub use security::*;
pub use session::*;
pub use space::*;
pub use thread::*;
pub use timeline::*;
pub use typing::*;
pub use upload::*;

/// Marker that domain DTO modules are linked (no Client / network / Tauri cmds).
pub const MATRIX_DTO_MARKER: &str = "matrix-domain-dtos-p1.4";

/// Policy: media bytes must never ride JSON IPC (mirrors IPC constant).
pub const FORBID_MEDIA_BYTES_OVER_JSON_IPC: bool = true;

/// Field names that must never appear on wire domain DTOs (privacy / security).
pub const FORBIDDEN_WIRE_FIELD_NAMES: &[&str] = &[
    "access_token",
    "accessToken",
    "refresh_token",
    "refreshToken",
    "password",
    "recovery_key",
    "recoveryKey",
    "private_key",
    "privateKey",
    "media_bytes",
    "mediaBytes",
    "file_bytes",
    "fileBytes",
    "ciphertext",
    "session_key",
    "sessionKey",
];

#[cfg(test)]
mod tests;
