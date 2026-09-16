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
/// - `unstable-msc4426` — MSC4426 `m.status` / `m.call` profile fields.
/// - `automatic-room-key-forwarding` — compile-in Megolm gossip among this
///   user's verified devices. Gated by Core `room-key-forwarding` (full-uniffi /
///   desktop only; never NSE). 0.19 has no public `Encryption` setter, so the
///   OlmMachine defaults stay on once compiled.
///
/// `e2e-encryption` continues to arrive via `matrix-sdk-ui` feature unification
/// (documented in P1.2) and enables the crypto store when combined with `sqlite`.
pub const APPROVED_MATRIX_SDK_FEATURES: &[&str] = &[
    "sqlite",
    "bundled-sqlite",
    "rustls-aws-lc-rs",
    "unstable-msc4426",
    "automatic-room-key-forwarding",
];

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
    "experimental-x509-identity-verification",
    "indexeddb",
    "js",
    "uniffi",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forbidden_features_keep_experimental_search() {
        assert!(FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-search"));
        assert!(FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-widgets"));
        assert!(FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-x509-identity-verification"));
        assert!(!APPROVED_MATRIX_SDK_FEATURES.contains(&"experimental-search"));
    }

    #[test]
    fn automatic_room_key_forwarding_is_approved_not_forbidden() {
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"automatic-room-key-forwarding"));
        assert!(!FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"automatic-room-key-forwarding"));
        assert!(FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-widgets"));
        assert!(FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-x509-identity-verification"));
        assert!(!FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"testing"));
        assert!(!APPROVED_MATRIX_SDK_FEATURES.contains(&"testing"));
    }

    #[test]
    fn core_manifest_requests_forwarding_only_via_product_feature() {
        let manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
        assert!(manifest.contains("nse-preview = []"));
        assert!(manifest.contains(r#"full-uniffi = ["room-key-forwarding"]"#));
        assert!(manifest.contains("matrix-sdk/automatic-room-key-forwarding"));
        assert!(manifest.contains("matrix-sdk-crypto/automatic-room-key-forwarding"));
        assert!(
            !manifest.contains(r#"nse-preview = ["room-key-forwarding"]"#),
            "NSE must not compile automatic room-key forwarding"
        );
    }

    #[cfg(feature = "room-key-forwarding")]
    #[test]
    fn olm_machine_forwarding_setters_exist_when_compiled() {
        // 0.19 exposes these only with the crypto cfg. Product cannot reach the
        // live OlmMachine (`Encryption` has no setter; `Client::olm_machine` is
        // pub(crate); `olm_machine_for_testing` needs matrix-sdk/testing).
        let _set_forwarding: fn(&matrix_sdk_crypto::OlmMachine, bool) =
            matrix_sdk_crypto::OlmMachine::set_room_key_forwarding_enabled;
        let _set_requests: fn(&matrix_sdk_crypto::OlmMachine, bool) =
            matrix_sdk_crypto::OlmMachine::set_room_key_requests_enabled;
        assert!(cfg!(feature = "room-key-forwarding"));
    }
}
