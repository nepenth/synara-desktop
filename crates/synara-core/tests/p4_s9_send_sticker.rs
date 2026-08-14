//! P4-S9-23: typed SharedCore consume of the registered send-sticker command.
//!
//! Calls the already-registered Core handler. Does not start SyncService.
//! Metadata / mxc only. No image bytes or file path.
//! Failed errors stay static and must not echo mxc or room id.
//! Timeline edit/redact/report stay off.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
use synara_core::transport::MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
use synara_core::{
    IosSecretVault, IosSecretVaultError, SendStickerDto, SendStickerError, SharedCore,
};

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-23-it-{tag}-{nanos}"));
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

fn send_plain(
    rt: &tokio::runtime::Runtime,
    shared: &SharedCore,
    room_id: String,
    body: String,
    mxc: String,
) -> Result<SendStickerDto, SendStickerError> {
    rt.block_on(shared.send_sticker(room_id, body, mxc, None, None, None, None, None, None))
}

fn error_text(error: &SendStickerError) -> String {
    format!("{error:?}{error}")
}

#[test]
fn send_sticker_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("send_sticker("));
    assert!(udl.contains("dictionary SendStickerDto"));
    assert!(udl.contains("interface SendStickerError"));
    assert!(udl.contains("send_text("));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(!udl.contains("matrix_send_poll"));
    assert!(!udl.contains("matrix_edit_message"));
    assert!(!udl.contains("matrix_poll_respond"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("send_sticker("));
    assert!(shared_core.contains("send_text("));
    assert!(shared_core.contains("composer_set_reply_draft("));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("backup_status"));
}

#[test]
fn send_sticker_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s923SecretRoom:example.org";
    let body = "s923SecretBody";
    let mxc = "mxc://example.org/s923SecretMxc";
    let error = send_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        body.to_owned(),
        mxc.to_owned(),
    )
    .expect_err("no attached send-sticker owner");
    let text = error_text(&error);
    assert!(text.contains("p2-send-sticker-no-session"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(body));
    assert!(!text.contains(mxc));
    assert!(!text.contains("@alice"));
}

#[test]
fn send_sticker_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s923OversizeRoom:example.org";
    let body = "s923OversizeBody";
    let mxc = format!(
        "mxc://example.org/{}",
        "m".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let error = send_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        body.to_owned(),
        mxc.clone(),
    )
    .expect_err("oversize send-sticker payload must fail closed");
    let text = error_text(&error);
    assert!(text.contains("p4-s9-23-send-sticker-failed"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(body));
    assert!(!text.contains(&mxc));
    assert!(!text.contains("s923SecretMxc"));
}

#[test]
fn send_sticker_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_23_send_sticker_access";
    let refresh = "syr_s9_23_send_sticker_refresh";
    let identity = alice();
    let room_id = "!s923SecretRoom:example.org";
    let body = "s923SecretBody";
    let mxc = "mxc://example.org/s923SecretMxc";
    let invalid_room = "s923-not-a-room-id";
    let invalid_mxc = "s923-not-an-mxc";
    let invalid_body = "";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("send-sticker-no-start");
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

    let missing_room = send_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        body.to_owned(),
        mxc.to_owned(),
    );
    let invalid_room_send = send_plain(
        &rt,
        &shared,
        invalid_room.to_owned(),
        body.to_owned(),
        mxc.to_owned(),
    );
    let invalid_mxc_send = send_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        body.to_owned(),
        invalid_mxc.to_owned(),
    );
    let invalid_body_send = send_plain(
        &rt,
        &shared,
        room_id.to_owned(),
        invalid_body.to_owned(),
        mxc.to_owned(),
    );
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let missing_room_text = missing_room
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted send must fail on local room lookup without a live server");
    let invalid_room_text = invalid_room_send
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted send must fail on invalid room id without a live server");
    let invalid_mxc_text = invalid_mxc_send
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted send must fail on invalid mxc without a live server");
    let invalid_body_text = invalid_body_send
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted send must fail on invalid body without a live server");

    assert!(
        missing_room_text.contains("v-send-sticker-room-not-found"),
        "send must return the registered room-not-found diagnostic: {missing_room_text}"
    );
    assert!(
        invalid_room_text.contains("d0.4-send-invalid-room-id"),
        "send must return the registered invalid-room diagnostic: {invalid_room_text}"
    );
    assert!(
        invalid_mxc_text.contains("v-send-sticker-invalid-mxc"),
        "send must return the registered invalid-mxc diagnostic: {invalid_mxc_text}"
    );
    assert!(
        invalid_body_text.contains("v-send-sticker-invalid-body"),
        "send must return the registered invalid-body diagnostic: {invalid_body_text}"
    );
    for (label, text) in [
        ("missing_room", &missing_room_text),
        ("invalid_room", &invalid_room_text),
        ("invalid_mxc", &invalid_mxc_text),
        ("invalid_body", &invalid_body_text),
    ] {
        assert!(
            !text.contains("p4-s9-23-send-sticker-failed"),
            "{label} must not hide a wrong envelope behind the generic fallback: {text}"
        );
    }
    let text =
        format!("{missing_room_text}{invalid_room_text}{invalid_mxc_text}{invalid_body_text}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(body));
    assert!(!text.contains(mxc));
    assert!(!text.contains(invalid_room));
    assert!(!text.contains(invalid_mxc));
}
