//! Apply [`super::ClientBuildConfig`] to `matrix_sdk::Client::builder`.
//!
//! This is the sole production construction site for `Client::builder` under
//! `src-tauri/src/matrix/`. It never performs login, restore_session, or sync.

use matrix_sdk::config::RequestConfig;
use matrix_sdk::encryption::EncryptionSettings;
use matrix_sdk::Client;

use super::config::{ClientBuildConfig, HomeserverMode};
use super::error::ClientBuilderError;
use crate::matrix::ipc::MatrixIpcErrorCategory;

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

    // Conservative crypto defaults: no auto backup / cross-signing bootstrap
    // until auth lifecycle tasks explicitly opt in.
    let encryption_settings = EncryptionSettings::default();

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
    // Keep a short message without secrets (SDK errors should not include keys).
    let message = format!("{err}");
    let message = if message.len() > 240 {
        format!("{}…", &message[..240])
    } else {
        message
    };

    let (category, diagnostic_id) = classify_build_error(&message);

    ClientBuilderError::SdkBuild {
        category,
        diagnostic_id,
        message,
    }
}

fn classify_build_error(message: &str) -> (MatrixIpcErrorCategory, &'static str) {
    let lower = message.to_ascii_lowercase();
    if lower.contains("store") || lower.contains("sqlite") || lower.contains("io") {
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
