//! Apply [`super::ClientBuildConfig`] to `matrix_sdk::Client::builder`.
//!
//! This is the sole production construction site for `Client::builder` in
//! `synara-core`. It never performs login, restore_session, or sync.

use matrix_sdk::config::{RequestConfig, StoreConfig};
use matrix_sdk::cross_process_lock::CrossProcessLockConfig;
use matrix_sdk::encryption::{BackupDownloadStrategy, EncryptionSettings};
#[cfg(feature = "search-index")]
use matrix_sdk::search_index::SearchIndexStoreKind;
use matrix_sdk::Client;
use matrix_sdk_crypto::CollectStrategy;

use super::ClientBuilderError;
use super::{ClientBuildConfig, HomeserverMode};
use crate::transport::MatrixIpcErrorCategory;

/// Which devices receive this client's room keys.
///
/// matrix-sdk 0.19.1 recommends `IdentityBased` (MSC4153), which only shares
/// with devices their owner cross-signed. Synara's agent bots and many small
/// accounts never set up cross-signing, so identity-based sharing would
/// silently stop them reading new messages. Keep the SDK default that Element
/// X also uses outside its opt-in "exclude insecure devices" mode. The
/// timeline's authenticity shields and the identity-change banner surface the
/// trust problems this strategy does not block.
const ROOM_KEY_RECIPIENT_STRATEGY: CollectStrategy = CollectStrategy::AllDevices;

/// Build an **unauthenticated** Matrix Rust SDK client from a validated config.
///
/// - Opens SQLite state/crypto + event-cache paths from P2.2 layout
/// - Applies user agent, timeouts, optional proxy
/// - Leaves SSL verification enabled (product invariant)
/// - Does **not** login, restore a session, or start sync
pub async fn build_unauthenticated_client(
    config: &ClientBuildConfig,
) -> Result<Client, ClientBuilderError> {
    build_client(config, ClientStoreMode::Persistent).await
}

/// Build an unauthenticated client with explicit in-memory state, crypto,
/// event-cache and media stores. Used only for bounded server-session cleanup
/// before a product session exists. Creates no product layout, search index,
/// session callback or vault entries; transport/encryption policy is shared.
pub async fn build_memory_only_client(
    config: &ClientBuildConfig,
) -> Result<Client, ClientBuilderError> {
    build_client(config, ClientStoreMode::Memory).await
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ClientStoreMode {
    Persistent,
    Memory,
}

async fn build_client(
    config: &ClientBuildConfig,
    store_mode: ClientStoreMode,
) -> Result<Client, ClientBuilderError> {
    config.validate()?;
    if store_mode == ClientStoreMode::Persistent {
        config.ensure_store_dirs()?;
    }

    let request_config = RequestConfig::new()
        .timeout(config.timeouts.request_timeout)
        .retry_limit(config.timeouts.retry_limit);

    // Recovery secrets remain explicitly user/verification driven. Once the
    // SDK has backup access it fetches the room key for each event that fails
    // to decrypt. `OneShot` downloads the whole backup once, logs a warning on
    // failure and never retries, which the SDK documents as unworkable for any
    // sizeable account.
    let encryption_settings = EncryptionSettings {
        backup_download_strategy: BackupDownloadStrategy::AfterDecryptionFailure,
        ..EncryptionSettings::default()
    };

    let cross_process = match store_mode {
        ClientStoreMode::Memory => CrossProcessLockConfig::SingleProcess,
        ClientStoreMode::Persistent => {
            CrossProcessLockConfig::multi_process(config.cross_process_store_lock_holder())
        }
    };
    let mut builder = Client::builder()
        .request_config(request_config)
        .user_agent(&config.user_agent)
        .with_encryption_settings(encryption_settings)
        .with_room_key_recipient_strategy(ROOM_KEY_RECIPIENT_STRATEGY)
        .cross_process_store_config(cross_process.clone());

    match config.homeserver_mode {
        HomeserverMode::ExplicitUrl => {
            builder = builder.homeserver_url(config.identity.homeserver_url());
        }
    }

    match store_mode {
        ClientStoreMode::Memory => {
            // StoreConfig::new explicitly selects SDK memory implementations
            // for all four stores; do not rely on a future builder default.
            builder = builder.store_config(StoreConfig::new(cross_process));
        }
        ClientStoreMode::Persistent => {
            // Keep state and cache paths separate in the product layout.
            let passphrase = config.store_passphrase_hex();
            builder = builder.sqlite_store_with_cache_path(
                config.state_store_path(),
                config.cache_store_path(),
                passphrase.as_deref(),
            );
            #[cfg(feature = "search-index")]
            {
                builder = apply_encrypted_search_index(builder, config, passphrase.as_deref())?;
            }
        }
    }

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

    builder = builder.with_enable_automatic_back_pagination(true);

    #[cfg(feature = "x509-identity")]
    {
        crate::app::x509::ensure_aws_lc_rustls_provider();
        builder = apply_x509_identity_hooks(
            builder,
            config.account_root(),
            store_mode == ClientStoreMode::Persistent,
        );
    }

    builder.build().await.map_err(map_build_error)
}

#[cfg(feature = "search-index")]
fn apply_encrypted_search_index(
    builder: matrix_sdk::ClientBuilder,
    config: &ClientBuildConfig,
    passphrase: Option<&str>,
) -> Result<matrix_sdk::ClientBuilder, ClientBuilderError> {
    if !config.indexed_message_search() {
        return Ok(builder);
    }
    // Never persist a plaintext on-disk index: decrypted bodies must not sit
    // in a world-readable Tantivy tree. Missing passphrase skips disk persistence.
    let Some(password) = passphrase.filter(|value| !value.is_empty()) else {
        return Ok(builder);
    };
    Ok(
        builder.search_index_store(SearchIndexStoreKind::EncryptedDirectory(
            config.search_store_path().to_path_buf(),
            password.to_owned(),
        )),
    )
}

#[cfg(feature = "x509-identity")]
fn apply_x509_identity_hooks(
    mut builder: matrix_sdk::ClientBuilder,
    account_root: &std::path::Path,
    record_configuration: bool,
) -> matrix_sdk::ClientBuilder {
    let runtime = crate::app::x509::load_runtime(account_root);
    let inject = runtime.should_inject_verifier();
    if inject {
        if let Some(verifier) = crate::app::x509::build_verifier(&runtime.trust_anchors_pem) {
            builder = builder.with_x509_verifier(Some(verifier));
            if let (Some(cert), Some(key)) = (
                runtime.signer_cert_pem.as_deref(),
                runtime.signer_key_pem.as_deref(),
            ) {
                if let Some(signer) = crate::app::x509::build_signer(cert, key) {
                    builder = builder.with_x509_signer(Some(signer));
                }
            }
            if record_configuration {
                crate::app::x509::record_applied(account_root, true);
            }
            return builder;
        }
    }
    if record_configuration {
        crate::app::x509::record_applied(account_root, false);
    }
    builder
}

fn map_build_error(err: matrix_sdk::ClientBuildError) -> ClientBuilderError {
    // Only a typed SQLite corruption result can enable explicit archive recovery.
    // Other SDK failures retain bounded diagnostics; raw paths/URLs never escape.
    let (category, diagnostic_id) = match &err {
        matrix_sdk::ClientBuildError::SqliteStore(store) => classify_sqlite_open_error(store),
        _ => classify_build_error(&err.to_string()),
    };

    ClientBuilderError::SdkBuild {
        category,
        diagnostic_id,
        message: safe_build_message(diagnostic_id).to_owned(),
    }
}

fn classify_sqlite_open_error(
    error: &matrix_sdk_sqlite::OpenStoreError,
) -> (MatrixIpcErrorCategory, &'static str) {
    use std::error::Error;

    // Cipher failures include a wrong passphrase or invalid encrypted payload.
    // Directory failures include permissions/IO. Neither authorizes recovery.
    if !matches!(
        error,
        matrix_sdk_sqlite::OpenStoreError::InitCipher(_)
            | matrix_sdk_sqlite::OpenStoreError::CreateDir(_)
    ) {
        // The SDK's migration/pool wrappers can hide rusqlite itself while
        // preserving its FFI error as a source. Inspect the named result code,
        // never an error string, including SQLite extended corruption codes.
        let mut source: Option<&(dyn Error + 'static)> = Some(error);
        while let Some(current) = source {
            if let Some(sqlite) = current.downcast_ref::<rusqlite::ffi::Error>() {
                return match sqlite.code {
                    rusqlite::ErrorCode::DatabaseCorrupt => (
                        MatrixIpcErrorCategory::StoreCorrupt,
                        "p2.3-sdk-build-store-corrupt",
                    ),
                    rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked => (
                        MatrixIpcErrorCategory::StoreLocked,
                        "p2.3-sdk-build-store-locked",
                    ),
                    // NOTADB, IO, permissions and unknown formats cannot prove
                    // corruption. A foreign/encrypted file can also be NOTADB.
                    _ => (
                        MatrixIpcErrorCategory::StoreUnavailable,
                        "p2.3-sdk-build-store",
                    ),
                };
            }
            source = current.source();
        }
    }
    (
        MatrixIpcErrorCategory::StoreUnavailable,
        "p2.3-sdk-build-store",
    )
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
        "p2.3-sdk-build-store-corrupt" => "store database is corrupt",
        "p2.3-sdk-build-store" => "store initialization failed",
        "p2.3-sdk-build-network" => "network configuration failed",
        "p2.3-sdk-build-homeserver" => "homeserver configuration failed",
        _ => "client build failed",
    }
}

#[cfg(test)]
mod privacy_tests {
    use super::*;

    struct StoreFixture(std::path::PathBuf);

    impl StoreFixture {
        fn new() -> Self {
            #[cfg(feature = "x509-identity")]
            crate::app::x509::ensure_aws_lc_rustls_provider();
            let mut random = [0u8; 16];
            getrandom::fill(&mut random).unwrap();
            let path = std::env::temp_dir().join(format!(
                "synara-client-build-{:032x}",
                u128::from_le_bytes(random)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn config(&self) -> ClientBuildConfig {
            ClientBuildConfig::product_default(
                &self.0,
                crate::app::store::AccountIdentity::new(
                    "@fixture:example.org",
                    "https://example.org",
                )
                .unwrap(),
                None,
            )
            .unwrap()
            .with_indexed_message_search(false)
        }
    }

    impl Drop for StoreFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn assert_private_category(error: ClientBuilderError, expected: MatrixIpcErrorCategory) {
        let ClientBuilderError::SdkBuild {
            category,
            diagnostic_id,
            message,
        } = error
        else {
            panic!("expected bounded SDK build error");
        };
        assert_eq!(category, expected);
        assert_eq!(message, safe_build_message(diagnostic_id));
        for sensitive in ["/Users/", "https://", "access_token", "syt_LEAK"] {
            assert!(!message.contains(sensitive));
        }
    }

    #[tokio::test]
    async fn memory_client_never_creates_product_layout_and_uses_configured_transport() {
        use wiremock::{
            matchers::{header, method, path},
            Mock, MockServer, ResponseTemplate,
        };
        let fixture = StoreFixture::new();
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/_matrix/client/versions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"versions": ["v1.11"]})),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/login"))
            .and(header("user-agent", "SynaraMemoryFactoryFixture"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"flows": []})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let absent_root = fixture.0.join("must-remain-absent");
        let config = ClientBuildConfig::product_default(
            &absent_root,
            crate::app::store::AccountIdentity::new("@fixture:example.org", &server.uri()).unwrap(),
            None,
        )
        .unwrap()
        .with_user_agent("SynaraMemoryFactoryFixture")
        .unwrap();
        let client = build_memory_only_client(&config).await.unwrap();
        assert!(client.user_id().is_none());
        client.matrix_auth().get_login_types().await.unwrap();
        assert!(!absent_root.exists(), "memory construction and SDK requests must not create any account/store/cache/search/X509 layout");
    }

    #[tokio::test]
    async fn memory_client_rejects_invalid_transport_without_touching_store_paths() {
        let fixture = StoreFixture::new();
        let mut config = fixture.config();
        config.network.ssl_verification = false;
        assert!(matches!(
            build_memory_only_client(&config).await,
            Err(ClientBuilderError::InvalidConfig(_))
        ));
        config.network.ssl_verification = true;
        config.network.proxy_url = Some("http://user:password@localhost:8080".into());
        assert!(matches!(
            build_memory_only_client(&config).await,
            Err(ClientBuilderError::InvalidConfig(_))
        ));
        assert!(!config.account_root().exists());
    }

    #[test]
    fn typed_sqlite_codes_only_authorize_proven_database_corruption() {
        for (code, expected) in [
            (
                rusqlite::ffi::SQLITE_CORRUPT,
                MatrixIpcErrorCategory::StoreCorrupt,
            ),
            (
                rusqlite::ffi::SQLITE_CORRUPT | (2 << 8),
                MatrixIpcErrorCategory::StoreCorrupt,
            ),
            (
                rusqlite::ffi::SQLITE_BUSY,
                MatrixIpcErrorCategory::StoreLocked,
            ),
            (
                rusqlite::ffi::SQLITE_LOCKED,
                MatrixIpcErrorCategory::StoreLocked,
            ),
            (
                rusqlite::ffi::SQLITE_NOTADB,
                MatrixIpcErrorCategory::StoreUnavailable,
            ),
            (
                rusqlite::ffi::SQLITE_IOERR,
                MatrixIpcErrorCategory::StoreUnavailable,
            ),
            (
                rusqlite::ffi::SQLITE_CANTOPEN,
                MatrixIpcErrorCategory::StoreUnavailable,
            ),
            (
                rusqlite::ffi::SQLITE_PERM,
                MatrixIpcErrorCategory::StoreUnavailable,
            ),
        ] {
            let error = matrix_sdk::ClientBuildError::SqliteStore(
                matrix_sdk_sqlite::OpenStoreError::LoadVersion(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(code),
                    Some("corrupt locked sqlite at /Users/alice for https://evil/?access_token=syt_LEAK".into()),
                )),
            );
            assert_private_category(map_build_error(error), expected);
        }
        for error in [
            matrix_sdk_sqlite::OpenStoreError::MissingVersion,
            matrix_sdk_sqlite::OpenStoreError::InvalidVersion,
            matrix_sdk_sqlite::OpenStoreError::LoadVersion(rusqlite::Error::InvalidQuery),
            matrix_sdk_sqlite::OpenStoreError::CreateDir(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "corrupt locked sqlite at /Users/alice",
            )),
        ] {
            assert_private_category(
                map_build_error(matrix_sdk::ClientBuildError::SqliteStore(error)),
                MatrixIpcErrorCategory::StoreUnavailable,
            );
        }
    }

    #[tokio::test]
    async fn actual_sdk_btree_corruption_arms_only_explicit_recovery_category() {
        let fixture = StoreFixture::new();
        let config = fixture.config();
        config.ensure_store_dirs().unwrap();
        let path = config
            .state_store_path()
            .join(matrix_sdk_sqlite::STATE_STORE_DATABASE_NAME);
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute_batch("CREATE TABLE fixture(value INTEGER);")
            .unwrap();
        connection.close().unwrap();
        let mut bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(b"SQLite format 3\0"));
        // Preserve the SQLite header, invalidate only page one's B-tree tag.
        // This distinguishes SQLITE_CORRUPT from foreign-file SQLITE_NOTADB.
        bytes[100] = 0;
        std::fs::write(&path, &bytes).unwrap();

        let sdk_error = Client::builder()
            .homeserver_url("https://example.org")
            .sqlite_store_with_cache_path(
                config.state_store_path(),
                config.cache_store_path(),
                None,
            )
            .build()
            .await
            .unwrap_err();
        let matrix_sdk::ClientBuildError::SqliteStore(store_error) = &sdk_error else {
            panic!("fixture must fail at the SDK SQLite boundary");
        };
        assert_eq!(
            classify_sqlite_open_error(store_error),
            (
                MatrixIpcErrorCategory::StoreCorrupt,
                "p2.3-sdk-build-store-corrupt"
            )
        );
        assert_private_category(
            map_build_error(sdk_error),
            MatrixIpcErrorCategory::StoreCorrupt,
        );
        let product_error = build_unauthenticated_client(&config).await.unwrap_err();
        assert_private_category(product_error, MatrixIpcErrorCategory::StoreCorrupt);
        assert_eq!(
            std::fs::read(path).unwrap(),
            bytes,
            "classification never rewrites or archives the failed database"
        );
    }

    #[tokio::test]
    async fn actual_sdk_wrong_passphrase_and_unavailable_path_do_not_arm_recovery() {
        let fixture = StoreFixture::new();
        let state = fixture.0.join("state");
        let cache = fixture.0.join("cache");
        let store = matrix_sdk_sqlite::SqliteStateStore::open(&state, Some("correct-passphrase"))
            .await
            .unwrap();
        drop(store);
        let error = Client::builder()
            .homeserver_url("https://example.org")
            .sqlite_store_with_cache_path(&state, &cache, Some("wrong-passphrase"))
            .build()
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            matrix_sdk::ClientBuildError::SqliteStore(
                matrix_sdk_sqlite::OpenStoreError::InitCipher(_)
            )
        ));
        assert_private_category(
            map_build_error(error),
            MatrixIpcErrorCategory::StoreUnavailable,
        );

        let blocked = fixture.0.join("not-a-directory");
        std::fs::write(&blocked, b"fixture").unwrap();
        let error = Client::builder()
            .homeserver_url("https://example.org")
            .sqlite_store_with_cache_path(&blocked, &cache, None)
            .build()
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            matrix_sdk::ClientBuildError::SqliteStore(
                matrix_sdk_sqlite::OpenStoreError::CreateDir(_)
            )
        ));
        assert_private_category(
            map_build_error(error),
            MatrixIpcErrorCategory::StoreUnavailable,
        );
    }

    #[tokio::test]
    async fn actual_sdk_locked_database_remains_non_recovery() {
        let fixture = StoreFixture::new();
        let state = fixture.0.join("state");
        std::fs::create_dir_all(&state).unwrap();
        let path = state.join(matrix_sdk_sqlite::STATE_STORE_DATABASE_NAME);
        let lock = rusqlite::Connection::open(path).unwrap();
        lock.execute_batch("CREATE TABLE fixture(value INTEGER); BEGIN EXCLUSIVE;")
            .unwrap();
        let error = Client::builder()
            .homeserver_url("https://example.org")
            .sqlite_store_with_cache_path(&state, fixture.0.join("cache"), None)
            .build()
            .await
            .unwrap_err();
        assert_private_category(map_build_error(error), MatrixIpcErrorCategory::StoreLocked);
        lock.execute_batch("ROLLBACK;").unwrap();
    }

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
    fn product_encryption_settings_only_override_backup_download() {
        let settings = EncryptionSettings {
            backup_download_strategy: BackupDownloadStrategy::AfterDecryptionFailure,
            ..EncryptionSettings::default()
        };
        assert_eq!(
            settings.backup_download_strategy,
            BackupDownloadStrategy::AfterDecryptionFailure
        );
        let source = include_str!("open.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or("");
        assert!(production
            .contains("backup_download_strategy: BackupDownloadStrategy::AfterDecryptionFailure"));
        assert!(!settings.auto_enable_cross_signing);
        assert!(!settings.auto_enable_backups);
    }

    #[test]
    fn room_keys_go_to_all_devices_explicitly() {
        assert_eq!(ROOM_KEY_RECIPIENT_STRATEGY, CollectStrategy::AllDevices);
        assert_eq!(CollectStrategy::default(), CollectStrategy::AllDevices);
        let source = include_str!("open.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or("");
        assert!(
            production.contains(".with_room_key_recipient_strategy(ROOM_KEY_RECIPIENT_STRATEGY)")
        );
    }

    #[test]
    fn product_search_index_never_uses_unencrypted_directory() {
        let source = include_str!("open.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production client open");
        assert!(production.contains("EncryptedDirectory"));
        assert!(
            !production.contains("UnencryptedDirectory"),
            "product client open must not name the plaintext on-disk index kind"
        );
        assert!(!production.contains("SearchIndexStoreKind::InMemory"));
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

    #[test]
    fn product_builder_enables_automatic_back_pagination() {
        let source = include_str!("open.rs");
        assert!(source.contains("with_enable_automatic_back_pagination(true)"));
        assert!(!source.contains(&format!("{}{}", "experimental", "-")));
    }
}
