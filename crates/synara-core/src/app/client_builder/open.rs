//! Apply [`super::ClientBuildConfig`] to `matrix_sdk::Client::builder`.
//!
//! This is the sole production construction site for `Client::builder` in
//! `synara-core`. It never performs login, restore_session, or sync.

use matrix_sdk::config::RequestConfig;
use matrix_sdk::encryption::{BackupDownloadStrategy, EncryptionSettings};
use matrix_sdk::Client;

use super::ClientBuilderError;
use super::{ClientBuildConfig, HomeserverMode};
use crate::transport::MatrixIpcErrorCategory;

/// Build an **unauthenticated** Matrix Rust SDK client from a validated config.
///
/// - Opens SQLite state/crypto + event-cache paths from P2.2 layout
/// - Applies user agent, timeouts, optional proxy
/// - Leaves SSL verification enabled (product invariant)
/// - Does **not** login, restore a session, or start sync
pub async fn build_unauthenticated_client(
    config: &ClientBuildConfig,
) -> Result<Client, ClientBuilderError> {
    config.validate()?;
    config.ensure_store_dirs()?;

    let request_config = RequestConfig::new()
        .timeout(config.timeouts.request_timeout)
        .retry_limit(config.timeouts.retry_limit);

    // Recovery secrets remain explicitly user/verification driven. Once the
    // SDK receives one, restore all available room keys into the native store.
    let encryption_settings = EncryptionSettings {
        backup_download_strategy: BackupDownloadStrategy::OneShot,
        ..EncryptionSettings::default()
    };

    let mut builder = Client::builder()
        .request_config(request_config)
        .user_agent(&config.user_agent)
        .with_encryption_settings(encryption_settings);

    match config.homeserver_mode {
        HomeserverMode::ExplicitUrl => {
            builder = builder.homeserver_url(config.identity.homeserver_url());
        }
    }

    // Prefer state dir + separate cache path so event-cache does not share the
    // crypto/state tree root layout unnecessarily.
    let passphrase = config.store_passphrase_hex();
    builder = builder.sqlite_store_with_cache_path(
        config.state_store_path(),
        config.cache_store_path(),
        passphrase.as_deref(),
    );

    if let Some(proxy) = &config.network.proxy_url {
        builder = builder.proxy(proxy);
    }

    // Product network policy forbids disable_ssl_verification; enforce again.
    if !config.network.ssl_verification {
        return Err(ClientBuilderError::InvalidConfig(
            "ssl verification must remain enabled for product clients",
        ));
    }

    if config.handle_refresh_tokens {
        builder = builder.handle_refresh_tokens();
    }

    builder.build().await.map_err(map_build_error)
}

fn map_build_error(err: matrix_sdk::ClientBuildError) -> ClientBuilderError {
    // R0.6 / REV-003: classify from the raw SDK text internally, but never
    // export the raw message (it may contain homeserver URLs, paths, or proxy data).
    let raw = format!("{err}");
    let (category, diagnostic_id) = classify_build_error(&raw);

    ClientBuilderError::SdkBuild {
        category,
        diagnostic_id,
        message: safe_build_message(diagnostic_id).to_owned(),
    }
}

fn classify_build_error(message: &str) -> (MatrixIpcErrorCategory, &'static str) {
    let lower = message.to_ascii_lowercase();
    // Check lock before generic store words: CrossProcessLock/keychain lock
    // failures often include both and should be retry/unlock actionable.
    if lower.contains("lock") && (lower.contains("store") || lower.contains("sqlite")) {
        (
            MatrixIpcErrorCategory::StoreLocked,
            "p2.3-sdk-build-store-locked",
        )
    } else if lower.contains("store") || lower.contains("sqlite") || lower.contains("io") {
        (
            MatrixIpcErrorCategory::StoreUnavailable,
            "p2.3-sdk-build-store",
        )
    } else if lower.contains("proxy") || lower.contains("http") || lower.contains("tls") {
        (
            MatrixIpcErrorCategory::Connectivity,
            "p2.3-sdk-build-network",
        )
    } else if lower.contains("homeserver") || lower.contains("url") {
        (
            MatrixIpcErrorCategory::HomeserverUnavailable,
            "p2.3-sdk-build-homeserver",
        )
    } else {
        (
            MatrixIpcErrorCategory::SdkInvariant,
            "p2.3-sdk-build-generic",
        )
    }
}

/// Bounded, non-sensitive public message for SDK build failures.
fn safe_build_message(diagnostic_id: &str) -> &'static str {
    match diagnostic_id {
        "p2.3-sdk-build-store-locked" => "store is locked",
        "p2.3-sdk-build-store" => "store initialization failed",
        "p2.3-sdk-build-network" => "network configuration failed",
        "p2.3-sdk-build-homeserver" => "homeserver configuration failed",
        _ => "client build failed",
    }
}

#[cfg(test)]
mod privacy_tests {
    use super::*;

    #[test]
    fn classify_and_safe_message_never_echo_raw_sdk_text() {
        let hostile = "failed sqlite open at /Users/alice/Library/Application Support/Synara/matrix/acct/state for https://matrix.evil.example/?access_token=syt_LEAK";
        let (category, id) = classify_build_error(hostile);
        assert_eq!(id, "p2.3-sdk-build-store");
        assert_eq!(category, MatrixIpcErrorCategory::StoreUnavailable);
        let msg = safe_build_message(id);
        assert!(!msg.contains("/Users/"));
        assert!(!msg.contains("https://"));
        assert!(!msg.contains("syt_"));
        assert!(!msg.contains("access_token"));
        assert_eq!(msg, "store initialization failed");

        let (cat2, id2) =
            classify_build_error("proxy http://user:p@ss@127.0.0.1:8080 tls handshake failed");
        assert_eq!(id2, "p2.3-sdk-build-network");
        assert_eq!(cat2, MatrixIpcErrorCategory::Connectivity);
        assert_eq!(safe_build_message(id2), "network configuration failed");
    }

    #[test]
    fn local_store_lock_is_distinct_before_generic_store_classification() {
        let (category, id) = classify_build_error(
            "sqlite store lock held at /Users/alice/Library/Application Support/Synara/matrix",
        );
        assert_eq!(category, MatrixIpcErrorCategory::StoreLocked);
        assert_eq!(id, "p2.3-sdk-build-store-locked");
        assert_eq!(safe_build_message(id), "store is locked");
    }
}
