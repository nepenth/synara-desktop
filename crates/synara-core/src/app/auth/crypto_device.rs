//! Peek the leftover SQLite crypto account before password login.
//!
//! Logout does not wipe the per-account crypto store. The next password login
//! must reuse that account's device id so `OlmMachine::with_store` does not
//! see `MismatchedAccount`. This helper is read-only: it never logs, wipes, or
//! returns account identifiers other than the device id needed for login.

use std::path::Path;

use matrix_sdk_crypto::store::CryptoStore;
use matrix_sdk_sqlite::SqliteCryptoStore;

use super::error::AuthError;

const CRYPTO_DB_NAME: &str = "matrix-sdk-crypto.sqlite3";

/// Return the leftover crypto device id for this account store, if any.
///
/// - Missing crypto database → `Ok(None)` (fresh login).
/// - Existing unreadable/unpicklable database → `p3.2-login-store-reset-required`.
/// - Open/pool failures (including a lock held by another instance) → `None`
///   so the subsequent client build/login can classify lock vs open failure.
pub async fn existing_sqlite_crypto_device_id(
    state_dir: &Path,
    passphrase: Option<&str>,
) -> Result<Option<String>, AuthError> {
    let db_path = state_dir.join(CRYPTO_DB_NAME);
    if !db_path.is_file() {
        return Ok(None);
    }

    let store = match SqliteCryptoStore::open(state_dir, passphrase).await {
        Ok(store) => store,
        Err(_) => return Ok(None),
    };

    let account = match CryptoStore::load_account(&store).await {
        Ok(account) => account,
        Err(_) => {
            return Err(AuthError::SdkInvariant {
                diagnostic_id: "p3.2-login-store-reset-required",
            });
        }
    };

    Ok(account.map(|account| account.device_id().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_state_dir() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("synara-crypto-device-{nanos}"));
        std::fs::create_dir_all(&dir).expect("temp state dir");
        dir
    }

    #[tokio::test]
    async fn missing_crypto_db_is_fresh_login() {
        let dir = temp_state_dir();
        let result = existing_sqlite_crypto_device_id(&dir, None)
            .await
            .expect("peek");
        assert_eq!(result, None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn leftover_crypto_account_returns_device_id() {
        let dir = temp_state_dir();
        let passphrase = "test-passphrase-not-a-secret";
        let user = ruma::UserId::parse("@alice:example.org").expect("mxid");
        let device: &ruma::DeviceId = "LEFTOVERDEV".into();
        let store = SqliteCryptoStore::open(&dir, Some(passphrase))
            .await
            .expect("open leftover store");
        let machine = matrix_sdk_crypto::OlmMachine::with_store(&user, device, store, None)
            .await
            .expect("create leftover olm account");
        drop(machine);

        let peeked = existing_sqlite_crypto_device_id(&dir, Some(passphrase))
            .await
            .expect("peek leftover");
        assert_eq!(peeked.as_deref(), Some("LEFTOVERDEV"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod live_tests {
    use super::*;
    use crate::app::{
        auth::{login_with_password, LoginOptions},
        client_builder::{build_unauthenticated_client, ClientBuildConfig},
        store::{AccountIdentity, StoreKeyMaterial},
        sync::{build_sync_service, SyncServiceConfig},
    };
    use matrix_sdk::{ruma::api::client::keys::get_keys, Client};
    use std::time::Duration;

    async fn published(client: &Client) -> Result<bool, &'static str> {
        let mut request = get_keys::v3::Request::new();
        let user = client.user_id().ok_or("missing user")?;
        let device = client.device_id().ok_or("missing device")?;
        request
            .device_keys
            .insert(user.to_owned(), vec![device.to_owned()]);
        let response = client.send(request).await.map_err(|_| "query failed")?;
        if !response.failures.is_empty() {
            return Err("query incomplete");
        }
        Ok(response
            .device_keys
            .get(user)
            .is_some_and(|keys| keys.contains_key(device)))
    }

    async fn wait_for_publication(client: &Client) -> Result<(), &'static str> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
        loop {
            if published(client).await? {
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                return Err("device keys absent from homeserver");
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    /// Reproduce the actual retained-store route, including real server logout.
    /// The fixture touches only its newly created test-account device and always
    /// attempts revocation before asserting the result. No trust is bootstrapped.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[ignore = "requires authorized dedicated SYNARA_LIVE_* test credentials"]
    async fn live_retained_store_verification_publishes_device_keys() {
        let homeserver = std::env::var("SYNARA_LIVE_HOMESERVER").expect("test homeserver required");
        let username = std::env::var("SYNARA_LIVE_USERNAME").expect("test username required");
        let password = std::env::var("SYNARA_LIVE_PASSWORD").expect("test password required");
        let user = if username.starts_with('@') {
            username
        } else {
            format!(
                "@{}:{}",
                username,
                url::Url::parse(&homeserver).unwrap().host_str().unwrap()
            )
        };
        assert!(
            !user.starts_with("@chris:"),
            "dedicated test account required"
        );
        let identity = AccountIdentity::new(&user, &homeserver).expect("test identity");
        let mut key = [0_u8; 32];
        getrandom::fill(&mut key).expect("store key entropy");
        let mut nonce = [0_u8; 8];
        getrandom::fill(&mut nonce).expect("path entropy");
        let root = std::env::temp_dir().join(format!(
            "synara-relogin-proof-{:016x}",
            u64::from_be_bytes(nonce)
        ));
        std::fs::create_dir_all(&root).expect("proof directory");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let mut current: Option<Client> = None;
        let mut sync = None;
        let result: Result<(), &'static str> = async {
            let config = ClientBuildConfig::product_default(
                &root,
                identity.clone(),
                Some(StoreKeyMaterial::from_bytes(key)),
            )
            .map_err(|_| "config failed")?;
            current = Some(
                build_unauthenticated_client(&config)
                    .await
                    .map_err(|_| "client failed")?,
            );
            let client = current.as_ref().unwrap();
            login_with_password(client, &user, &password, &LoginOptions::default())
                .await
                .map_err(|_| "fresh login failed")?;
            let device = client.device_id().ok_or("fresh device missing")?.to_owned();
            sync = Some(
                build_sync_service(client, 1, SyncServiceConfig::default())
                    .await
                    .map_err(|_| "fresh sync build failed")?,
            );
            sync.as_ref()
                .unwrap()
                .start()
                .await
                .map_err(|_| "fresh sync failed")?;
            wait_for_publication(client).await?;
            let original_keys = client
                .encryption()
                .get_own_device()
                .await
                .map_err(|_| "fresh keys read failed")?
                .ok_or("fresh keys missing")?
                .as_device_keys()
                .keys
                .clone();
            eprintln!("synara_relogin_proof checkpoint=fresh_keys_published");
            sync.take()
                .unwrap()
                .stop()
                .await
                .map_err(|_| "fresh stop failed")?;
            client
                .matrix_auth()
                .logout()
                .await
                .map_err(|_| "fresh logout failed")?;
            current.take();

            let existing = existing_sqlite_crypto_device_id(
                config.state_store_path(),
                config.store_passphrase_hex().as_deref(),
            )
            .await
            .map_err(|_| "retained device read failed")?
            .ok_or("retained device missing")?;
            if existing != device.as_str() {
                return Err("retained device changed");
            }
            current = Some(
                build_unauthenticated_client(&config)
                    .await
                    .map_err(|_| "relogin client failed")?,
            );
            let client = current.as_ref().unwrap();
            login_with_password(
                client,
                &user,
                &password,
                &LoginOptions {
                    device_id: Some(existing),
                    ..Default::default()
                },
            )
            .await
            .map_err(|_| "retained login failed")?;
            let retained_keys = client
                .encryption()
                .get_own_device()
                .await
                .map_err(|_| "retained keys read failed")?
                .ok_or("retained keys missing")?
                .as_device_keys()
                .keys
                .clone();
            if retained_keys != original_keys {
                return Err("retained identity keys changed");
            }
            sync = Some(
                build_sync_service(client, 2, SyncServiceConfig::default())
                    .await
                    .map_err(|_| "relogin sync build failed")?,
            );
            sync.as_ref()
                .unwrap()
                .start()
                .await
                .map_err(|_| "relogin sync failed")?;
            let owner = crate::app::verification::NativeVerificationOwner::new(client, 2);
            let request = owner
                .start(None)
                .await
                .map_err(|_| "retained verification start failed")?;
            // Read immediately after the product start returns. The action must
            // publish before it sends a request; later sync cannot mask a gap.
            let present = published(client).await?;
            owner
                .cancel(&request.flow_id)
                .await
                .map_err(|_| "proof request cancellation failed")?;
            if !present {
                return Err("verification sent without published keys");
            }
            wait_for_publication(client).await?;
            eprintln!("synara_relogin_proof checkpoint=retained_keys_published");
            Ok(())
        }
        .await;
        if let Some(sync) = sync {
            let _ = sync.stop().await;
        }
        let revoked = if let Some(client) = current.as_ref().filter(|c| c.session().is_some()) {
            client.matrix_auth().logout().await.is_ok()
        } else {
            true
        };
        drop(current);
        if revoked {
            std::fs::remove_dir_all(&root).expect("remove disposable proof store");
        }
        assert!(revoked, "disposable device revocation failed");
        assert_eq!(result, Ok(()));
    }
}
