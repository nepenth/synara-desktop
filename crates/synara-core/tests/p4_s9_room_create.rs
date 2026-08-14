//! P4-S9-15: typed SharedCore consume of the registered room-create command.
//!
//! Calls the already-registered Core handler. Does not start SyncService.
//! Name, topic, alias, visibility, preset, and Core scalar extras may cross
//! as the typed request. creation_content, power_level_content_override,
//! paths, passphrases, and media bytes stay off. Success returns the created
//! room id. Failed errors stay static and must not echo name, topic, alias,
//! invite, or parent. Members snapshots and spaces stay off. Power levels
//! are already S9-14.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
use synara_core::transport::MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
use synara_core::{IosSecretVault, IosSecretVaultError, RoomCreateRequestDto, SharedCore};

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-15-it-{tag}-{nanos}"));
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

fn create_request(
    name: Option<&str>,
    topic: Option<&str>,
    alias: Option<&str>,
    invite: Vec<&str>,
    parent: Option<&str>,
) -> RoomCreateRequestDto {
    RoomCreateRequestDto {
        name: name.map(ToOwned::to_owned),
        topic: topic.map(ToOwned::to_owned),
        room_alias_name: alias.map(ToOwned::to_owned),
        visibility: Some("private".to_owned()),
        preset: Some("private_chat".to_owned()),
        is_direct: false,
        encryption: false,
        invite: invite.into_iter().map(ToOwned::to_owned).collect(),
        room_version: None,
        join_rule: None,
        knock: false,
        parent_room_id: parent.map(ToOwned::to_owned),
    }
}

#[test]
fn room_create_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("room_create("));
    assert!(udl.contains("dictionary RoomCreateRequestDto"));
    assert!(udl.contains("dictionary RoomCreateDto"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(!udl.contains("matrix_timeline_reaction_toggle"));
    let request_dto = udl
        .split("dictionary RoomCreateRequestDto {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("RoomCreateRequestDto");
    assert!(!request_dto.contains("creation_content"));
    assert!(!request_dto.contains("power_level_content_override"));
    assert!(!request_dto.contains("password"));
    assert!(!request_dto.contains("passphrase"));
    assert!(!request_dto.contains("bytes"));
    assert!(!request_dto.contains("path"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("room_create("));
    assert!(shared_core.contains("room_set_power_level("));
    assert!(shared_core.contains("room_set_power_level_tags("));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("reaction_toggle"));
    assert!(!shared_core.contains("reaction_ensure"));
    assert!(!shared_core.contains("backup_status"));
}

#[test]
fn room_create_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let name = "s915SecretName";
    let topic = "s915SecretTopic";
    let alias = "s915secretalias";
    let invite = "s915SecretInvite";
    let parent = "!s915SecretParent:example.org";
    let error = rt
        .block_on(shared.room_create(create_request(
            Some(name),
            Some(topic),
            Some(alias),
            vec![invite],
            Some(parent),
        )))
        .expect_err("no attached room-create owner");
    let text = format!("{error:?}{error}");
    assert!(text.contains("p2-room-create-no-session"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(name));
    assert!(!text.contains(topic));
    assert!(!text.contains(alias));
    assert!(!text.contains(invite));
    assert!(!text.contains(parent));
    assert!(!text.contains("@alice"));
}

#[test]
fn room_create_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let name = "n".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8);
    let topic = "t".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8);
    let alias = "a".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8);
    let invite = format!(
        "@{}:example.org",
        "i".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let rt = test_runtime();
    let named = rt
        .block_on(shared.room_create(create_request(
            Some(&name),
            Some(&topic),
            Some(&alias),
            Vec::new(),
            None,
        )))
        .expect_err("oversize room-create payload must fail closed");
    let invited = rt
        .block_on(shared.room_create(create_request(None, None, None, vec![&invite], None)))
        .expect_err("oversize room-create invite list must fail closed");
    let named_text = format!("{named:?}{named}");
    let invited_text = format!("{invited:?}{invited}");
    assert!(named_text.contains("p4-s9-15-room-create-failed"));
    assert!(invited_text.contains("p4-s9-15-room-create-failed"));
    assert!(!named_text.contains(&name));
    assert!(!named_text.contains(&topic));
    assert!(!named_text.contains(&alias));
    assert!(!invited_text.contains(&invite));
}

#[test]
fn room_create_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_15_room_create_access";
    let refresh = "syr_s9_15_room_create_refresh";
    let identity = alice();
    let name = "s915SecretName";
    let topic = "s915SecretTopic";
    let alias = "s915secretalias";
    let invite = "s915SecretInvite";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("room-create-no-start");
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

    let created = rt.block_on(shared.room_create(create_request(
        Some(name),
        Some(topic),
        Some(alias),
        vec![invite],
        None,
    )));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let text = created
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted room-create must not require a live server");
    assert!(
        text.contains("v-rooms-room-create-"),
        "create must return a registered owner diagnostic: {text}"
    );
    assert!(
        !text.contains("p4-s9-15-room-create-failed"),
        "create must not hide a wrong envelope behind the generic fallback: {text}"
    );
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(name));
    assert!(!text.contains(topic));
    assert!(!text.contains(alias));
    assert!(!text.contains(invite));
}
