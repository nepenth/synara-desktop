//! P7.6 — Media cache retention and privacy policy foundation (harness).
//!
//! Produces deterministic purge plans from cache-entry metadata only. No media
//! bytes, filesystem access, SDK media network, production Tauri commands, or
//! dual-backend selector live here.
//!
//! Authoritative design note:
//! `docs/matrix-rust-sdk/p7.6-cache-privacy.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod policy;

pub use policy::{plan_purge, EntryMeta, PrivacyPurgePlan, RetentionPolicy};

/// Static marker for link / schema smoke.
pub const MATRIX_MEDIA_CACHE_POLICY_MARKER: &str = "matrix-media-cache-policy-p7.6";

/// Touch media-cache policy paths so they remain linked in non-test builds.
pub fn matrix_media_cache_policy_markers() -> &'static str {
    let policy = RetentionPolicy {
        max_entries: 0,
        max_age_secs: None,
        purge_on_logout: false,
    };
    let plan = plan_purge(&[], &policy, 0);
    debug_assert!(plan.is_empty());
    debug_assert_eq!(
        MATRIX_MEDIA_CACHE_POLICY_MARKER,
        "matrix-media-cache-policy-p7.6"
    );
    MATRIX_MEDIA_CACHE_POLICY_MARKER
}

#[cfg(test)]
mod tests;
