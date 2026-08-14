//! P4-S7: typed SharedCore consume of typing/presence commands only.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s7-it-{tag}-{nanos}"));
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
fn typing_presence_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("typing_snapshot"));
    assert!(udl.contains("typing_set"));
    assert!(udl.contains("presence_snapshot"));
    assert!(udl.contains("presence_subscribe"));
    assert!(udl.contains("presence_unsubscribe"));
    assert!(!udl.contains("matrix_verification_start"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_timeline_jump_latest"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("typing_snapshot"));
    assert!(shared_core.contains("typing_set"));
    assert!(shared_core.contains("presence_snapshot"));
    assert!(shared_core.contains("presence_subscribe"));
    assert!(shared_core.contains("presence_unsubscribe"));
    assert!(shared_core.contains("timeline_open"));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("device_delete_password"));
    assert!(!shared_core.contains("room_notes_snapshot"));
    assert!(!shared_core.contains("crypto_status"));
    assert!(!shared_core.contains("jump_latest"));
}

#[test]
fn typing_presence_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let typing = rt
        .block_on(shared.typing_snapshot())
        .expect_err("no attached typing owner");
    let typing_set = rt
        .block_on(shared.typing_set("!r:example.org".to_owned(), true))
        .expect_err("no attached typing owner");
    let presence = rt
        .block_on(shared.presence_snapshot("@bob:example.org".to_owned()))
        .expect_err("no attached presence owner");
    let subscribe = rt
        .block_on(shared.presence_subscribe("@bob:example.org".to_owned()))
        .expect_err("no attached presence owner");
    let unsubscribe = rt
        .block_on(shared.presence_unsubscribe("presence-1-0".to_owned()))
        .expect_err("no attached presence owner");
    let text = format!(
        "{typing:?}{typing}{typing_set:?}{typing_set}{presence:?}{presence}{subscribe:?}{subscribe}{unsubscribe:?}{unsubscribe}"
    );
    assert!(text.contains("p2-typing-snapshot-no-session"));
    assert!(text.contains("p2-typing-set-no-session"));
    assert!(text.contains("p2-presence-snapshot-no-session"));
    assert!(text.contains("p2-presence-subscribe-no-session"));
    assert!(text.contains("p2-presence-unsubscribe-no-session"));
    assert!(!text.contains("password"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("@bob"));
}

#[test]
fn typing_presence_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s7_typing_access";
    let refresh = "syr_s7_typing_refresh";
    let identity = alice();
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("typing-presence-no-start");
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
    rt.block_on(shared.attach_session_owners())
        .expect("owners attached");

    let typing = rt
        .block_on(shared.typing_snapshot())
        .expect("unstarted sync yields the registered typing snapshot");
    assert_eq!(typing.session_generation, 1);
    assert!(typing.rooms.is_empty());

    let typing_set = rt
        .block_on(shared.typing_set("!missing:example.org".to_owned(), true))
        .expect_err("unstarted sync still uses the registered typing-set handler");
    let typing_set_text = format!("{typing_set:?}{typing_set}");
    assert!(typing_set_text.contains("v-rooms.4-typing-room-missing"));

    let presence = rt
        .block_on(shared.presence_snapshot("@bob:example.org".to_owned()))
        .expect("unstarted sync yields the registered presence snapshot");
    assert_eq!(presence.status, "unknown");
    assert_eq!(presence.session_generation, 1);
    assert_eq!(presence.user_id, "@bob:example.org");

    let subscription = rt
        .block_on(shared.presence_subscribe("@bob:example.org".to_owned()))
        .expect("unstarted sync yields the registered subscribe handler");
    assert_eq!(subscription.session_generation, 1);
    assert_eq!(subscription.user_id, "@bob:example.org");
    assert!(subscription.subscription_id.starts_with("presence-1-"));

    rt.block_on(shared.presence_unsubscribe(subscription.subscription_id.clone()))
        .expect("unsubscribe of a live subscription is the registered handler result");

    let text = format!("{typing:?}{presence:?}{subscription:?}{typing_set_text}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("password"));
    assert!(!text.contains("syt_"));

    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}
