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
