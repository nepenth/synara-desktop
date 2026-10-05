//! Approved Matrix Rust SDK Cargo feature surface for synara-core (P2.3).

/// Program crate pin (exact crates.io version; git alignment is
/// `b18166c68bb958a21f0bca8b2d8320cb53583362` / tag `matrix-sdk-0.19.1`).
pub const MATRIX_SDK_PIN_VERSION: &str = "0.19.1";

/// Approved explicitly requested Matrix SDK capabilities (dependency or Core feature).
///
/// - `sqlite` — state + event-cache stores via `ClientBuilder::sqlite_store*`
/// - `bundled-sqlite` — portable desktop binary without system libsqlite
/// - `rustls-aws-lc-rs` — rustls crypto provider. Required when
///   `default-features = false` on matrix-sdk 0.19.0.
/// - `unstable-msc4426` — MSC4426 `m.status` / `m.call` profile fields.
/// - `automatic-room-key-forwarding` — compile-in Megolm gossip among this
///   user's verified devices. Gated by Core `room-key-forwarding` (full-app /
///   desktop only; never NSE). 0.19 has no public `Encryption` setter, so the
///   OlmMachine defaults stay on once compiled.
/// - `experimental-widgets` — compile pin for the experimental widget host.
///   Runtime enablement is a separate in-client setting (default off).
/// - `experimental-search` — desktop-only local Tantivy index via the
///   synara-core `search-index` feature. NSE / iOS SharedCore must not enable it.
/// - `experimental-encrypted-state-events` — MSC4362 encrypted state. Compile-in
///   decrypt is always on; product create/opt-in stays behind the account setting.
/// - `experimental-x509-identity-verification` — compiled via Core
///   `x509-identity` from **desktop `src-tauri` only**. Runtime-inert until a
///   verifier is injected (Devices setting on **and** a CA PEM imported).
///
/// `e2e-encryption` is explicit on the common dependency so NSE and full-app
/// stores keep the same encryption contract without relying on UI unification.
///
/// `experimental-send-custom-to-device` arrives **transitively** via
/// `experimental-widgets`. It must not be requested as a direct `Cargo.toml`
/// feature (see `FORBIDDEN_MATRIX_SDK_FEATURES`).
pub const APPROVED_MATRIX_SDK_FEATURES: &[&str] = &[
    "sqlite",
    "bundled-sqlite",
    "rustls-aws-lc-rs",
    "unstable-msc4426",
    "automatic-room-key-forwarding",
    "experimental-widgets",
    "experimental-search",
    "experimental-encrypted-state-events",
    "experimental-x509-identity-verification",
];

/// Features that must **not** be enabled on the product dependency line.
///
/// Experimental / policy-sensitive surfaces stay gated until explicit later tasks.
/// `experimental-send-custom-to-device` is a **direct-request** ban only; widgets
/// may pull it transitively. Do not treat a `cargo tree` hit as a quality-gate
/// failure when it is not listed on the `matrix-sdk` features array.
pub const FORBIDDEN_MATRIX_SDK_FEATURES: &[&str] = &[
    "experimental-element-recent-emojis",
    "experimental-push-secrets",
    "experimental-send-custom-to-device",
    "indexeddb",
    "js",
    "uniffi",
];

/// Cargo packages whose requested features must stay disjoint from
/// [`FORBIDDEN_MATRIX_SDK_FEATURES`].
const GATED_MATRIX_SDK_PACKAGES: &[&str] = &["matrix-sdk", "matrix-sdk-ui", "matrix-sdk-sqlite"];

/// Collect explicit capability requests from dependency feature arrays and
/// `crate/feature` (including weak `crate?/feature`) forwarding declarations.
/// Nested table braces are tracked so dependency scans stop at the matching close.
pub fn requested_cargo_features(cargo_toml: &str, crate_name: &str) -> Vec<String> {
    let needle = format!("{crate_name} = {{");
    let mut features = Vec::new();
    let mut search_from = 0;
    while let Some(rel) = cargo_toml[search_from..].find(&needle) {
        let start = search_from + rel + needle.len();
        let Some(block) = cargo_table_block(&cargo_toml[start..]) else {
            break;
        };
        features.extend(quoted_feature_names(block));
        search_from = start + block.len();
    }
    let direct_prefix = format!("{crate_name}/");
    let weak_prefix = format!("{crate_name}?/");
    for quoted in cargo_toml.split('"').skip(1).step_by(2) {
        if let Some(feature) = quoted
            .strip_prefix(&direct_prefix)
            .or_else(|| quoted.strip_prefix(&weak_prefix))
        {
            if !features.iter().any(|existing| existing == feature) {
                features.push(feature.to_owned());
            }
        }
    }
    features
}

fn cargo_table_block(source: &str) -> Option<&str> {
    let mut depth = 1_i32;
    for (index, ch) in source.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&source[..index]);
                }
            }
            _ => {}
        }
    }
    None
}

fn quoted_feature_names(block: &str) -> Vec<String> {
    let Some(features_at) = block.find("features") else {
        return Vec::new();
    };
    let after_features = &block[features_at..];
    let Some(bracket) = after_features.find('[') else {
        return Vec::new();
    };
    let list = &after_features[bracket + 1..];
    let Some(end) = list.find(']') else {
        return Vec::new();
    };
    list[..end]
        .split(',')
        .filter_map(|item| {
            let item = item.trim();
            let start = item.find('"')?;
            let rest = &item[start + 1..];
            let end = rest.find('"')?;
            let name = rest[..end].trim();
            (!name.is_empty()).then(|| name.to_owned())
        })
        .collect()
}

/// True when any requested Cargo feature is still on the forbid-list.
pub fn forbidden_requested_features(cargo_toml: &str) -> Vec<String> {
    let mut hits = Vec::new();
    for package in GATED_MATRIX_SDK_PACKAGES {
        for feature in requested_cargo_features(cargo_toml, package) {
            if FORBIDDEN_MATRIX_SDK_FEATURES.contains(&feature.as_str()) && !hits.contains(&feature)
            {
                hits.push(feature);
            }
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORE_CARGO: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    const DESKTOP_CARGO: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../src-tauri/Cargo.toml"
    ));

    #[test]
    fn forbidden_features_keep_remaining_experimentals() {
        assert!(FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-element-recent-emojis"));
        assert!(!FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-encrypted-state-events"));
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"experimental-encrypted-state-events"));
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"experimental-x509-identity-verification"));
        assert!(!FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-x509-identity-verification"));
    }

    #[test]
    fn automatic_room_key_forwarding_is_approved_not_forbidden() {
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"automatic-room-key-forwarding"));
        assert!(!FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"automatic-room-key-forwarding"));
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"experimental-x509-identity-verification"));
        assert!(!FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"testing"));
        assert!(!APPROVED_MATRIX_SDK_FEATURES.contains(&"testing"));
    }

    #[test]
    fn core_manifest_requests_forwarding_only_via_product_feature() {
        let manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
        assert!(manifest.contains("nse-preview = []"));
        assert!(manifest.contains(r#"full-uniffi = ["full-app", "dep:uniffi"]"#));
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
    }

    #[test]
    fn experimental_widgets_is_approved_not_forbidden() {
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"experimental-widgets"));
        assert!(!FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-widgets"));
        assert!(FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-send-custom-to-device"));
        assert!(!APPROVED_MATRIX_SDK_FEATURES.contains(&"experimental-send-custom-to-device"));
    }

    #[cfg(feature = "full-app")]
    #[test]
    fn widget_driver_type_resolves_with_experimental_widgets() {
        let name = std::any::type_name::<matrix_sdk::widget::WidgetDriver>();
        assert!(
            name.contains("WidgetDriver"),
            "matrix_sdk::widget::WidgetDriver must resolve, got {name}"
        );
    }

    #[test]
    fn core_cargo_toml_requests_widgets_only_for_full_app_not_custom_to_device() {
        // The package owns production feature requests; workspace inheritance
        // owns the version/default policy. Exclude dev-only feature unification.
        let production = CORE_CARGO
            .split("[dev-dependencies]")
            .next()
            .expect("Core production features/dependency sections");
        let features = requested_cargo_features(production, "matrix-sdk");
        assert!(
            features
                .iter()
                .any(|feature| feature == "experimental-widgets"),
            "Core full-app feature must request the widget capability explicitly"
        );
        assert!(
            !features
                .iter()
                .any(|feature| feature == "experimental-send-custom-to-device"),
            "custom to-device must remain transitive through the widget feature"
        );
        let common = production.split("[dependencies]").nth(1).unwrap();
        assert!(!requested_cargo_features(common, "matrix-sdk")
            .iter()
            .any(|feature| feature == "experimental-widgets"));
        assert!(forbidden_requested_features(production).is_empty());

        let workspace = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml"));
        let needle = "matrix-sdk = {";
        let inherited = cargo_table_block(
            &production[production.find(needle).expect("Core SDK dependency") + needle.len()..],
        )
        .expect("Core SDK dependency table");
        assert!(inherited.contains("workspace = true"));
        let shared = cargo_table_block(
            &workspace[workspace.find(needle).expect("workspace SDK dependency") + needle.len()..],
        )
        .expect("workspace SDK dependency table");
        assert!(shared.contains(&format!(r#"version = "={MATRIX_SDK_PIN_VERSION}""#)));
        assert!(shared.contains("default-features = false"));
        assert!(forbidden_requested_features(workspace).is_empty());
    }

    #[test]
    fn experimental_search_is_approved_for_desktop_index() {
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"experimental-search"));
        assert!(!FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-search"));
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"experimental-x509-identity-verification"));
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"experimental-encrypted-state-events"));
    }

    #[test]
    fn encrypted_state_is_approved_and_not_forbidden() {
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"sqlite"));
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"bundled-sqlite"));
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"rustls-aws-lc-rs"));
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"experimental-encrypted-state-events"));
        assert!(!FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-encrypted-state-events"));
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"experimental-widgets"));
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"experimental-search"));
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"experimental-x509-identity-verification"));
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"automatic-room-key-forwarding"));
    }

    #[test]
    fn common_clients_retain_encrypted_state_store_compatibility() {
        let common = CORE_CARGO
            .split("[dependencies]")
            .nth(1)
            .and_then(|rest| rest.split("[dev-dependencies]").next())
            .expect("shared production dependencies");
        for package in ["matrix-sdk", "matrix-sdk-ui"] {
            assert!(
                requested_cargo_features(common, package)
                    .iter()
                    .any(|feature| feature == "experimental-encrypted-state-events"),
                "{package} must retain encrypted-state support in NSE/shared client stores"
            );
        }
    }

    #[test]
    fn x509_identity_is_approved_not_forbidden() {
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"experimental-x509-identity-verification"));
        assert!(!FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-x509-identity-verification"));
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"rustls-aws-lc-rs"));
    }

    #[test]
    fn core_and_desktop_cargo_request_encrypted_state_without_forbidden_flags() {
        for (label, cargo) in [("synara-core", CORE_CARGO), ("src-tauri", DESKTOP_CARGO)] {
            let sdk = requested_cargo_features(cargo, "matrix-sdk");
            let ui = requested_cargo_features(cargo, "matrix-sdk-ui");
            assert!(
                sdk.iter()
                    .any(|feature| feature == "experimental-encrypted-state-events"),
                "{label} matrix-sdk must request experimental-encrypted-state-events, got {sdk:?}"
            );
            assert!(
                ui.iter()
                    .any(|feature| feature == "experimental-encrypted-state-events"),
                "{label} matrix-sdk-ui must request experimental-encrypted-state-events, got {ui:?}"
            );
            let forbidden = forbidden_requested_features(cargo);
            assert!(
                forbidden.is_empty(),
                "{label} Cargo.toml requests still-forbidden features: {forbidden:?}"
            );
        }
    }

    #[test]
    fn parser_checks_direct_and_weak_feature_forwarding() {
        let cargo = r#"
[features]
full-app = ["matrix-sdk/experimental-widgets"]
leak = ["matrix-sdk-ui?/experimental-push-secrets", "other/experimental-element-recent-emojis"]
"#;
        assert_eq!(
            requested_cargo_features(cargo, "matrix-sdk"),
            vec!["experimental-widgets"]
        );
        assert_eq!(
            forbidden_requested_features(cargo),
            vec!["experimental-push-secrets"]
        );
    }

    #[test]
    fn parser_detects_a_forbidden_feature_on_a_gated_package() {
        let cargo = r#"
matrix-sdk = { version = "=0.19.0", features = [
  "sqlite",
  "experimental-element-recent-emojis",
] }
"#;
        assert_eq!(
            forbidden_requested_features(cargo),
            vec!["experimental-element-recent-emojis".to_owned()]
        );
    }
}
