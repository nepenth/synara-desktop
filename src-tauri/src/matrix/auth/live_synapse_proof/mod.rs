//! Opt-in live V-SEND.R-CALL-MEDIA proof against disposable Synapse.
//!
//! Path marker `.../tests.rs` keeps this file outside production matrix
//! client/sync guardrails while remaining crate-private.
//!
//! Gated by:
//! - `SYNARA_RUN_MATRIX_RUST_CALL_MEDIA_LIVE=1`
//! - `SYNARA_MATRIX_HOMESERVER_URL=http://127.0.0.1:<port>` (credential-free HTTP loopback)
//!
//! Exercises the authenticated native CallWidget media route end-to-end:
//! register/login → `matrix_call_media_config` → authenticated media upload →
//! `matrix_media_download` with `MediaFormat::File` → original bytes.
//!
//! The proof intentionally uses one managed `matrix-sdk::Client`, matching the
//! product session owner. JS two-client Synapse CI is not this proof.

mod tests;
