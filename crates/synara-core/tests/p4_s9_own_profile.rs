//! P4-S9-8: typed SharedCore consume of the two registered own-profile commands.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Display name and `mxc://` (or empty clear) may cross as method arguments.
//! Image/media bytes stay off. Failed errors stay static and must not echo
//! display name or mxc. Room name/topic/avatar and leftover secret envelopes
//! stay off.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-8-it-{tag}-{nanos}"));
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
fn own_profile_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("set_own_display_name"));
    assert!(udl.contains("set_own_avatar"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(!udl.contains("matrix_upload_media"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("set_own_display_name"));
    assert!(shared_core.contains("set_own_avatar"));
    assert!(shared_core.contains("room_notes_snapshot"));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("set_room_name"));
    assert!(!shared_core.contains("set_room_topic"));
    assert!(!shared_core.contains("set_room_avatar"));
    assert!(!shared_core.contains("backup_status"));
}

#[test]
fn own_profile_family_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let display_name = "S98 Secret Display Name";
    let mxc = "mxc://example.org/s98SecretAvatarId";
    let display = rt
        .block_on(shared.set_own_display_name(display_name.to_owned()))
        .expect_err("no attached own-profile owner");
    let avatar = rt
        .block_on(shared.set_own_avatar(mxc.to_owned()))
        .expect_err("no attached own-profile owner");
    let display_text = format!("{display:?}{display}");
    let avatar_text = format!("{avatar:?}{avatar}");
    assert!(display_text.contains("p2-set-own-display-name-no-session"));
    assert!(avatar_text.contains("p2-set-own-avatar-no-session"));
    let text = format!("{display_text}{avatar_text}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(display_name));
    assert!(!text.contains(mxc));
    assert!(!text.contains("@alice"));
}

#[test]
fn own_profile_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let display_name = "x".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8);
    let mxc = format!(
        "mxc://example.org/{}",
        "y".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let rt = test_runtime();
    let display = rt
        .block_on(shared.set_own_display_name(display_name.clone()))
        .expect_err("oversize display-name payload must fail closed");
    let avatar = rt
        .block_on(shared.set_own_avatar(mxc.clone()))
        .expect_err("oversize avatar payload must fail closed");
    let display_text = format!("{display:?}{display}");
    let avatar_text = format!("{avatar:?}{avatar}");
    assert!(display_text.contains("p4-s9-8-own-profile-failed"));
    assert!(avatar_text.contains("p4-s9-8-own-profile-failed"));
    assert!(!display_text.contains(&display_name));
    assert!(!avatar_text.contains(&mxc));
}

#[test]
fn own_profile_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_8_own_profile_access";
    let refresh = "syr_s9_8_own_profile_refresh";
    let identity = alice();
    let display_name = "S98 Secret Display Name";
    let mxc = "mxc://example.org/s98SecretAvatarId";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("own-profile-no-start");
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

    let display = rt.block_on(shared.set_own_display_name(display_name.to_owned()));
    let avatar = rt.block_on(shared.set_own_avatar(mxc.to_owned()));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let display_text = display
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered display-name handler");
    let avatar_text = avatar
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered avatar handler");

    assert!(
        display_text.contains("v-send.r-avatar-"),
        "display name must return a registered owner diagnostic: {display_text}"
    );
    assert!(
        avatar_text.contains("v-send.r-avatar-"),
        "avatar must return a registered owner diagnostic: {avatar_text}"
    );
    assert!(
        !display_text.contains("p4-s9-8-own-profile-failed"),
        "display name must not hide a wrong envelope behind the generic fallback: {display_text}"
    );
    assert!(
        !avatar_text.contains("p4-s9-8-own-profile-failed"),
        "avatar must not hide a wrong envelope behind the generic fallback: {avatar_text}"
    );
    let text = format!("{display_text}{avatar_text}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(display_name));
    assert!(!text.contains(mxc));
}
