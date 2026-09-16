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
/// - `experimental-widgets` — compile pin for the experimental widget host.
///   Runtime enablement is a separate in-client setting (default off).
///
/// `e2e-encryption` continues to arrive via `matrix-sdk-ui` feature unification
/// (documented in P1.2) and enables the crypto store when combined with `sqlite`.
///
/// `experimental-send-custom-to-device` arrives **transitively** via
/// `experimental-widgets`. It must not be requested as a direct `Cargo.toml`
/// feature (see `FORBIDDEN_MATRIX_SDK_FEATURES`).
pub const APPROVED_MATRIX_SDK_FEATURES: &[&str] = &[
    "sqlite",
    "bundled-sqlite",
    "rustls-aws-lc-rs",
    "experimental-widgets",
];

/// Features that must **not** be enabled on the product dependency line.
///
/// Experimental / policy-sensitive surfaces stay gated until explicit later tasks.
/// `experimental-send-custom-to-device` is a **direct-request** ban only; widgets
/// may pull it transitively. Do not treat a `cargo tree` hit as a quality-gate
/// failure when it is not listed on the `matrix-sdk` features array.
pub const FORBIDDEN_MATRIX_SDK_FEATURES: &[&str] = &[
    "experimental-search",
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
    fn experimental_widgets_is_approved_not_forbidden() {
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"experimental-widgets"));
        assert!(!FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-widgets"));
        assert!(FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-send-custom-to-device"));
        assert!(!APPROVED_MATRIX_SDK_FEATURES.contains(&"experimental-send-custom-to-device"));
    }

    #[test]
    fn widget_driver_type_resolves_with_experimental_widgets() {
        let name = std::any::type_name::<matrix_sdk::widget::WidgetDriver>();
        assert!(
            name.contains("WidgetDriver"),
            "matrix_sdk::widget::WidgetDriver must resolve, got {name}"
        );
    }

    #[test]
    fn core_cargo_toml_requests_widgets_directly_not_custom_to_device() {
        let manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
        assert!(manifest.contains("\"experimental-widgets\""));
        let features_block = manifest
            .split("matrix-sdk = { version = \"=0.19.0\"")
            .nth(1)
            .and_then(|rest| rest.split("matrix-sdk-ui").next())
            .expect("direct matrix-sdk dependency features");
        assert!(
            features_block.contains("experimental-widgets"),
            "direct matrix-sdk features must compile in experimental-widgets"
        );
        assert!(
            !features_block.contains("experimental-send-custom-to-device"),
            "experimental-send-custom-to-device must stay a transitive-only feature"
        );
    }
}
