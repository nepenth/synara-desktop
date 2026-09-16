//! Approved Matrix Rust SDK Cargo feature surface for synara-core (P2.3).

/// Program crate pin (exact crates.io version; git alignment is
/// `30be7cadc5d50214081d313f6d86a4421f3bd9a7` / tag `matrix-sdk-0.19.0`).
pub const MATRIX_SDK_PIN_VERSION: &str = "0.19.0";

/// Features intentionally enabled on direct `matrix-sdk` dependency after P2.3.
///
/// - `sqlite` — state + event-cache stores via `ClientBuilder::sqlite_store*`
/// - `bundled-sqlite` — portable desktop binary without system libsqlite
/// - `rustls-aws-lc-rs` — rustls crypto provider. Required when
///   `default-features = false` on matrix-sdk 0.19.0.
/// - `experimental-search` — desktop-only local Tantivy index via the
///   synara-core `search-index` feature. NSE / iOS SharedCore must not enable it.
///
/// `e2e-encryption` continues to arrive via `matrix-sdk-ui` feature unification
/// (documented in P1.2) and enables the crypto store when combined with `sqlite`.
pub const APPROVED_MATRIX_SDK_FEATURES: &[&str] = &[
    "sqlite",
    "bundled-sqlite",
    "rustls-aws-lc-rs",
    "experimental-search",
];

/// Features that must **not** be enabled on the product dependency line.
///
/// Experimental / policy-sensitive surfaces stay gated until explicit later tasks.
pub const FORBIDDEN_MATRIX_SDK_FEATURES: &[&str] = &[
    "experimental-widgets",
    "experimental-encrypted-state-events",
    "experimental-element-recent-emojis",
    "experimental-push-secrets",
    "experimental-send-custom-to-device",
    "experimental-x509-identity-verification",
    "automatic-room-key-forwarding",
    "indexeddb",
    "js",
    "uniffi",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn experimental_search_is_approved_for_desktop_index() {
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"experimental-search"));
        assert!(!FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-search"));
        assert!(FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-widgets"));
        assert!(FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-x509-identity-verification"));
        assert!(FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"automatic-room-key-forwarding"));
        assert!(FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-encrypted-state-events"));
    }
}
