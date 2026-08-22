//! P4 leftover: typed SharedCore consume of `message_search`.
//!
//! Calls the already-registered Core handler. Does not start SyncService.
//! Search term, optional next token, rooms, senders, and order may cross as
//! method arguments. Results stay ids plus a capped body snippet. Failed
//! errors stay static and must not echo term, room id, event id, or tokens.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
use synara_core::transport::MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-message-search-it-{tag}-{nanos}"));
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
fn message_search_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("message_search"));
    assert!(udl.contains("interface MessageSearchError"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("message_search"));
    assert!(shared_core.contains("user_directory_search"));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("matrix_backup_status"));
}

#[test]
fn message_search_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let term = "s9MessageSearchSecretTerm";
    let search = rt
        .block_on(shared.message_search(
            term.to_owned(),
            None,
            Some(vec!["!r:example.org".to_owned()]),
            None,
            Some("recent".to_owned()),
        ))
        .expect_err("no attached message-search owner");
    let text = format!("{search:?}{search}");
    assert!(text.contains("p2-message-search-no-session"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(term));
    assert!(!text.contains("!r:example.org"));
    assert!(!text.contains("@alice"));
}

#[test]
fn message_search_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let term = "t".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8);
    let rt = test_runtime();
    let search = rt
        .block_on(shared.message_search(term.clone(), None, None, None, None))
        .expect_err("oversize message-search payload must fail closed");
    let text = format!("{search:?}{search}");
    assert!(text.contains("p4-s9-message-search-failed"));
    assert!(!text.contains(&term));
}

#[test]
fn message_search_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_message_search_access";
    let refresh = "syr_s9_message_search_refresh";
    let identity = alice();
    let term = "s9MessageSearchSecretTerm";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("message-search-no-start");
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

    let search = rt.block_on(shared.message_search(
        term.to_owned(),
        None,
        Some(vec!["!r:example.org".to_owned()]),
        None,
        Some("recent".to_owned()),
    ));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let search_text = search
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered message-search handler");

    assert!(
        search_text.contains("v-search."),
        "search must return a registered owner diagnostic: {search_text}"
    );
    assert!(
        !search_text.contains("p4-s9-message-search-failed"),
        "search must not hide a wrong envelope behind the generic fallback: {search_text}"
    );
    assert!(!search_text.contains("leftover-unavailable"));
    assert!(!search_text.contains(access));
    assert!(!search_text.contains(refresh));
    assert!(!search_text.contains("syt_"));
    assert!(!search_text.contains(term));
    assert!(!search_text.contains("!r:example.org"));
    assert!(!search_text.contains("@alice"));
}
