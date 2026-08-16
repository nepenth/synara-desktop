//! Approved Matrix Rust SDK Cargo feature surface for synara-core (P2.3).

/// Program crate pin (exact crates.io version; git alignment is
/// `1c44fb66214667c6d00acaf72ab592493653708b` / tag `matrix-sdk-0.18.0`).
pub const MATRIX_SDK_PIN_VERSION: &str = "0.18.0";

/// Features intentionally enabled on direct `matrix-sdk` dependency after P2.3.
///
/// - `sqlite` — state + event-cache stores via `ClientBuilder::sqlite_store*`
/// - `bundled-sqlite` — portable desktop binary without system libsqlite
///
/// `e2e-encryption` continues to arrive via `matrix-sdk-ui` feature unification
/// (documented in P1.2) and enables the crypto store when combined with `sqlite`.
pub const APPROVED_MATRIX_SDK_FEATURES: &[&str] = &["sqlite", "bundled-sqlite"];

/// Features that must **not** be enabled on the product dependency line.
///
/// Experimental / policy-sensitive surfaces stay gated until explicit later tasks.
pub const FORBIDDEN_MATRIX_SDK_FEATURES: &[&str] = &[
    "experimental-search",
    "experimental-widgets",
    "experimental-encrypted-state-events",
    "experimental-element-recent-emojis",
    "experimental-push-secrets",
    "experimental-send-custom-to-device",
    "automatic-room-key-forwarding",
    "indexeddb",
    "js",
    "uniffi",
];
