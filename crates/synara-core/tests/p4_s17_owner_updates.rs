//! P4-S17: poll privacy-safe owner emit summaries on SharedCore.
//!
//! Presence, devices, join_rules, and image_packs. This is not
//! Platform::emit, not Core.command, and not P4 acceptance. NSE still
//! cannot poll. Presence user ids never appear.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s17-it-{tag}-{nanos}"));
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
fn owner_update_surface_is_poll_only_and_not_a_leftover() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("dictionary OwnerUpdateDto"));
    assert!(udl.contains("interface OwnerUpdateError"));
    assert!(udl.contains("sequence<OwnerUpdateDto> poll_owner_updates()"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("poll_owner_updates()"));
    assert!(!shared_core.contains("command("));
}

#[test]
fn poll_owner_updates_without_attach_returns_empty() {
    let shared = SharedCore::new();
    let updates = test_runtime()
        .block_on(shared.poll_owner_updates())
        .expect("empty queue is success");
    assert!(updates.is_empty());
    let text = format!("{updates:?}");
    assert!(!text.contains("password"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("@alice"));
}

#[test]
fn enqueue_then_poll_owner_updates_is_privacy_safe() {
    let shared = SharedCore::new();
    shared.enqueue_owner_update_for_test("presence".to_owned(), 3, None);
    shared.enqueue_owner_update_for_test(
        "join_rules".to_owned(),
        3,
        Some("!s17Room:example.org".to_owned()),
    );
    let updates = test_runtime()
        .block_on(shared.poll_owner_updates())
        .expect("queued summaries drain");
    assert_eq!(updates.len(), 2);
    assert_eq!(updates[0].family, "presence");
    assert_eq!(updates[0].session_generation, 3);
    assert_eq!(updates[0].room_id, None);
    assert_eq!(updates[1].family, "join_rules");
    assert_eq!(updates[1].room_id.as_deref(), Some("!s17Room:example.org"));
    let text = format!("{updates:?}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("password"));
    assert!(!text.contains("@alice"));
    assert!(!text.contains("https://"));
    let drained = test_runtime()
        .block_on(shared.poll_owner_updates())
        .expect("second poll is empty");
    assert!(drained.is_empty());
}

#[test]
fn poll_owner_updates_on_nse_store_fails_closed_without_echo() {
    let access = "syt_s17_nse_poll_access";
    let refresh = "syr_s17_nse_poll_refresh";
    let identity = alice();
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("nse-forbids-owner-poll");
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
    .expect("planted persist");
    rt.block_on(shared.nse_open_read_only_store(
        identity.user_id().to_owned(),
        identity.homeserver_url().to_owned(),
        root.to_string_lossy().into_owned(),
    ))
    .expect("planted NSE open");
    shared.enqueue_owner_update_for_test("devices".to_owned(), 1, None);
    let error = rt
        .block_on(shared.poll_owner_updates())
        .expect_err("NSE cannot poll owner updates");
    let text = format!("{error:?}{error}");
    assert!(text.contains("p4-s17-nse-forbids-poll"));
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("@alice"));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}
