//! Desktop Matrix adapters and shared domain reexports.
//!
//! Native command adapters call SDK-backed Core owners and project bounded
//! Synara DTOs through the versioned IPC contract. Session credentials, stores,
//! timeline handles, crypto decisions, and sync ownership remain in Rust.
//! Shared pure domain models are reexported from synara-core; their behavioral
//! tests do not substitute for the live owner/command integration tests.
//! Obsolete phase markers and the legacy transition harness are retired.

pub mod account_data;
pub mod auth;
pub mod backup;
pub mod client_builder;
pub mod cross_signing;
pub mod dehydrated_devices;
pub mod devices;
#[cfg(test)]
pub use synara_core::dto;

// SNC-P1-3: matrix/ipc moved into crates/synara-core; re-export so all
// `crate::matrix::ipc::…` paths keep resolving (path-only, no behavior change).
pub use synara_core::transport as ipc;
pub mod lifecycle;
pub mod media;
pub mod members;
pub mod notifications;
pub mod presence;
pub mod room_directory;
pub mod room_keys;
pub mod room_list;
pub mod room_ops;
pub mod room_profile;
pub mod rtc_transports;
pub mod search;
pub mod secret_storage;
pub mod send;
pub mod spaces;
pub mod store;
pub mod sync;
pub mod timeline;
pub mod typing;
pub mod user_profile;
pub mod user_status;
pub mod utd_recovery;
pub mod verification;
pub mod widgets;
pub mod x509;
