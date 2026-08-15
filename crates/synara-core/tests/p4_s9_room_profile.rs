//! P4-S9-9: typed SharedCore consume of the three registered room-profile commands.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Room id, name, topic, and `mxc://` (or empty clear) may cross as method
//! arguments. Image/media bytes stay off. Failed errors stay static and must
//! not echo room id, name, topic, or mxc. Directory visibility and leftover
//! secret envelopes stay off. Join-rule snapshot is already S9-3.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-9-it-{tag}-{nanos}"));
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
fn room_profile_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("set_room_name"));
    assert!(udl.contains("set_room_topic"));
    assert!(udl.contains("set_room_avatar"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(!udl.contains("matrix_upload_media"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("set_room_name"));
    assert!(shared_core.contains("set_room_topic"));
    assert!(shared_core.contains("set_room_avatar"));
    assert!(shared_core.contains("set_own_display_name"));
    assert!(shared_core.contains("room_join_rule_snapshot"));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("invites_accept"));
    assert!(!shared_core.contains("backup_status"));
}

#[test]
fn room_profile_family_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s99SecretRoom:example.org";
    let name = "S99 Secret Room Name";
    let topic = "S99 Secret Room Topic";
    let mxc = "mxc://example.org/s99SecretRoomAvatarId";
    let name_result = rt
        .block_on(shared.set_room_name(room_id.to_owned(), name.to_owned()))
        .expect_err("no attached room-profile owner");
    let topic_result = rt
        .block_on(shared.set_room_topic(room_id.to_owned(), topic.to_owned()))
        .expect_err("no attached room-profile owner");
    let avatar = rt
        .block_on(shared.set_room_avatar(room_id.to_owned(), mxc.to_owned()))
        .expect_err("no attached room-profile owner");
    let name_text = format!("{name_result:?}{name_result}");
    let topic_text = format!("{topic_result:?}{topic_result}");
    let avatar_text = format!("{avatar:?}{avatar}");
    assert!(name_text.contains("p2-set-room-name-no-session"));
    assert!(topic_text.contains("p2-set-room-topic-no-session"));
    assert!(avatar_text.contains("p2-set-room-avatar-no-session"));
    let text = format!("{name_text}{topic_text}{avatar_text}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(name));
    assert!(!text.contains(topic));
    assert!(!text.contains(mxc));
    assert!(!text.contains("@alice"));
}

#[test]
fn room_profile_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let room_id = "!s99SecretRoom:example.org";
    let name = "x".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8);
    let topic = "y".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8);
    let mxc = format!(
        "mxc://example.org/{}",
        "z".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let rt = test_runtime();
    let name_result = rt
        .block_on(shared.set_room_name(room_id.to_owned(), name.clone()))
        .expect_err("oversize room-name payload must fail closed");
    let topic_result = rt
        .block_on(shared.set_room_topic(room_id.to_owned(), topic.clone()))
        .expect_err("oversize room-topic payload must fail closed");
    let avatar = rt
        .block_on(shared.set_room_avatar(room_id.to_owned(), mxc.clone()))
        .expect_err("oversize room-avatar payload must fail closed");
    let name_text = format!("{name_result:?}{name_result}");
    let topic_text = format!("{topic_result:?}{topic_result}");
    let avatar_text = format!("{avatar:?}{avatar}");
    assert!(name_text.contains("p4-s9-9-room-profile-failed"));
    assert!(topic_text.contains("p4-s9-9-room-profile-failed"));
    assert!(avatar_text.contains("p4-s9-9-room-profile-failed"));
    assert!(!name_text.contains(&name));
    assert!(!topic_text.contains(&topic));
    assert!(!avatar_text.contains(&mxc));
    assert!(!name_text.contains(room_id));
    assert!(!topic_text.contains(room_id));
    assert!(!avatar_text.contains(room_id));
}

#[test]
fn room_profile_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_9_room_profile_access";
    let refresh = "syr_s9_9_room_profile_refresh";
    let identity = alice();
    let room_id = "!s99SecretRoom:example.org";
    let name = "S99 Secret Room Name";
    let topic = "S99 Secret Room Topic";
    let mxc = "mxc://example.org/s99SecretRoomAvatarId";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("room-profile-no-start");
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

    let name_result = rt.block_on(shared.set_room_name(room_id.to_owned(), name.to_owned()));
    let topic_result = rt.block_on(shared.set_room_topic(room_id.to_owned(), topic.to_owned()));
    let avatar = rt.block_on(shared.set_room_avatar(room_id.to_owned(), mxc.to_owned()));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let name_text = name_result
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered room-name handler");
    let topic_text = topic_result
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered room-topic handler");
    let avatar_text = avatar
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered room-avatar handler");

    assert!(
        name_text.contains("v-send.r-room-profile-"),
        "room name must return a registered owner diagnostic: {name_text}"
    );
    assert!(
        topic_text.contains("v-send.r-room-profile-"),
        "room topic must return a registered owner diagnostic: {topic_text}"
    );
    assert!(
        avatar_text.contains("v-send.r-room-profile-"),
        "room avatar must return a registered owner diagnostic: {avatar_text}"
    );
    assert!(
        !name_text.contains("p4-s9-9-room-profile-failed"),
        "room name must not hide a wrong envelope behind the generic fallback: {name_text}"
    );
    assert!(
        !topic_text.contains("p4-s9-9-room-profile-failed"),
        "room topic must not hide a wrong envelope behind the generic fallback: {topic_text}"
    );
    assert!(
        !avatar_text.contains("p4-s9-9-room-profile-failed"),
        "room avatar must not hide a wrong envelope behind the generic fallback: {avatar_text}"
    );
    let text = format!("{name_text}{topic_text}{avatar_text}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(name));
    assert!(!text.contains(topic));
    assert!(!text.contains(mxc));
}
