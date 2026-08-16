//! P5.9 — Custom Synara raw-content extraction foundation (harness).
//!
//! Allowlisted extraction of agent / custom event content fields with optional
//! unknown-field preservation (short strings only). **No full JSON dumps, no
//! tokens/secrets, no dual-backend.**
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p5.9-raw-content.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod extract;

pub use error::RawContentError;
pub use extract::{
    ContentValue, ExtractedContent, RawContentExtractor, DEFAULT_AGENT_ALLOWLIST,
    MATRIX_CUSTOM_MSGTYPE_PREFIX, MAX_FIELDS, MAX_KEY_LEN, MAX_UNKNOWN_FIELDS, MAX_VALUE_LEN,
    SYNARA_AGENT_EVENT_PREFIX,
};

/// Static marker for link / schema smoke.
pub const MATRIX_RAW_CONTENT_MARKER: &str = "matrix-raw-content-p5.9";

/// Touch raw-content paths so they remain linked in non-test builds.
pub fn matrix_raw_content_markers() -> &'static str {
    let ext = RawContentExtractor::new(0);
    debug_assert!(
        ext.allowlist().contains(&"body".to_owned()) || ext.allowlist().iter().any(|k| k == "body")
    );
    debug_assert_eq!(SYNARA_AGENT_EVENT_PREFIX, "dev.synara.");
    debug_assert_eq!(MATRIX_RAW_CONTENT_MARKER, "matrix-raw-content-p5.9");
    MATRIX_RAW_CONTENT_MARKER
}

#[cfg(test)]
mod tests;
