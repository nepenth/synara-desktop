//! P4-S9-16: typed SharedCore consume of the four registered members snapshots.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Room id may cross as the method argument. These are reads. Failed errors
//! stay static and must not echo room id or member user ids. Spaces stay
//! off. Power-level writers and room create are already S9-14 / S9-15.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-16-it-{tag}-{nanos}"));
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
fn room_members_snapshots_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("room_members_snapshot("));
    assert!(udl.contains("room_power_levels_snapshot("));
    assert!(udl.contains("room_creators_snapshot("));
    assert!(udl.contains("room_power_level_tags_snapshot("));
    assert!(udl.contains("dictionary RoomMembersSnapshotDto"));
    assert!(udl.contains("dictionary RoomPowerLevelsSnapshotDto"));
    assert!(udl.contains("dictionary RoomCreatorsSnapshotDto"));
    assert!(udl.contains("dictionary RoomPowerLevelTagsSnapshotDto"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(!udl.contains("matrix_send_sticker"));
    assert!(!udl.contains("matrix_send_poll"));
    assert!(!udl.contains("matrix_edit_message"));
    assert!(!udl.contains("matrix_poll_respond"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("room_members_snapshot("));
    assert!(shared_core.contains("room_power_levels_snapshot("));
    assert!(shared_core.contains("room_creators_snapshot("));
    assert!(shared_core.contains("room_power_level_tags_snapshot("));
    assert!(shared_core.contains("room_create("));
    assert!(shared_core.contains("room_set_power_level("));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("poll_respond"));
    assert!(!shared_core.contains("backup_status"));
}

#[test]
fn room_members_snapshots_family_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s916SecretRoom:example.org";
    let member = "@s916SecretMember:example.org";
    let members = rt
        .block_on(shared.room_members_snapshot(room_id.to_owned()))
        .expect_err("no attached room-members-snapshot owner");
    let power_levels = rt
        .block_on(shared.room_power_levels_snapshot(room_id.to_owned()))
        .expect_err("no attached room-power-levels-snapshot owner");
    let creators = rt
        .block_on(shared.room_creators_snapshot(room_id.to_owned()))
        .expect_err("no attached room-creators-snapshot owner");
    let tags = rt
        .block_on(shared.room_power_level_tags_snapshot(room_id.to_owned()))
        .expect_err("no attached room-power-level-tags-snapshot owner");
    let members_text = format!("{members:?}{members}");
    let power_levels_text = format!("{power_levels:?}{power_levels}");
    let creators_text = format!("{creators:?}{creators}");
    let tags_text = format!("{tags:?}{tags}");
    assert!(members_text.contains("p2-room-members-snapshot-no-session"));
    assert!(power_levels_text.contains("p2-room-power-levels-snapshot-no-session"));
    assert!(creators_text.contains("p2-room-creators-snapshot-no-session"));
    assert!(tags_text.contains("p2-room-power-level-tags-snapshot-no-session"));
    let text = format!("{members_text}{power_levels_text}{creators_text}{tags_text}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(member));
    assert!(!text.contains("@alice"));
}

#[test]
fn room_members_snapshots_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let room_id = format!(
        "!{}:example.org",
        "s".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let rt = test_runtime();
    let members = rt
        .block_on(shared.room_members_snapshot(room_id.clone()))
        .expect_err("oversize members-snapshot payload must fail closed");
    let power_levels = rt
        .block_on(shared.room_power_levels_snapshot(room_id.clone()))
        .expect_err("oversize power-levels-snapshot payload must fail closed");
    let creators = rt
        .block_on(shared.room_creators_snapshot(room_id.clone()))
        .expect_err("oversize creators-snapshot payload must fail closed");
    let tags = rt
        .block_on(shared.room_power_level_tags_snapshot(room_id.clone()))
        .expect_err("oversize power-level-tags-snapshot payload must fail closed");
    let members_text = format!("{members:?}{members}");
    let power_levels_text = format!("{power_levels:?}{power_levels}");
    let creators_text = format!("{creators:?}{creators}");
    let tags_text = format!("{tags:?}{tags}");
    assert!(members_text.contains("p4-s9-16-room-members-snapshots-failed"));
    assert!(power_levels_text.contains("p4-s9-16-room-members-snapshots-failed"));
    assert!(creators_text.contains("p4-s9-16-room-members-snapshots-failed"));
    assert!(tags_text.contains("p4-s9-16-room-members-snapshots-failed"));
    assert!(!members_text.contains(&room_id));
    assert!(!power_levels_text.contains(&room_id));
    assert!(!creators_text.contains(&room_id));
    assert!(!tags_text.contains(&room_id));
}

#[test]
fn room_members_snapshots_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_16_room_members_access";
    let refresh = "syr_s9_16_room_members_refresh";
    let identity = alice();
    let room_id = "!s916SecretRoom:example.org";
    let member = "@s916SecretMember:example.org";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("room-members-no-start");
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

    let members = rt.block_on(shared.room_members_snapshot(room_id.to_owned()));
    let power_levels = rt.block_on(shared.room_power_levels_snapshot(room_id.to_owned()));
    let creators = rt.block_on(shared.room_creators_snapshot(room_id.to_owned()));
    let tags = rt.block_on(shared.room_power_level_tags_snapshot(room_id.to_owned()));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let members_text = members
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted members snapshot must not require a live server");
    let power_levels_text = power_levels
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted power-levels snapshot must not require a live server");
    let creators_text = creators
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted creators snapshot must not require a live server");
    let tags_text = tags
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted power-level-tags snapshot must not require a live server");

    for (label, text) in [
        ("members", &members_text),
        ("power_levels", &power_levels_text),
        ("creators", &creators_text),
        ("tags", &tags_text),
    ] {
        assert!(
            text.contains("v-rooms-members-read-"),
            "{label} must return a registered owner diagnostic: {text}"
        );
        assert!(
            !text.contains("p4-s9-16-room-members-snapshots-failed"),
            "{label} must not hide a wrong envelope behind the generic fallback: {text}"
        );
    }
    let text = format!("{members_text}{power_levels_text}{creators_text}{tags_text}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(member));
}
