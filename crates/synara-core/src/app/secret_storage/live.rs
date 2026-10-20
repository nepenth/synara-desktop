//! Privacy-safe live Matrix secret-storage product projection.

use zeroize::Zeroizing;

use matrix_sdk::{
    encryption::recovery::{RecoveryError, RecoveryState},
    ruma::{
        events::{
            secret::request::SecretName,
            secret_storage::{
                default_key::SecretStorageDefaultKeyEventContent,
                key::SecretStorageKeyEventContent, secret::SecretEventContent,
            },
            AnyGlobalAccountDataEventContent, EventContentFromType, GlobalAccountDataEventType,
        },
        serde::Raw,
    },
    Client,
};

use super::{
    operation_result, project_secret_storage_status, NativeMissingSecret, NativeRecoveryPhase,
    NativeSecretStorageOperationResult, NativeSecretStorageOutcome, NativeSecretStorageStatus,
};

/// Explicit one-time display result. No Debug, Clone or serialization implementation.
/// If a status/postcondition fails, the generated secret is wiped when dropped.
pub struct SecretStorageSetup {
    pub result: NativeSecretStorageOperationResult,
    pub recovery_key: Option<Zeroizing<String>>,
}

/// Where a status read takes account data from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AccountDataSource {
    /// The synced local store. The homeserver is asked only when the SDK's
    /// recovery state says a default key exists (or is still unknown) but sync
    /// has not delivered it yet. Screens and device/security wakeups use this,
    /// so a status read never sends requests that re-arm those wakeups.
    LocalFirst,
    /// Always the homeserver. `set_account_data` does not update the local
    /// store, so mutation pre- and postconditions cannot wait for a sync echo.
    Homeserver,
}

/// Secret-storage status from synced account data, with a homeserver fallback
/// only until sync delivers the default key.
pub async fn status(
    client: &Client,
    session_generation: u64,
) -> Result<NativeSecretStorageStatus, &'static str> {
    status_from(client, session_generation, AccountDataSource::LocalFirst).await
}

/// Homeserver readback for setup, unlock and reset completion checks.
async fn authoritative_status(
    client: &Client,
    session_generation: u64,
) -> Result<NativeSecretStorageStatus, &'static str> {
    status_from(client, session_generation, AccountDataSource::Homeserver).await
}

async fn status_from(
    client: &Client,
    session_generation: u64,
    source: AccountDataSource,
) -> Result<NativeSecretStorageStatus, &'static str> {
    let recovery_state = client.encryption().recovery().state();
    let local_default_key = match source {
        AccountDataSource::LocalFirst => client
            .account()
            .account_data::<SecretStorageDefaultKeyEventContent>()
            .await
            .map_err(|_| "v-crypto.4-status-default-key-failed")?,
        AccountDataSource::Homeserver => None,
    };
    // Once the default key is in the local store, sync has delivered the
    // account data, so the key description and secrets are read locally too.
    let synced = local_default_key.is_some();
    let default_key = match local_default_key {
        Some(raw) => Some(raw),
        None if fetch_default_key(source, recovery_state) => client
            .encryption()
            .secret_storage()
            .fetch_default_key_id()
            .await
            .map_err(|_| "v-crypto.4-status-default-key-failed")?,
        None => None,
    };
    let default_key = default_key
        .map(|raw| {
            raw.deserialize()
                .map_err(|_| "v-crypto.4-status-default-key-failed")
        })
        .transpose()?;
    let default_key_set = default_key.is_some();
    let default_key_id = default_key.as_ref().map(|content| content.key_id.as_str());
    let (exists, passphrase_configured) = match default_key.as_ref() {
        Some(default_key) => {
            let event_type =
                GlobalAccountDataEventType::SecretStorageKey(default_key.key_id.to_owned());
            let key = read_account_data(client, event_type.to_owned(), synced)
                .await
                .map_err(|_| "v-crypto.4-status-key-info-failed")?;
            let key = key
                .map(|raw| {
                    let event_type = event_type.to_string();
                    let value = serde_json::value::to_raw_value(&raw)
                        .map_err(|_| "v-crypto.4-status-key-info-failed")?;
                    SecretStorageKeyEventContent::from_parts(&event_type, &value)
                        .map_err(|_| "v-crypto.4-status-key-info-failed")
                })
                .transpose()?;
            (
                key.is_some(),
                key.as_ref()
                    .is_some_and(|content| content.passphrase.is_some()),
            )
        }
        None => (false, false),
    };

    let missing_secrets = missing_secrets(client, default_key_id, synced).await?;
    let bootstrap_ready = client
        .encryption()
        .cross_signing_status()
        .await
        .is_some_and(|status| status.is_complete());
    Ok(project_status(
        session_generation,
        recovery_state,
        exists,
        default_key_set,
        passphrase_configured,
        bootstrap_ready,
        missing_secrets,
    ))
}

/// A missing local default key is authoritative only once the SDK has found
/// recovery disabled. `Unknown` has not been checked yet, and `Enabled` or
/// `Incomplete` mean the key exists but has not synced to this store.
fn fetch_default_key(source: AccountDataSource, recovery_state: RecoveryState) -> bool {
    source == AccountDataSource::Homeserver || recovery_state != RecoveryState::Disabled
}

/// Synced account data comes from the local store; otherwise ask the
/// homeserver, as the default key did.
async fn read_account_data(
    client: &Client,
    event_type: GlobalAccountDataEventType,
    synced: bool,
) -> matrix_sdk::Result<Option<Raw<AnyGlobalAccountDataEventContent>>> {
    if synced {
        client.account().account_data_raw(event_type).await
    } else {
        client.account().fetch_account_data(event_type).await
    }
}

pub async fn bootstrap(
    client: &Client,
    session_generation: u64,
    passphrase: &str,
) -> Result<SecretStorageSetup, &'static str> {
    if passphrase.trim().is_empty() || passphrase.len() > 100_000 {
        return Err("recovery-secret-invalid");
    }
    let before = authoritative_status(client, session_generation).await?;
    if before.exists || before.default_key_set {
        // Existing but incomplete state is not enrollment success. Never enable
        // again automatically after a partial prior setup or lost display key.
        require_setup_complete(&before)?;
        return Ok(SecretStorageSetup {
            result: operation_result(NativeSecretStorageOutcome::AlreadyConfigured, false, before),
            recovery_key: None,
        });
    }
    if !before.bootstrap_ready {
        return Err("v-crypto.4-bootstrap-cross-signing-required");
    }

    finish_secret_setup(
        async {
            enable_recovery(client, passphrase)
                .await
                .map_err(map_bootstrap_error)
        },
        authoritative_status(client, session_generation),
    )
    .await
}

/// The one enrolment path for secret storage and key backup.
///
/// `Recovery::enable` creates the default secret-storage key, stores the
/// cross-signing and backup secrets in it, and enables key backup. Both
/// `bootstrap` and `backup::setup` call this, so there is a single place that
/// generates a recovery key; callers that can display it must do so.
pub(crate) async fn enable_recovery(
    client: &Client,
    passphrase: &str,
) -> Result<Zeroizing<String>, RecoveryError> {
    let recovery_key = Zeroizing::new(
        client
            .encryption()
            .recovery()
            .enable()
            .with_passphrase(passphrase)
            .wait_for_backups_to_upload()
            .await?,
    );
    let _ = crate::app::dehydrated_devices::start_with_secret(client, &recovery_key).await;
    Ok(recovery_key)
}

pub async fn unlock(
    client: &Client,
    session_generation: u64,
    recovery_secret: &str,
) -> Result<NativeSecretStorageOperationResult, &'static str> {
    if recovery_secret.trim().is_empty() || recovery_secret.len() > 100_000 {
        return Err("recovery-secret-invalid");
    }
    finish_secret_unlock(
        async {
            client
                .encryption()
                .recovery()
                .recover(recovery_secret)
                .await
                .map_err(|_| "v-crypto.4-unlock-rejected")?;
            let _ =
                crate::app::dehydrated_devices::start_with_secret(client, recovery_secret).await;
            Ok(())
        },
        authoritative_status(client, session_generation),
    )
    .await
}

pub async fn reset(
    client: &Client,
    session_generation: u64,
    passphrase: &str,
) -> Result<SecretStorageSetup, &'static str> {
    if passphrase.trim().is_empty() || passphrase.len() > 100_000 {
        return Err("recovery-secret-invalid");
    }
    let before = authoritative_status(client, session_generation).await?;
    if !before.unlocked {
        return Err("v-crypto.4-reset-requires-unlock");
    }

    finish_secret_setup(
        async {
            let recovery_key = Zeroizing::new(
                client
                    .encryption()
                    .recovery()
                    .reset_key()
                    .with_passphrase(passphrase)
                    .await
                    .map_err(|_| "v-crypto.4-reset-failed")?,
            );
            let _ = crate::app::dehydrated_devices::start_with_secret(client, &recovery_key).await;

            Ok(recovery_key)
        },
        authoritative_status(client, session_generation),
    )
    .await
}

fn recovery_phase(state: RecoveryState) -> NativeRecoveryPhase {
    match state {
        RecoveryState::Unknown => NativeRecoveryPhase::Unknown,
        RecoveryState::Disabled => NativeRecoveryPhase::Disabled,
        RecoveryState::Incomplete => NativeRecoveryPhase::Incomplete,
        RecoveryState::Enabled => NativeRecoveryPhase::Enabled,
    }
}

async fn missing_secrets(
    client: &Client,
    default_key_id: Option<&str>,
    synced: bool,
) -> Result<Vec<NativeMissingSecret>, &'static str> {
    let known = [
        (
            SecretName::CrossSigningMasterKey,
            NativeMissingSecret::CrossSigningMaster,
        ),
        (
            SecretName::CrossSigningSelfSigningKey,
            NativeMissingSecret::CrossSigningSelfSigning,
        ),
        (
            SecretName::CrossSigningUserSigningKey,
            NativeMissingSecret::CrossSigningUserSigning,
        ),
        (
            SecretName::RecoveryKey,
            NativeMissingSecret::EncryptionBackup,
        ),
    ];
    // A secret counts only when it is encrypted for the default key, so with
    // no default key every secret is missing and there is nothing to read.
    let Some(default_key_id) = default_key_id else {
        return Ok(known
            .into_iter()
            .map(|(_, projection)| projection)
            .collect());
    };
    let mut missing = Vec::new();
    for (name, projection) in known {
        let event_type = GlobalAccountDataEventType::from(name);
        let content = read_account_data(client, event_type, synced)
            .await
            .map_err(|_| "v-crypto.4-status-secret-check-failed")?;
        let present = content
            .map(|raw| {
                raw.deserialize_as_unchecked::<SecretEventContent>()
                    .map_err(|_| "v-crypto.4-status-secret-check-failed")
            })
            .transpose()?
            .is_some_and(|content| content.encrypted.contains_key(default_key_id));
        if !present {
            missing.push(projection);
        }
    }
    Ok(missing)
}

fn project_status(
    session_generation: u64,
    recovery_state: RecoveryState,
    exists: bool,
    default_key_set: bool,
    passphrase_configured: bool,
    bootstrap_ready: bool,
    missing_secrets: Vec<NativeMissingSecret>,
) -> NativeSecretStorageStatus {
    project_secret_storage_status(
        session_generation,
        recovery_phase(recovery_state),
        exists,
        default_key_set,
        passphrase_configured,
        bootstrap_ready,
        missing_secrets,
    )
}

fn map_bootstrap_error(error: RecoveryError) -> &'static str {
    match error {
        RecoveryError::BackupExistsOnServer => "v-crypto.4-bootstrap-existing-backup",
        _ => "v-crypto.4-bootstrap-failed",
    }
}

/// A generated key stays zeroizing until the same authoritative completion
/// check used by unlock succeeds. Failed readbacks never release a display key.
async fn finish_secret_setup(
    operation: impl std::future::Future<Output = Result<Zeroizing<String>, &'static str>>,
    readback: impl std::future::Future<Output = Result<NativeSecretStorageStatus, &'static str>>,
) -> Result<SecretStorageSetup, &'static str> {
    let recovery_key = operation.await?;
    let status = readback.await?;
    require_setup_complete(&status)?;
    Ok(SecretStorageSetup {
        result: operation_result(NativeSecretStorageOutcome::Complete, false, status),
        recovery_key: Some(recovery_key),
    })
}

async fn finish_secret_unlock(
    operation: impl std::future::Future<Output = Result<(), &'static str>>,
    readback: impl std::future::Future<Output = Result<NativeSecretStorageStatus, &'static str>>,
) -> Result<NativeSecretStorageOperationResult, &'static str> {
    operation.await?;
    let status = readback.await?;
    require_complete(&status)?;
    Ok(operation_result(
        NativeSecretStorageOutcome::Complete,
        false,
        status,
    ))
}

fn require_setup_complete(status: &NativeSecretStorageStatus) -> Result<(), &'static str> {
    require_complete(status)?;
    if !status.passphrase_configured {
        return Err("v-crypto.4-passphrase-not-configured");
    }
    Ok(())
}

fn require_complete(status: &NativeSecretStorageStatus) -> Result<(), &'static str> {
    if !status.exists
        || !status.default_key_set
        || !status.unlocked
        || !status.bootstrap_ready
        || !status.missing_secrets.is_empty()
    {
        return Err("v-crypto.4-operation-incomplete");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_setup_and_bootstrap_share_one_recovery_enrolment() {
        let secret_storage = include_str!("live.rs");
        let backup = include_str!("../backup/live.rs");
        let enable_call = concat!(".recovery()\n", "            .enable()");
        assert_eq!(secret_storage.matches(enable_call).count(), 1);
        assert!(!backup.contains(concat!(".enable", "()")));
        assert!(backup.contains("crate::app::secret_storage::enable_recovery(client, passphrase)"));
        let bootstrap = secret_storage
            .split("pub async fn bootstrap(")
            .nth(1)
            .and_then(|rest| rest.split("pub(crate) async fn enable_recovery").next())
            .expect("bootstrap body");
        assert!(bootstrap.contains("enable_recovery(client, passphrase)"));
    }

    #[tokio::test]
    async fn setup_requires_passphrase_readback_while_key_only_unlock_remains_valid() {
        let key_only = project_secret_storage_status(
            8,
            NativeRecoveryPhase::Enabled,
            true,
            true,
            false,
            true,
            vec![],
        );
        let result = finish_secret_setup(
            async { Ok(Zeroizing::new("test-generated-key".into())) },
            async { Ok(key_only.clone()) },
        )
        .await;
        match result {
            Err(error) => assert_eq!(error, "v-crypto.4-passphrase-not-configured"),
            Ok(_) => panic!("requested passphrase enrollment must be confirmed before key display"),
        }
        let result = finish_secret_unlock(async { Ok(()) }, async { Ok(key_only) })
            .await
            .unwrap();
        assert_eq!(result.outcome, NativeSecretStorageOutcome::Complete);
        let mut incomplete = result.status;
        incomplete.unlocked = false;
        assert_eq!(
            require_setup_complete(&incomplete),
            Err("v-crypto.4-operation-incomplete")
        );
    }

    #[tokio::test]
    async fn existing_incomplete_sdk_secret_storage_is_not_success_and_is_not_enabled_again() {
        use matrix_sdk::{config::RequestConfig, test_utils::mocks::MatrixMockServer};
        use wiremock::{
            matchers::{method, path_regex},
            Mock, ResponseTemplate,
        };
        let server = MatrixMockServer::new().await;
        let client = server
            .client_builder()
            .on_builder(|builder| builder.request_config(RequestConfig::new().disable_retry()))
            .build()
            .await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/_matrix/client/.*/user/.*/account_data/.*$"))
            .respond_with(
                ResponseTemplate::new(404).set_body_json(
                    serde_json::json!({"errcode":"M_NOT_FOUND","error":"not stored"}),
                ),
            )
            .with_priority(10)
            .mount(server.server())
            .await;
        Mock::given(method("GET"))
            .and(path_regex(
                r"^/_matrix/client/.*/user/.*/account_data/m.secret_storage.default_key$",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"key":"test-key"})),
            )
            .with_priority(1)
            .mount(server.server())
            .await;
        Mock::given(method("GET"))
            .and(path_regex(
                r"^/_matrix/client/.*/user/.*/account_data/m.secret_storage.key.test-key$",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"algorithm":"m.secret_storage.v1.aes-hmac-sha2"}),
            ))
            .with_priority(1)
            .mount(server.server())
            .await;
        let before = server.server().received_requests().await.unwrap().len();
        match bootstrap(&client, 8, "test-passphrase").await {
            Err(error) => assert_eq!(error, "v-crypto.4-operation-incomplete"),
            Ok(_) => panic!("an existing but locked/incomplete key is not enrollment success"),
        }
        let requests = server.server().received_requests().await.unwrap();
        assert!(
            requests[before..]
                .iter()
                .all(|request| request.method.as_str() == "GET"),
            "existing incomplete state must not trigger another enable/write"
        );
    }

    #[tokio::test]
    async fn successful_setup_reset_and_unlock_mutations_reject_incomplete_readback() {
        use std::cell::Cell;
        let mutated = Cell::new(false);
        let incomplete = project_secret_storage_status(
            8,
            NativeRecoveryPhase::Enabled,
            true,
            true,
            true,
            true,
            vec![NativeMissingSecret::EncryptionBackup],
        );
        // Bootstrap and reset both use this production key-display completion route.
        let result = finish_secret_setup(
            async {
                mutated.set(true);
                Ok(Zeroizing::new("test-generated-key".to_owned()))
            },
            async {
                assert!(mutated.get());
                Ok(incomplete.clone())
            },
        )
        .await;
        match result {
            Err(error) => assert_eq!(error, "v-crypto.4-operation-incomplete"),
            Ok(_) => panic!("partial success must not release a display key"),
        }
        mutated.set(false);
        let result = finish_secret_unlock(
            async {
                mutated.set(true);
                Ok(())
            },
            async {
                assert!(mutated.get());
                Ok(incomplete)
            },
        )
        .await;
        assert_eq!(result.unwrap_err(), "v-crypto.4-operation-incomplete");
    }

    #[tokio::test]
    async fn successful_key_mutation_followed_by_malformed_sdk_account_data_releases_no_key() {
        use matrix_sdk::{config::RequestConfig, test_utils::mocks::MatrixMockServer};
        use wiremock::{
            matchers::{method, path_regex},
            Mock, ResponseTemplate,
        };
        let server = MatrixMockServer::new().await;
        let client = server
            .client_builder()
            .on_builder(|builder| builder.request_config(RequestConfig::new().disable_retry()))
            .build()
            .await;
        Mock::given(method("GET"))
            .and(path_regex(
                r"^/_matrix/client/.*/user/.*/account_data/m.secret_storage.default_key$",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"key":42})))
            .mount(server.server())
            .await;
        // Inject only the already-successful SDK mutation; readback is the real
        // SDK HTTP/account-data adapter and the production completion function.
        let mutated = std::cell::Cell::new(false);
        let result = finish_secret_setup(
            async {
                mutated.set(true);
                Ok(Zeroizing::new("test-generated-key".to_owned()))
            },
            authoritative_status(&client, 8),
        )
        .await;
        assert!(mutated.get());
        match result {
            Err(error) => assert_eq!(error, "v-crypto.4-status-default-key-failed"),
            Ok(_) => panic!("malformed authoritative readback must not return a recovery key"),
        }
    }

    #[tokio::test]
    async fn secret_storage_sdk_query_failure_is_static_and_no_key_is_displayed() {
        use matrix_sdk::{config::RequestConfig, test_utils::mocks::MatrixMockServer};
        use wiremock::{
            matchers::{method, path_regex},
            Mock, ResponseTemplate,
        };
        let server = MatrixMockServer::new().await;
        let client = server
            .client_builder()
            .on_builder(|builder| builder.request_config(RequestConfig::new().disable_retry()))
            .build()
            .await;
        Mock::given(method("GET"))
            .and(path_regex(
                r"^/_matrix/client/.*/user/.*/account_data/m.secret_storage.default_key$",
            ))
            .respond_with(ResponseTemplate::new(503).set_body_json(
                serde_json::json!({"errcode":"M_UNKNOWN","error":"private key material"}),
            ))
            .mount(server.server())
            .await;
        assert_eq!(
            status(&client, 8).await.unwrap_err(),
            "v-crypto.4-status-default-key-failed"
        );
        match bootstrap(&client, 8, "test-passphrase").await {
            Err(diagnostic) => assert_eq!(diagnostic, "v-crypto.4-status-default-key-failed"),
            Ok(_) => panic!("SDK failure must not return a key"),
        }
    }

    #[tokio::test]
    async fn status_reads_synced_account_data_without_homeserver_requests() {
        use matrix_sdk::{config::RequestConfig, test_utils::mocks::MatrixMockServer};
        use wiremock::{
            matchers::{method, path_regex},
            Mock, ResponseTemplate,
        };
        let server = MatrixMockServer::new().await;
        let client = server
            .client_builder()
            .on_builder(|builder| builder.request_config(RequestConfig::new().disable_retry()))
            .build()
            .await;
        server
            .mock_sync()
            .ok_and_run(&client, |builder| {
                builder
                    .add_custom_global_account_data(serde_json::json!({
                        "type": "m.secret_storage.default_key",
                        "content": {"key": "test-key"},
                    }))
                    .add_custom_global_account_data(serde_json::json!({
                        "type": "m.secret_storage.key.test-key",
                        "content": {
                            "algorithm": "m.secret_storage.v1.aes-hmac-sha2",
                            "passphrase": {
                                "algorithm": "m.pbkdf2",
                                "salt": "test-salt",
                                "iterations": 10,
                            },
                        },
                    }));
            })
            .await;
        // The SDK's own recovery setup and its default-key sync handler fetch
        // the default key once each; let them finish before counting.
        client
            .encryption()
            .wait_for_e2ee_initialization_tasks()
            .await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/_matrix/client/.*/user/.*/account_data/.*$"))
            .respond_with(ResponseTemplate::new(500))
            .mount(server.server())
            .await;
        let account_data_reads = || async {
            server
                .server()
                .received_requests()
                .await
                .unwrap()
                .iter()
                .filter(|request| request.url.path().contains("/account_data/"))
                .count()
        };
        let before = account_data_reads().await;

        for _ in 0..3 {
            let status = status(&client, 8).await.expect("synced status");
            assert!(status.default_key_set);
            assert!(status.exists);
            assert!(status.passphrase_configured);
            assert_eq!(status.missing_secrets.len(), 4);
        }
        assert_eq!(
            account_data_reads().await,
            before,
            "a status read after sync must not ask the homeserver again"
        );
        // The mutation readback still asks, and would see the failure.
        assert!(authoritative_status(&client, 8).await.is_err());
        assert!(account_data_reads().await > before);
    }

    #[test]
    fn missing_local_default_key_is_fetched_unless_recovery_is_known_disabled() {
        assert!(!fetch_default_key(
            AccountDataSource::LocalFirst,
            RecoveryState::Disabled
        ));
        for state in [
            RecoveryState::Unknown,
            RecoveryState::Enabled,
            RecoveryState::Incomplete,
        ] {
            assert!(fetch_default_key(AccountDataSource::LocalFirst, state));
        }
        assert!(fetch_default_key(
            AccountDataSource::Homeserver,
            RecoveryState::Disabled
        ));
    }

    #[test]
    fn mutation_readbacks_ask_the_homeserver() {
        let source = include_str!("live.rs");
        let mutations = source
            .split("pub async fn bootstrap(")
            .nth(1)
            .and_then(|rest| rest.split("fn recovery_phase(").next())
            .expect("mutation bodies");
        assert!(!mutations.contains(" status(client, session_generation)"));
        assert_eq!(
            mutations
                .matches("authoritative_status(client, session_generation)")
                .count(),
            5
        );
    }
    #[test]
    fn completion_requires_all_authoritative_state() {
        let ready = project_secret_storage_status(
            9,
            NativeRecoveryPhase::Enabled,
            true,
            true,
            true,
            true,
            vec![],
        );
        assert_eq!(require_complete(&ready), Ok(()));
        let mut cases = vec![];
        let mut s = ready.clone();
        s.exists = false;
        cases.push(s);
        let mut s = ready.clone();
        s.default_key_set = false;
        cases.push(s);
        let mut s = ready.clone();
        s.unlocked = false;
        cases.push(s);
        let mut s = ready.clone();
        s.bootstrap_ready = false;
        cases.push(s);
        let mut s = ready;
        s.missing_secrets
            .push(NativeMissingSecret::EncryptionBackup);
        cases.push(s);
        for s in cases {
            assert_eq!(require_complete(&s), Err("v-crypto.4-operation-incomplete"));
        }
    }
}
