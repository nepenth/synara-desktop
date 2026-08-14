//! P4-S9-12: typed SharedCore consume of the two registered room leave/join commands.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Room id, alias, and via servers may cross as method arguments. Write ack
//! is status only. Failed errors stay static and must not echo room id,
//! alias, or via servers. Power levels/room create and leftover secret
//! envelopes stay off. Directory search is already S9-11.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-12-it-{tag}-{nanos}"));
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
fn room_leave_join_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("room_leave"));
    assert!(udl.contains("room_join("));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(!udl.contains("matrix_room_create"));
    assert!(!udl.contains("matrix_room_members_snapshot"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("room_leave"));
    assert!(shared_core.contains("room_join("));
    assert!(shared_core.contains("room_directory_search"));
    assert!(shared_core.contains("set_room_name"));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("room_members_snapshot"));
    assert!(!shared_core.contains("space_parents_snapshot"));
    assert!(!shared_core.contains("backup_status"));
}

#[test]
fn room_leave_join_family_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s912SecretRoom:example.org";
    let alias = "#s912SecretAlias:example.org";
    let via = "s912.secret.example.org";
    let leave = rt
        .block_on(shared.room_leave(room_id.to_owned()))
        .expect_err("no attached room-membership owner");
    let join = rt
        .block_on(shared.room_join(alias.to_owned(), Some(vec![via.to_owned()])))
        .expect_err("no attached room-membership owner");
    let leave_text = format!("{leave:?}{leave}");
    let join_text = format!("{join:?}{join}");
    assert!(leave_text.contains("p2-room-leave-no-session"));
    assert!(join_text.contains("p2-room-join-no-session"));
    let text = format!("{leave_text}{join_text}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(alias));
    assert!(!text.contains(via));
    assert!(!text.contains("@alice"));
}

#[test]
fn room_leave_join_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let room_id = format!(
        "!{}:example.org",
        "s".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let alias = format!(
        "#{}:example.org",
        "a".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let via = format!(
        "{}.example.org",
        "v".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let rt = test_runtime();
    let leave = rt
        .block_on(shared.room_leave(room_id.clone()))
        .expect_err("oversize room-leave payload must fail closed");
    let join = rt
        .block_on(shared.room_join(alias.clone(), Some(vec![via.clone()])))
        .expect_err("oversize room-join payload must fail closed");
    let leave_text = format!("{leave:?}{leave}");
    let join_text = format!("{join:?}{join}");
    assert!(leave_text.contains("p4-s9-12-room-membership-failed"));
    assert!(join_text.contains("p4-s9-12-room-membership-failed"));
    assert!(!leave_text.contains(&room_id));
    assert!(!join_text.contains(&alias));
    assert!(!join_text.contains(&via));
}

#[test]
fn room_leave_join_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_12_room_membership_access";
    let refresh = "syr_s9_12_room_membership_refresh";
    let identity = alice();
    let room_id = "!s912SecretRoom:example.org";
    let invalid_join = "s912-not-a-room";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("room-leave-join-no-start");
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

    let leave = rt.block_on(shared.room_leave(room_id.to_owned()));
    let join = rt.block_on(shared.room_join(invalid_join.to_owned(), None));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let leave_text = leave
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted leave must not require a live server");
    let join_text = join
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted join must fail locally on an invalid id");

    assert!(
        leave_text.contains("v-rooms-room-leave-"),
        "leave must return a registered owner diagnostic: {leave_text}"
    );
    assert!(
        join_text.contains("v-rooms-room-join-"),
        "join must return a registered owner diagnostic: {join_text}"
    );
    assert!(
        !leave_text.contains("p4-s9-12-room-membership-failed"),
        "leave must not hide a wrong envelope behind the generic fallback: {leave_text}"
    );
    assert!(
        !join_text.contains("p4-s9-12-room-membership-failed"),
        "join must not hide a wrong envelope behind the generic fallback: {join_text}"
    );
    let text = format!("{leave_text}{join_text}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(invalid_join));
}
