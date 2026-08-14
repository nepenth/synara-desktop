//! P4-S9-4: typed SharedCore consume of the six registered image-pack commands.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Pack metadata/IDs/URLs/JSON may cross. Image/media bytes stay off.
//! Own display-name/avatar and leftover secret envelopes stay off.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-4-it-{tag}-{nanos}"));
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
fn image_pack_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("get_global_image_packs"));
    assert!(udl.contains("get_user_image_pack"));
    assert!(udl.contains("get_room_image_packs"));
    assert!(udl.contains("set_user_image_pack"));
    assert!(udl.contains("set_global_image_packs"));
    assert!(udl.contains("set_room_image_pack"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(!udl.contains("matrix_upload_media"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("get_global_image_packs"));
    assert!(shared_core.contains("get_user_image_pack"));
    assert!(shared_core.contains("get_room_image_packs"));
    assert!(shared_core.contains("set_user_image_pack"));
    assert!(shared_core.contains("set_global_image_packs"));
    assert!(shared_core.contains("set_room_image_pack"));
    assert!(shared_core.contains("room_join_rule_snapshot"));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("room_create"));
    assert!(!shared_core.contains("backup_status"));
}

#[test]
fn image_pack_family_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s94pack:example.org";
    let state_key = "s94state";
    let content =
        r#"{"pack":{"display_name":"S94"},"images":{"smile":{"url":"mxc://example.org/abc"}}}"#;
    let global = rt
        .block_on(shared.get_global_image_packs())
        .expect_err("no attached image-pack owner");
    let user = rt
        .block_on(shared.get_user_image_pack())
        .expect_err("no attached image-pack owner");
    let room = rt
        .block_on(shared.get_room_image_packs(room_id.to_owned()))
        .expect_err("no attached image-pack owner");
    let set_user = rt
        .block_on(shared.set_user_image_pack(content.to_owned()))
        .expect_err("no attached image-pack owner");
    let set_global = rt
        .block_on(shared.set_global_image_packs(content.to_owned()))
        .expect_err("no attached image-pack owner");
    let set_room = rt
        .block_on(shared.set_room_image_pack(
            room_id.to_owned(),
            state_key.to_owned(),
            content.to_owned(),
        ))
        .expect_err("no attached image-pack owner");
    let global_text = format!("{global:?}{global}");
    let user_text = format!("{user:?}{user}");
    let room_text = format!("{room:?}{room}");
    let set_user_text = format!("{set_user:?}{set_user}");
    let set_global_text = format!("{set_global:?}{set_global}");
    let set_room_text = format!("{set_room:?}{set_room}");
    assert!(global_text.contains("p2-global-image-packs-no-session"));
    assert!(user_text.contains("p2-user-image-pack-no-session"));
    assert!(room_text.contains("p2-room-image-packs-no-session"));
    assert!(set_user_text.contains("p2-set-user-image-pack-no-session"));
    assert!(set_global_text.contains("p2-set-global-image-packs-no-session"));
    assert!(set_room_text.contains("p2-set-room-image-pack-no-session"));
    let text = format!(
        "{global_text}{user_text}{room_text}{set_user_text}{set_global_text}{set_room_text}"
    );
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(state_key));
    assert!(!text.contains("mxc://"));
    assert!(!text.contains(content));
}

#[test]
fn image_pack_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_4_pack_access";
    let refresh = "syr_s9_4_pack_refresh";
    let identity = alice();
    let room_id = "!s94pack:example.org";
    let state_key = "s94state";
    let content =
        r#"{"pack":{"display_name":"S94"},"images":{"smile":{"url":"mxc://example.org/abc"}}}"#;
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("image-packs-no-start");
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

    let global = rt.block_on(shared.get_global_image_packs());
    let user = rt.block_on(shared.get_user_image_pack());
    let room = rt.block_on(shared.get_room_image_packs(room_id.to_owned()));
    let set_user = rt.block_on(shared.set_user_image_pack(content.to_owned()));
    let set_global = rt.block_on(shared.set_global_image_packs(r#"{"rooms":{}}"#.to_owned()));
    let set_room = rt.block_on(shared.set_room_image_pack(
        room_id.to_owned(),
        state_key.to_owned(),
        content.to_owned(),
    ));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let global_text = match &global {
        Ok(snapshot) => format!("ok:{}", snapshot.packs.len()),
        Err(error) => format!("{error:?}{error}"),
    };
    let user_text = match &user {
        Ok(snapshot) => format!("ok:{}", snapshot.pack.is_some()),
        Err(error) => format!("{error:?}{error}"),
    };
    let room_text = room
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered room-pack handler");
    let set_user_text = set_user
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered set-user handler");
    let set_global_text = set_global
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered set-global handler");
    let set_room_text = set_room
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("unstarted sync still uses the registered set-room handler");

    assert!(
        global.is_ok() || global_text.contains("v-send.r-pack-"),
        "global snapshot must return the registered handler result: {global_text}"
    );
    assert!(
        !global_text.contains("p4-s9-4-image-pack-failed"),
        "global snapshot must not hide a wrong envelope behind the generic fallback: {global_text}"
    );
    assert!(
        user.is_ok() || user_text.contains("v-send.r-pack-"),
        "user snapshot must return the registered handler result: {user_text}"
    );
    assert!(
        !user_text.contains("p4-s9-4-image-pack-failed"),
        "user snapshot must not hide a wrong envelope behind the generic fallback: {user_text}"
    );
    assert!(
        room_text.contains("v-send.r-pack-"),
        "room snapshot must return a registered owner diagnostic: {room_text}"
    );
    assert!(
        !room_text.contains("p4-s9-4-image-pack-failed"),
        "room snapshot must not hide a wrong envelope behind the generic fallback: {room_text}"
    );
    assert!(
        set_user_text.contains("v-send.r-pack-"),
        "set_user must return a registered owner diagnostic: {set_user_text}"
    );
    assert!(
        set_global_text.contains("v-send.r-pack-"),
        "set_global must return a registered owner diagnostic: {set_global_text}"
    );
    assert!(
        set_room_text.contains("v-send.r-pack-"),
        "set_room must return a registered owner diagnostic: {set_room_text}"
    );
    let text = format!(
        "{global_text}{user_text}{room_text}{set_user_text}{set_global_text}{set_room_text}"
    );
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(state_key));
    assert!(!text.contains("mxc://"));
    assert!(!text.contains(content));
}
