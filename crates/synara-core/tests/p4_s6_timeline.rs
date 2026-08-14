//! P4-S6: typed SharedCore consume of timeline open/close/paginate only.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
use synara_core::{IosSecretVault, IosSecretVaultError, SharedCore, TimelineOpenPositionDto};

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
    let root = std::env::temp_dir().join(format!("synara-p4-s6-it-{tag}-{nanos}"));
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

fn live_bottom() -> TimelineOpenPositionDto {
    TimelineOpenPositionDto {
        kind: "live_bottom".to_owned(),
        at_bottom: false,
        restored_anchor_event_id: None,
        live_tail_event_id: None,
        updated_at_ms: None,
        event_id: None,
    }
}

#[test]
fn timeline_surface_exposes_only_open_close_paginate() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("timeline_open"));
    assert!(udl.contains("timeline_close"));
    assert!(udl.contains("timeline_paginate"));
    assert!(!udl.contains("composer_set_reply_draft"));
    assert!(!udl.contains("matrix_composer_set_reply_draft"));
    assert!(!udl.contains("matrix_composer_get_reply_draft"));
    assert!(!udl.contains("matrix_composer_clear_reply_draft"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_verification_start"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("timeline_open"));
    assert!(shared_core.contains("timeline_close"));
    assert!(shared_core.contains("timeline_paginate"));
    assert!(shared_core.contains("invites_snapshot"));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("composer_set_reply_draft"));
    assert!(!shared_core.contains("composer_get_reply_draft"));
    assert!(!shared_core.contains("composer_clear_reply_draft"));
}

#[test]
fn timeline_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let open = rt
        .block_on(shared.timeline_open("!missing:example.org".to_owned(), live_bottom()))
        .expect_err("no attached timeline owner");
    let close = rt
        .block_on(shared.timeline_close("view-1".to_owned()))
        .expect_err("no attached timeline owner");
    let paginate = rt
        .block_on(shared.timeline_paginate("view-1".to_owned(), "backwards".to_owned()))
        .expect_err("no attached timeline owner");
    let text = format!("{open:?}{open}{close:?}{close}{paginate:?}{paginate}");
    assert!(text.contains("p2-timeline-open-no-session"));
    assert!(text.contains("p2-timeline-close-no-session"));
    assert!(text.contains("p2-timeline-paginate-no-session"));
    assert!(!text.contains("password"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("@alice"));
}

#[test]
fn timeline_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s6_timeline_access";
    let refresh = "syr_s6_timeline_refresh";
    let identity = alice();
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("timeline-no-start");
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

    let open = rt
        .block_on(shared.timeline_open("!missing:example.org".to_owned(), live_bottom()))
        .expect_err("unstarted sync still uses the registered open handler");
    let open_text = format!("{open:?}{open}");
    assert!(open_text.contains("d0.3-timeline-room-not-found"));
    assert!(!open_text.contains(access));
    assert!(!open_text.contains(refresh));
    assert!(!open_text.contains("password"));
    assert!(!open_text.contains("syt_"));

    let closed = rt
        .block_on(shared.timeline_close("view-missing".to_owned()))
        .expect("close of an unknown stream is the registered handler's false");
    assert!(!closed);

    let paginate = rt
        .block_on(shared.timeline_paginate("view-missing".to_owned(), "backwards".to_owned()))
        .expect_err("paginate of an unknown stream uses the registered handler");
    let paginate_text = format!("{paginate:?}{paginate}");
    assert!(paginate_text.contains("v-timeline-view-not-open"));
    assert!(!paginate_text.contains(access));
    assert!(!paginate_text.contains(refresh));
    assert!(!paginate_text.contains("password"));
    assert!(!paginate_text.contains("syt_"));

    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}
