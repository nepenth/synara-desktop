//! P4-S9-13: typed SharedCore consume of the four registered room moderation commands.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Room id, user id, and optional reason may cross as method arguments. Write
//! ack is status only. Failed errors stay static and must not echo room id,
//! user id, or reason. Power levels, room create, members, and spaces stay
//! off. Leave/join is already S9-12.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-13-it-{tag}-{nanos}"));
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
fn room_moderation_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("room_invite"));
    assert!(udl.contains("room_kick"));
    assert!(udl.contains("room_ban"));
    assert!(udl.contains("room_unban"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(!udl.contains("matrix_room_create"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("room_invite"));
    assert!(shared_core.contains("room_kick"));
    assert!(shared_core.contains("room_ban"));
    assert!(shared_core.contains("room_unban"));
    assert!(shared_core.contains("room_leave"));
    assert!(shared_core.contains("room_join("));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("invites_accept"));
    assert!(!shared_core.contains("backup_status"));
}

#[test]
fn room_moderation_family_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s913SecretRoom:example.org";
    let user_id = "@s913SecretUser:example.org";
    let reason = "s913SecretReason";
    let invite = rt
        .block_on(shared.room_invite(
            room_id.to_owned(),
            user_id.to_owned(),
            Some(reason.to_owned()),
        ))
        .expect_err("no attached room-moderation owner");
    let kick = rt
        .block_on(shared.room_kick(
            room_id.to_owned(),
            user_id.to_owned(),
            Some(reason.to_owned()),
        ))
        .expect_err("no attached room-moderation owner");
    let ban = rt
        .block_on(shared.room_ban(
            room_id.to_owned(),
            user_id.to_owned(),
            Some(reason.to_owned()),
        ))
        .expect_err("no attached room-moderation owner");
    let unban = rt
        .block_on(shared.room_unban(room_id.to_owned(), user_id.to_owned()))
        .expect_err("no attached room-moderation owner");
    let invite_text = format!("{invite:?}{invite}");
    let kick_text = format!("{kick:?}{kick}");
    let ban_text = format!("{ban:?}{ban}");
    let unban_text = format!("{unban:?}{unban}");
    assert!(invite_text.contains("p2-room-invite-no-session"));
    assert!(kick_text.contains("p2-room-kick-no-session"));
    assert!(ban_text.contains("p2-room-ban-no-session"));
    assert!(unban_text.contains("p2-room-unban-no-session"));
    let text = format!("{invite_text}{kick_text}{ban_text}{unban_text}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(user_id));
    assert!(!text.contains(reason));
    assert!(!text.contains("@alice"));
}

#[test]
fn room_moderation_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let room_id = format!(
        "!{}:example.org",
        "s".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let user_id = format!(
        "@{}:example.org",
        "u".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let reason = "r".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8);
    let rt = test_runtime();
    let invite = rt
        .block_on(shared.room_invite(room_id.clone(), user_id.clone(), Some(reason.clone())))
        .expect_err("oversize room-invite payload must fail closed");
    let kick = rt
        .block_on(shared.room_kick(room_id.clone(), user_id.clone(), Some(reason.clone())))
        .expect_err("oversize room-kick payload must fail closed");
    let ban = rt
        .block_on(shared.room_ban(room_id.clone(), user_id.clone(), Some(reason.clone())))
        .expect_err("oversize room-ban payload must fail closed");
    let unban = rt
        .block_on(shared.room_unban(room_id.clone(), user_id.clone()))
        .expect_err("oversize room-unban payload must fail closed");
    let invite_text = format!("{invite:?}{invite}");
    let kick_text = format!("{kick:?}{kick}");
    let ban_text = format!("{ban:?}{ban}");
    let unban_text = format!("{unban:?}{unban}");
    assert!(invite_text.contains("p4-s9-13-room-moderation-failed"));
    assert!(kick_text.contains("p4-s9-13-room-moderation-failed"));
    assert!(ban_text.contains("p4-s9-13-room-moderation-failed"));
    assert!(unban_text.contains("p4-s9-13-room-moderation-failed"));
    assert!(!invite_text.contains(&room_id));
    assert!(!invite_text.contains(&user_id));
    assert!(!invite_text.contains(&reason));
    assert!(!kick_text.contains(&room_id));
    assert!(!ban_text.contains(&user_id));
    assert!(!unban_text.contains(&room_id));
}

#[test]
fn room_moderation_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_13_room_moderation_access";
    let refresh = "syr_s9_13_room_moderation_refresh";
    let identity = alice();
    let room_id = "!s913SecretRoom:example.org";
    let user_id = "@s913SecretUser:example.org";
    let reason = "s913SecretReason";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("room-moderation-no-start");
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

    let invite = rt.block_on(shared.room_invite(
        room_id.to_owned(),
        user_id.to_owned(),
        Some(reason.to_owned()),
    ));
    let kick = rt.block_on(shared.room_kick(
        room_id.to_owned(),
        user_id.to_owned(),
        Some(reason.to_owned()),
    ));
    let ban = rt.block_on(shared.room_ban(
        room_id.to_owned(),
        user_id.to_owned(),
        Some(reason.to_owned()),
    ));
    let unban = rt.block_on(shared.room_unban(room_id.to_owned(), user_id.to_owned()));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let invite_text = invite
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted invite must not require a live server");
    let kick_text = kick
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted kick must not require a live server");
    let ban_text = ban
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted ban must not require a live server");
    let unban_text = unban
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted unban must not require a live server");

    for (label, text) in [
        ("invite", &invite_text),
        ("kick", &kick_text),
        ("ban", &ban_text),
        ("unban", &unban_text),
    ] {
        assert!(
            text.contains("v-rooms-members-moderation-"),
            "{label} must return a registered owner diagnostic: {text}"
        );
        assert!(
            !text.contains("p4-s9-13-room-moderation-failed"),
            "{label} must not hide a wrong envelope behind the generic fallback: {text}"
        );
    }
    let text = format!("{invite_text}{kick_text}{ban_text}{unban_text}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(user_id));
    assert!(!text.contains(reason));
}
