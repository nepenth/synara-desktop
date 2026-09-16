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
/// - `experimental-encrypted-state-events` — MSC4362 encrypted state. Compile-in
///   decrypt is always on; product create/opt-in stays behind the account setting.
///
/// `e2e-encryption` continues to arrive via `matrix-sdk-ui` feature unification
/// (documented in P1.2) and enables the crypto store when combined with `sqlite`.
pub const APPROVED_MATRIX_SDK_FEATURES: &[&str] = &[
    "sqlite",
    "bundled-sqlite",
    "rustls-aws-lc-rs",
    "experimental-encrypted-state-events",
];

/// Features that must **not** be enabled on the product dependency line.
///
/// Experimental / policy-sensitive surfaces stay gated until explicit later tasks.
pub const FORBIDDEN_MATRIX_SDK_FEATURES: &[&str] = &[
    "experimental-search",
    "experimental-widgets",
    "experimental-element-recent-emojis",
    "experimental-push-secrets",
    "experimental-send-custom-to-device",
    "experimental-x509-identity-verification",
    "automatic-room-key-forwarding",
    "indexeddb",
    "js",
    "uniffi",
];

/// Cargo packages whose requested features must stay disjoint from
/// [`FORBIDDEN_MATRIX_SDK_FEATURES`].
const GATED_MATRIX_SDK_PACKAGES: &[&str] = &["matrix-sdk", "matrix-sdk-ui", "matrix-sdk-sqlite"];

/// Collect quoted feature names from every `features = [ ... ]` array on a
/// direct `{crate} = { ... }` dependency line. Nested braces (e.g. other
/// tables) are tracked so the scan stops at the matching close.
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
    fn encrypted_state_is_approved_and_not_forbidden() {
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"sqlite"));
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"bundled-sqlite"));
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"rustls-aws-lc-rs"));
        assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"experimental-encrypted-state-events"));
        assert!(!FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-encrypted-state-events"));
        assert!(FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-widgets"));
        assert!(FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-search"));
        assert!(FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-x509-identity-verification"));
        assert!(FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"automatic-room-key-forwarding"));
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
    fn parser_detects_a_forbidden_feature_on_a_gated_package() {
        let cargo = r#"
matrix-sdk = { version = "=0.19.0", features = [
  "sqlite",
  "experimental-widgets",
] }
"#;
        assert_eq!(
            forbidden_requested_features(cargo),
            vec!["experimental-widgets".to_owned()]
        );
    }
}
