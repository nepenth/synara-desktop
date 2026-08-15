//! P4-S9-11: typed SharedCore consume of the three registered directory-search commands.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Session generation, request id, server/term/alias metadata may cross as
//! method arguments. Search results stay metadata (room ids, names, aliases,
//! mxc). Avatar bytes stay off. Failed errors stay static and must not echo
//! term, server, or room id. Cancel is local and must not require a live
//! server on a planted session. Power levels/room create and leftover secret
//! envelopes stay off. Directory visibility is already S9-10.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-11-it-{tag}-{nanos}"));
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
fn directory_search_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("room_directory_protocols"));
    assert!(udl.contains("room_directory_search"));
    assert!(udl.contains("room_directory_cancel"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(!udl.contains("matrix_room_create"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("room_directory_protocols"));
    assert!(shared_core.contains("room_directory_search"));
    assert!(shared_core.contains("room_directory_cancel"));
    assert!(shared_core.contains("get_room_directory_visibility"));
    assert!(shared_core.contains("set_room_name"));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("room_create"));
    assert!(!shared_core.contains("backup_status"));
}

#[test]
fn directory_search_family_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let term = "s911SecretTerm";
    let server = "s911.secret.example.org";
    let protocols = rt
        .block_on(shared.room_directory_protocols())
        .expect_err("no attached directory-search owner");
    let search = rt
        .block_on(shared.room_directory_search(
            1,
            1,
            Some(server.to_owned()),
            Some(term.to_owned()),
            None,
            None,
            20,
            None,
        ))
        .expect_err("no attached directory-search owner");
    let cancel = rt
        .block_on(shared.room_directory_cancel(1, 1))
        .expect_err("no attached directory-search owner");
    let protocols_text = format!("{protocols:?}{protocols}");
    let search_text = format!("{search:?}{search}");
    let cancel_text = format!("{cancel:?}{cancel}");
    assert!(protocols_text.contains("p2-room-directory-protocols-no-session"));
    assert!(search_text.contains("p2-room-directory-search-no-session"));
    assert!(cancel_text.contains("p2-room-directory-cancel-no-session"));
    let text = format!("{protocols_text}{search_text}{cancel_text}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(term));
    assert!(!text.contains(server));
    assert!(!text.contains("@alice"));
}

#[test]
fn directory_search_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let term = "t".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8);
    let server = format!(
        "{}.example.org",
        "s".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let rt = test_runtime();
    let search = rt
        .block_on(shared.room_directory_search(
            1,
            1,
            Some(server.clone()),
            Some(term.clone()),
            None,
            None,
            20,
            None,
        ))
        .expect_err("oversize directory-search payload must fail closed");
    let text = format!("{search:?}{search}");
    assert!(text.contains("p4-s9-11-directory-search-failed"));
    assert!(!text.contains(&term));
    assert!(!text.contains(&server));
}

#[test]
fn directory_search_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_11_directory_search_access";
    let refresh = "syr_s9_11_directory_search_refresh";
    let identity = alice();
    let term = "s911SecretTerm";
    let server = "s911.secret.example.org";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("directory-search-no-start");
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

    let protocols = rt.block_on(shared.room_directory_protocols());
    let search = rt.block_on(shared.room_directory_search(
        1,
        1,
        Some(server.to_owned()),
        Some(term.to_owned()),
        None,
        None,
        20,
        None,
    ));
    let cancel = rt
        .block_on(shared.room_directory_cancel(1, 2))
        .expect("planted cancel must not require a live server");
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let protocols_text = protocols
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered directory-protocols handler");
    let search_text = search
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered directory-search handler");

    assert!(
        protocols_text.contains("v-rooms.directory-"),
        "protocols must return a registered owner diagnostic: {protocols_text}"
    );
    assert!(
        search_text.contains("v-rooms.directory-"),
        "search must return a registered owner diagnostic: {search_text}"
    );
    assert!(
        !protocols_text.contains("p4-s9-11-directory-search-failed"),
        "protocols must not hide a wrong envelope behind the generic fallback: {protocols_text}"
    );
    assert!(
        !search_text.contains("p4-s9-11-directory-search-failed"),
        "search must not hide a wrong envelope behind the generic fallback: {search_text}"
    );
    assert_eq!(cancel.status, "cancelled");
    assert!(cancel.page.is_none());
    assert_eq!(cancel.session_generation, 1);
    assert_eq!(cancel.request_id, 2);
    let text = format!("{protocols_text}{search_text}{cancel:?}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(term));
    assert!(!text.contains(server));
}
