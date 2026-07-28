//! P9.2 — Push-rules projection index foundation (harness).
//!
//! Pure index of push rule kinds/actions for notification settings UI.
//! No SDK push-rules network, no dual-backend, no tokens.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p9.2-push-rules.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod index;

pub use error::PushRulesError;
pub use index::{
    PushAction, PushRule, PushRuleKind, PushRulesIndex, MAX_ACTIONS, MAX_PATTERN_CHARS, MAX_RULES,
    MAX_RULE_ID_CHARS,
};

/// Static marker for link / schema smoke.
pub const MATRIX_PUSH_RULES_MARKER: &str = "matrix-push-rules-p9.2";

/// Touch push-rules paths so they remain linked in non-test builds.
pub fn matrix_push_rules_markers() -> &'static str {
    let idx = PushRulesIndex::new(0);
    debug_assert!(idx.is_empty());
    debug_assert!(idx.global_enabled());
    debug_assert_eq!(PushRuleKind::Override.as_str(), "override");
    debug_assert_eq!(MATRIX_PUSH_RULES_MARKER, "matrix-push-rules-p9.2");
    MATRIX_PUSH_RULES_MARKER
}

#[cfg(test)]
mod tests;
