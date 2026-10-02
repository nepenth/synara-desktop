//! P5.9 — Custom Synara raw-content extraction foundation (harness).
//!
//! Allowlisted extraction of agent / custom event content fields with optional
//! unknown-field preservation (short strings only). **No full JSON dumps, no
//! tokens/secrets, no dual-backend.**
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p5.9-raw-content.md`

mod error;
mod extract;

pub use error::RawContentError;
pub use extract::{
    ContentValue, ExtractedContent, RawContentExtractor, DEFAULT_AGENT_ALLOWLIST,
    MATRIX_CUSTOM_MSGTYPE_PREFIX, MAX_FIELDS, MAX_KEY_LEN, MAX_UNKNOWN_FIELDS, MAX_VALUE_LEN,
    SYNARA_AGENT_EVENT_PREFIX,
};

#[cfg(test)]
mod tests;
