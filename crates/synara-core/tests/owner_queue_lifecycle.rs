//! SharedCore polling queue teardown and reuse across retained SDK sessions.
//! Moved from desktop lifecycle tests: this proof has no desktop-shell owner.

use std::fs;
use std::path::PathBuf;
use synara_core::app::store::AccountIdentity;

fn temp_root(label: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "synara-p2.6-{}-{}-{}",
        label,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp root");
    fs::canonicalize(&dir).unwrap_or(dir)
}

fn r0_7_identity() -> AccountIdentity {
    let hs = std::env::var("SYNARA_MATRIX_HOMESERVER_URL")
        .ok()
        .filter(|u| {
            std::env::var("SYNARA_RUN_MATRIX_RUST_AUTH_LIVE")
                .ok()
                .as_deref()
                == Some("1")
                && {
                    let Ok(parsed) = url::Url::parse(u) else {
                        return false;
                    };
                    parsed.scheme() == "http"
                        && matches!(
                            parsed.host_str(),
                            Some("127.0.0.1") | Some("localhost") | Some("::1")
                        )
                        && parsed.username().is_empty()
                        && parsed.password().is_none()
                }
        })
        .unwrap_or_else(|| "http://127.0.0.1:8008".to_owned());
    AccountIdentity::new("@r07-lifecycle:localhost", &hs).unwrap()
}

/// Exercise SharedCore queue clearing over a retained SDK session. This
/// regression covers teardown and reuse of the polling queue; it does not
/// attach owners or claim session-generation advancement/rejection.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn r0_7_owner_queue_teardown_after_real_sdk_logout() {
    use std::collections::HashMap;
    use std::sync::Mutex;
    use synara_core::{IosSecretVault, IosSecretVaultError, SharedCore};
    #[derive(Default)]
    struct MemoryCallbackVault(Mutex<HashMap<String, Vec<u8>>>);
    impl IosSecretVault for MemoryCallbackVault {
        fn get(&self, key: String) -> Result<Option<Vec<u8>>, IosSecretVaultError> {
            Ok(self.0.lock().unwrap().get(&key).cloned())
        }
        fn put(&self, key: String, value: Vec<u8>) -> Result<(), IosSecretVaultError> {
            self.0.lock().unwrap().insert(key, value);
            Ok(())
        }
        fn delete(&self, key: String) -> Result<(), IosSecretVaultError> {
            self.0.lock().unwrap().remove(&key);
            Ok(())
        }
    }
    let root = temp_root("r07-native-stale-owner");
    let identity = r0_7_identity();
    let core = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault::default()));
    core.persist_planted_session_for_test(
        identity.user_id().into(),
        identity.homeserver_url().into(),
        root.to_string_lossy().into_owned(),
        "DEVICE".into(),
        "fixture-token".into(),
        None,
    )
    .await
    .unwrap();
    let generation = core
        .session_snapshot()
        .await
        .unwrap()
        .session_generation
        .unwrap();
    core.enqueue_owner_update_for_test("typing".into(), generation, None);
    assert_eq!(core.poll_owner_updates().await.unwrap().len(), 1);
    core.enqueue_owner_update_for_test("typing".into(), generation, None);
    core.logout().await.unwrap();
    assert!(core.poll_owner_updates().await.unwrap().is_empty());
    let logged_out = core.session_snapshot().await.unwrap();
    assert_eq!(logged_out.status, "logged_out");
    assert!(logged_out.session_generation.is_none());
    core.restore_persisted_session(
        identity.user_id().into(),
        identity.homeserver_url().into(),
        root.to_string_lossy().into_owned(),
    )
    .await
    .unwrap();
    let restored = core
        .session_snapshot()
        .await
        .unwrap()
        .session_generation
        .unwrap();
    core.enqueue_owner_update_for_test("typing".into(), restored, None);
    assert_eq!(core.poll_owner_updates().await.unwrap().len(), 1);
    core.logout().await.unwrap();
    drop(core);
    let _ = fs::remove_dir_all(root);
}
