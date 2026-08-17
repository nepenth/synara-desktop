//! P4-S15: leftover status that already has a Core owner can be live.
//!
//! Planted leftover I/O that needs a live homeserver stays fail-closed.
//! This is not leftover registration, not a byte/secret envelope, and
//! not P4 acceptance. Recover stays unavailable until a later owner
//! decision.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
use synara_core::{IosSecretVault, IosSecretVaultError, SharedCore};

struct MemoryCallbackVault(Arc<Mutex<HashMap<String, Vec<u8>>>>);

impl IosSecretVault for MemoryCallbackVault {
    fn get(&self, key: String) -> Result<Option<Vec<u8>>, IosSecretVaultError> {
        Ok(self.0.lock().expect("vault").get(&key).cloned())
    }

    fn put(&self, key: String, value: Vec<u8>) -> Result<(), IosSecretVaultError> {
        self.0.lock().expect("vault").insert(key, value);
        Ok(())
    }

    fn delete(&self, key: String) -> Result<(), IosSecretVaultError> {
        self.0.lock().expect("vault").remove(&key);
        Ok(())
    }
}

fn alice() -> AccountIdentity {
    AccountIdentity::new("@alice:example.org", "https://matrix.example.org").unwrap()
}

fn temp_root(tag: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("synara-p4-s15-it-{tag}-{nanos}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

fn test_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

#[test]
fn leftover_status_without_attach_returns_owner_no_session_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let backup = rt
        .block_on(shared.backup_status())
        .expect_err("backup status needs an attached device owner");
    let room_keys = rt
        .block_on(shared.room_key_transfer_status())
        .expect_err("room-key status needs an attached device owner");
    let backup_text = format!("{backup:?}{backup}");
    let room_keys_text = format!("{room_keys:?}{room_keys}");
    assert!(backup_text.contains("p2-backup-status-no-session"));
    assert!(room_keys_text.contains("p2-room-key-transfer-status-no-session"));
    let combined = format!("{backup_text}{room_keys_text}");
    assert!(!combined.contains("password"));
    assert!(!combined.contains("syt_"));
    assert!(!combined.contains("@alice"));
    assert!(!combined.contains("https://"));
}

#[test]
fn leftover_owner_status_after_attach_is_privacy_safe_and_homeserver_io_stays_closed() {
    let access = "syt_s15_leftover_access";
    let refresh = "syr_s15_leftover_refresh";
    let recovery_key = "s15-secret-recovery-key";
    let room_id = "!s15SecretRoom:example.org";
    let event_body = "s15-secret-event-body";
    let mxc = "mxc://example.org/s15SecretMedia";
    let identity = alice();
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("owner-status");
    let rt = test_runtime();
    let _enter = rt.enter();
    rt.block_on(shared.persist_planted_session_for_test(
        identity.user_id().to_owned(),
        identity.homeserver_url().to_owned(),
        root.to_string_lossy().into_owned(),
        "DEVICEABC".to_owned(),
        access.to_owned(),
        Some(refresh.to_owned()),
    ))
    .expect("planted persist retains a Client");
    rt.block_on(shared.attach_session_owners())
        .expect("attach after retained session");
    rt.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(15), shared.start_sync())
            .await
            .expect("start_sync timed out")
    })
    .expect("start after attach");

    let room_keys = rt
        .block_on(shared.room_key_transfer_status())
        .expect("room-key status is local owner state");
    let room_keys_text = format!("{room_keys:?}");
    assert!(!room_keys.phase.is_empty());
    assert!(!room_keys_text.contains(access));
    assert!(!room_keys_text.contains(refresh));
    assert!(!room_keys_text.contains("password"));
    assert!(!room_keys_text.contains("@alice"));
    assert!(!room_keys_text.contains("https://"));

    let recover = rt
        .block_on(shared.recover(recovery_key.to_owned()))
        .expect_err("recover needs live secret-storage I/O");
    let recover_text = format!("{recover:?}{recover}");
    assert!(recover_text.contains("p4-s10-leftover-unavailable"));
    assert!(!recover_text.contains(recovery_key));
    assert!(!recover_text.contains(access));

    let raw = rt
        .block_on(shared.send_raw_room_event(
            room_id.to_owned(),
            "m.room.message".to_owned(),
            event_body.to_owned(),
        ))
        .expect_err("raw send needs a live homeserver");
    let raw_text = format!("{raw:?}{raw}");
    assert!(raw_text.contains("p4-s10-leftover-unavailable"));
    assert!(!raw_text.contains(room_id));
    assert!(!raw_text.contains(event_body));

    let media = rt
        .block_on(shared.media_download(mxc.to_owned()))
        .expect_err("media download needs a live homeserver");
    let media_text = format!("{media:?}{media}");
    assert!(media_text.contains("p4-s10-leftover-unavailable"));
    assert!(!media_text.contains(mxc));

    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}
