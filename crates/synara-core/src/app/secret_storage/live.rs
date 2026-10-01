//! Privacy-safe live Matrix secret-storage product projection.

use zeroize::Zeroizing;

use matrix_sdk::{
    encryption::recovery::{RecoveryError, RecoveryState},
    ruma::events::{
        secret::request::SecretName,
        secret_storage::{key::SecretStorageKeyEventContent, secret::SecretEventContent},
        EventContentFromType, GlobalAccountDataEventType,
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

pub async fn status(
    client: &Client,
    session_generation: u64,
) -> Result<NativeSecretStorageStatus, &'static str> {
    let secret_storage = client.encryption().secret_storage();
    let default_key = secret_storage
        .fetch_default_key_id()
        .await
        .map_err(|_| "v-crypto.4-status-default-key-failed")?;
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
            let key = client
                .account()
                .fetch_account_data(event_type.to_owned())
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

    let missing_secrets = missing_secrets(client, default_key_id).await?;
    let bootstrap_ready = client
        .encryption()
        .cross_signing_status()
        .await
        .is_some_and(|status| status.is_complete());
    Ok(project_status(
        session_generation,
        client.encryption().recovery().state(),
        exists,
        default_key_set,
        passphrase_configured,
        bootstrap_ready,
        missing_secrets,
    ))
}

pub async fn bootstrap(
    client: &Client,
    session_generation: u64,
    passphrase: &str,
) -> Result<SecretStorageSetup, &'static str> {
    if passphrase.trim().is_empty() || passphrase.len() > 100_000 {
        return Err("recovery-secret-invalid");
    }
    let before = status(client, session_generation).await?;
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
            let recovery_key = Zeroizing::new(
                client
                    .encryption()
                    .recovery()
                    .enable()
                    .with_passphrase(passphrase)
                    .wait_for_backups_to_upload()
                    .await
                    .map_err(map_bootstrap_error)?,
            );
            let _ = crate::app::dehydrated_devices::start_with_secret(client, &recovery_key).await;

            Ok(recovery_key)
        },
        status(client, session_generation),
    )
    .await
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
        status(client, session_generation),
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
    let before = status(client, session_generation).await?;
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
        status(client, session_generation),
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
    let mut missing = Vec::new();
    for (name, projection) in known {
        let event_type = GlobalAccountDataEventType::from(name);
        let content = client
            .account()
            .fetch_account_data(event_type)
            .await
            .map_err(|_| "v-crypto.4-status-secret-check-failed")?;
        let present = content
            .map(|raw| {
                raw.deserialize_as_unchecked::<SecretEventContent>()
                    .map_err(|_| "v-crypto.4-status-secret-check-failed")
            })
            .transpose()?
            .is_some_and(|content| {
                default_key_id.is_some_and(|key_id| content.encrypted.contains_key(key_id))
            });
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
            status(&client, 8),
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
