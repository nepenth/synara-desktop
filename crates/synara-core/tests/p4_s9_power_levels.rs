//! P4-S9-14: typed SharedCore consume of the three registered power-level writers.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Room id, user id, power level, and content JSON may cross as method
//! arguments. Write ack is status only. Failed errors stay static and must
//! not echo room id, user id, power level, or content. Members snapshots
//! and spaces stay off. Invite/kick/ban is already S9-13. Room create is
//! S9-15.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-14-it-{tag}-{nanos}"));
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
fn room_power_levels_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("room_set_power_level("));
    assert!(udl.contains("room_set_power_levels("));
    assert!(udl.contains("room_set_power_level_tags("));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(!udl.contains("matrix_room_create"));
    assert!(!udl.contains("matrix_send_sticker"));
    assert!(!udl.contains("matrix_send_poll"));
    assert!(!udl.contains("matrix_edit_message"));
    assert!(!udl.contains("matrix_poll_respond"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("room_set_power_level("));
    assert!(shared_core.contains("room_set_power_levels("));
    assert!(shared_core.contains("room_set_power_level_tags("));
    assert!(shared_core.contains("room_invite"));
    assert!(shared_core.contains("room_unban"));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("edit_message"));
    assert!(!shared_core.contains("poll_respond"));
    assert!(!shared_core.contains("backup_status"));
}

#[test]
fn room_power_levels_family_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s914SecretRoom:example.org";
    let user_id = "@s914SecretUser:example.org";
    let power_level = 914_i64;
    let content = r#"{"users":{"@s914SecretUser:example.org":50}}"#;
    let tags = r#"{"50":{"name":"s914SecretTag"}}"#;
    let single = rt
        .block_on(shared.room_set_power_level(room_id.to_owned(), user_id.to_owned(), power_level))
        .expect_err("no attached room-power-level owner");
    let bulk = rt
        .block_on(shared.room_set_power_levels(room_id.to_owned(), content.to_owned()))
        .expect_err("no attached room-power-levels owner");
    let tags_err = rt
        .block_on(shared.room_set_power_level_tags(room_id.to_owned(), tags.to_owned()))
        .expect_err("no attached room-power-level-tags owner");
    let single_text = format!("{single:?}{single}");
    let bulk_text = format!("{bulk:?}{bulk}");
    let tags_text = format!("{tags_err:?}{tags_err}");
    assert!(single_text.contains("p2-room-set-power-level-no-session"));
    assert!(bulk_text.contains("p2-room-set-power-levels-no-session"));
    assert!(tags_text.contains("p2-room-set-power-level-tags-no-session"));
    let text = format!("{single_text}{bulk_text}{tags_text}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(user_id));
    assert!(!text.contains("914"));
    assert!(!text.contains(content));
    assert!(!text.contains(tags));
    assert!(!text.contains("@alice"));
}

#[test]
fn room_power_levels_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let room_id = format!(
        "!{}:example.org",
        "s".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let user_id = format!(
        "@{}:example.org",
        "u".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let content = format!(
        "{{\"users\":{{\"@{}:example.org\":50}}}}",
        "c".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let rt = test_runtime();
    let single = rt
        .block_on(shared.room_set_power_level(room_id.clone(), user_id.clone(), 50))
        .expect_err("oversize set-power-level payload must fail closed");
    let bulk = rt
        .block_on(shared.room_set_power_levels(room_id.clone(), content.clone()))
        .expect_err("oversize set-power-levels payload must fail closed");
    let tags = rt
        .block_on(shared.room_set_power_level_tags(room_id.clone(), content.clone()))
        .expect_err("oversize set-power-level-tags payload must fail closed");
    let single_text = format!("{single:?}{single}");
    let bulk_text = format!("{bulk:?}{bulk}");
    let tags_text = format!("{tags:?}{tags}");
    assert!(single_text.contains("p4-s9-14-room-power-levels-failed"));
    assert!(bulk_text.contains("p4-s9-14-room-power-levels-failed"));
    assert!(tags_text.contains("p4-s9-14-room-power-levels-failed"));
    assert!(!single_text.contains(&room_id));
    assert!(!single_text.contains(&user_id));
    assert!(!bulk_text.contains(&room_id));
    assert!(!bulk_text.contains(&content));
    assert!(!tags_text.contains(&room_id));
}

#[test]
fn room_power_levels_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_14_room_power_levels_access";
    let refresh = "syr_s9_14_room_power_levels_refresh";
    let identity = alice();
    let room_id = "!s914SecretRoom:example.org";
    let user_id = "@s914SecretUser:example.org";
    let content = r#"{"users":{}}"#;
    let tags = "{}";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("room-power-levels-no-start");
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

    let single =
        rt.block_on(shared.room_set_power_level(room_id.to_owned(), user_id.to_owned(), 50));
    let bulk = rt.block_on(shared.room_set_power_levels(room_id.to_owned(), content.to_owned()));
    let tags_result =
        rt.block_on(shared.room_set_power_level_tags(room_id.to_owned(), tags.to_owned()));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let single_text = single
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted set-power-level must not require a live server");
    let bulk_text = bulk
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted set-power-levels must not require a live server");
    let tags_text = tags_result
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted set-power-level-tags must not require a live server");

    assert!(
        single_text.contains("v-rooms-members-moderation-"),
        "single setter must return a registered owner diagnostic: {single_text}"
    );
    assert!(
        bulk_text.contains("v-rooms-power-levels-"),
        "bulk writer must return a registered owner diagnostic: {bulk_text}"
    );
    assert!(
        tags_text.contains("v-rooms-power-levels-"),
        "tag writer must return a registered owner diagnostic: {tags_text}"
    );
    for (label, text) in [
        ("single", &single_text),
        ("bulk", &bulk_text),
        ("tags", &tags_text),
    ] {
        assert!(
            !text.contains("p4-s9-14-room-power-levels-failed"),
            "{label} must not hide a wrong envelope behind the generic fallback: {text}"
        );
    }
    let text = format!("{single_text}{bulk_text}{tags_text}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(user_id));
    assert!(!text.contains(content));
}
