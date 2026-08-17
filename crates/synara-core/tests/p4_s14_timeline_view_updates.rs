//! P4-S14: poll privacy-safe timeline view-delta summaries on SharedCore.
//!
//! This is not Platform::emit, not Core.command, and not P4 acceptance.
//! NSE still cannot poll. Empty queue is success, not an error.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
use synara_core::app::timeline::TIMELINE_VIEW_SCHEMA_VERSION;
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
    let root = std::env::temp_dir().join(format!("synara-p4-s14-it-{tag}-{nanos}"));
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
fn timeline_view_update_surface_is_poll_only_and_not_a_leftover() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("dictionary TimelineViewUpdateDto"));
    assert!(udl.contains("interface TimelineViewUpdateError"));
    assert!(udl.contains("sequence<TimelineViewUpdateDto> poll_timeline_view_updates()"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("poll_timeline_view_updates()"));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("matrix_login_password"));
    assert!(!shared_core.contains("Platform::emit"));
}

#[test]
fn poll_timeline_view_updates_without_attach_returns_empty() {
    let shared = SharedCore::new();
    let updates = test_runtime()
        .block_on(shared.poll_timeline_view_updates())
        .expect("empty queue is success");
    assert!(updates.is_empty());
    let text = format!("{updates:?}");
    assert!(!text.contains("password"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("@alice"));
    assert!(!text.contains("https://"));
}

#[test]
fn enqueue_then_poll_returns_privacy_safe_summaries() {
    let access = "syt_s14_timeline_view_access";
    let refresh = "syr_s14_timeline_view_refresh";
    let user_id = "@alice:example.org";
    let homeserver = "https://matrix.example.org";
    let shared = SharedCore::new();
    shared.enqueue_timeline_view_update_for_test(
        "stream-s14".to_owned(),
        "!s14Room:example.org".to_owned(),
        7,
    );
    let updates = test_runtime()
        .block_on(shared.poll_timeline_view_updates())
        .expect("queued summary drains");
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].schema_version, TIMELINE_VIEW_SCHEMA_VERSION);
    assert_eq!(updates[0].session_generation, 1);
    assert_eq!(updates[0].stream_id, "stream-s14");
    assert_eq!(updates[0].room_id, "!s14Room:example.org");
    assert_eq!(updates[0].revision, 7);
    assert_eq!(updates[0].op_count, 0);
    let text = format!("{updates:?}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("password"));
    assert!(!text.contains(user_id));
    assert!(!text.contains(homeserver));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    let drained = test_runtime()
        .block_on(shared.poll_timeline_view_updates())
        .expect("second poll is empty");
    assert!(drained.is_empty());
}

#[test]
fn poll_timeline_view_updates_on_nse_store_fails_closed_without_echo() {
    let access = "syt_s14_nse_poll_access";
    let refresh = "syr_s14_nse_poll_refresh";
    let identity = alice();
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("nse-forbids-poll");
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
    shared.enqueue_timeline_view_update_for_test(
        "stream-nse".to_owned(),
        "!s14NseRoom:example.org".to_owned(),
        3,
    );
    let error = rt
        .block_on(shared.poll_timeline_view_updates())
        .expect_err("NSE cannot poll timeline view updates");
    let text = format!("{error:?}{error}");
    assert!(text.contains("p4-s14-nse-forbids-poll"));
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("@alice"));
    assert!(!text.contains("https://"));
    assert!(!text.contains("!s14NseRoom"));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}
