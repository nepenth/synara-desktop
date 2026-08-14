//! P4-S9-10: typed SharedCore consume of the two registered directory-visibility commands.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Room id, session generation, and public/private visibility may cross as
//! method arguments. Failed errors stay static and must not echo room id or
//! visibility. Power levels/room create and leftover secret envelopes stay off.
//! Room name/topic/avatar is already S9-9.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-10-it-{tag}-{nanos}"));
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
fn directory_visibility_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("get_room_directory_visibility"));
    assert!(udl.contains("set_room_directory_visibility"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(!udl.contains("matrix_room_create"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("get_room_directory_visibility"));
    assert!(shared_core.contains("set_room_directory_visibility"));
    assert!(shared_core.contains("set_room_name"));
    assert!(shared_core.contains("room_join_rule_snapshot"));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("poll_respond"));
    assert!(!shared_core.contains("backup_status"));
}

#[test]
fn directory_visibility_family_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s910SecretRoom:example.org";
    let visibility = "public";
    let get = rt
        .block_on(shared.get_room_directory_visibility(room_id.to_owned(), 1))
        .expect_err("no attached directory-visibility owner");
    let set = rt
        .block_on(shared.set_room_directory_visibility(
            room_id.to_owned(),
            1,
            visibility.to_owned(),
        ))
        .expect_err("no attached directory-visibility owner");
    let get_text = format!("{get:?}{get}");
    let set_text = format!("{set:?}{set}");
    assert!(get_text.contains("p2-get-room-directory-visibility-no-session"));
    assert!(set_text.contains("p2-set-room-directory-visibility-no-session"));
    let text = format!("{get_text}{set_text}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(visibility));
    assert!(!text.contains("@alice"));
}

#[test]
fn directory_visibility_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let room_id = format!(
        "!{}:example.org",
        "s".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let visibility = "v".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8);
    let rt = test_runtime();
    let get = rt
        .block_on(shared.get_room_directory_visibility(room_id.clone(), 1))
        .expect_err("oversize directory-visibility get payload must fail closed");
    let set = rt
        .block_on(shared.set_room_directory_visibility(room_id.clone(), 1, visibility.clone()))
        .expect_err("oversize directory-visibility set payload must fail closed");
    let get_text = format!("{get:?}{get}");
    let set_text = format!("{set:?}{set}");
    assert!(get_text.contains("p4-s9-10-directory-visibility-failed"));
    assert!(set_text.contains("p4-s9-10-directory-visibility-failed"));
    assert!(!get_text.contains(&room_id));
    assert!(!set_text.contains(&room_id));
    assert!(!set_text.contains(&visibility));
}

#[test]
fn directory_visibility_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_10_directory_visibility_access";
    let refresh = "syr_s9_10_directory_visibility_refresh";
    let identity = alice();
    let room_id = "!s910SecretRoom:example.org";
    let visibility = "public";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("directory-visibility-no-start");
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

    let get = rt.block_on(shared.get_room_directory_visibility(room_id.to_owned(), 1));
    let set = rt.block_on(shared.set_room_directory_visibility(
        room_id.to_owned(),
        1,
        visibility.to_owned(),
    ));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let get_text = get
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered directory-visibility get handler");
    let set_text = set
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered directory-visibility set handler");

    assert!(
        get_text.contains("v-send.r-room-profile-directory-visibility-"),
        "get must return a registered owner diagnostic: {get_text}"
    );
    assert!(
        set_text.contains("v-send.r-room-profile-directory-visibility-"),
        "set must return a registered owner diagnostic: {set_text}"
    );
    assert!(
        !get_text.contains("p4-s9-10-directory-visibility-failed"),
        "get must not hide a wrong envelope behind the generic fallback: {get_text}"
    );
    assert!(
        !set_text.contains("p4-s9-10-directory-visibility-failed"),
        "set must not hide a wrong envelope behind the generic fallback: {set_text}"
    );
    let text = format!("{get_text}{set_text}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(visibility));
}
