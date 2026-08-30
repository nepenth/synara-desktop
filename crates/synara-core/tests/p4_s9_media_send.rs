//! Typed SharedCore consume of live `upload_content` / `send_room_attachment`.
//!
//! Bytes are method arguments, never `Core::command` JSON. Leftover
//! `media_upload` stays on SharedCore. Failed errors stay static and must
//! not echo bytes, filename, mime, room id, or tokens.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
use synara_core::transport::MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
use synara_core::{
    IosSecretVault, IosSecretVaultError, MediaUploadError, SendRoomAttachmentError, SharedCore,
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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-media-send-it-{tag}-{nanos}"));
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

fn upload_error_text(error: &MediaUploadError) -> String {
    format!("{error:?}{error}")
}

fn send_error_text(error: &SendRoomAttachmentError) -> String {
    format!("{error:?}{error}")
}

#[test]
fn media_send_surface_exposes_live_owners_and_keeps_leftover_upload() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("upload_content("));
    assert!(udl.contains("send_room_attachment("));
    assert!(udl.contains("string? transaction_id"));
    assert!(udl.contains("sequence<string>? mention_user_ids"));
    assert!(udl.contains("boolean? mention_room"));
    assert!(udl.contains("dictionary MediaUploadDto"));
    assert!(udl.contains("dictionary SendRoomAttachmentDto"));
    assert!(udl.contains("interface MediaUploadError"));
    assert!(udl.contains("interface SendRoomAttachmentError"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(!udl.contains("matrix_upload_media"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("upload_content("));
    assert!(shared_core.contains("send_room_attachment("));
    assert!(shared_core.contains("media_upload("));
    assert!(shared_core.contains("upload_avatar("));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("matrix_backup_status"));
    assert!(!shared_core.contains("send_sticker("));
}

#[test]
fn media_send_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s9MediaSecretRoom:example.org";
    let filename = "s9MediaSecret.bin";
    let mime = "application/octet-stream";
    let marker = "s9MediaSecretBytes";
    let payload = marker.as_bytes().to_vec();

    let upload = rt
        .block_on(shared.upload_content(
            payload.clone(),
            mime.to_owned(),
            Some(filename.to_owned()),
        ))
        .expect_err("no attached content-upload owner");
    let send = rt
        .block_on(shared.send_room_attachment(
            room_id.to_owned(),
            filename.to_owned(),
            mime.to_owned(),
            payload,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ))
        .expect_err("no attached room-attachment owner");
    let leftover = rt
        .block_on(shared.media_upload(
            marker.as_bytes().to_vec(),
            mime.to_owned(),
            filename.to_owned(),
        ))
        .expect_err("leftover upload stays fail-closed");

    let upload_text = upload_error_text(&upload);
    let send_text = send_error_text(&send);
    let leftover_text = format!("{leftover:?}{leftover}");
    assert!(upload_text.contains("p2-upload-content-no-session"));
    assert!(send_text.contains("p2-send-room-attachment-no-session"));
    assert!(leftover_text.contains("p4-s10-leftover-no-session"));
    let text = format!("{upload_text}{send_text}{leftover_text}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(filename));
    assert!(!text.contains(marker));
    assert!(!text.contains("@alice"));
}

#[test]
fn media_send_oversize_mime_and_filename_fail_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s9MediaOversizeRoom:example.org";
    let filename = "s9MediaOversize.bin";
    let mime = format!(
        "application/{}",
        "m".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let oversize_name = format!("{}.bin", "f".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8));
    let payload = b"s9MediaOversizeBytes".to_vec();

    let upload_mime = rt
        .block_on(shared.upload_content(payload.clone(), mime.clone(), Some(filename.to_owned())))
        .expect_err("oversize mime must fail closed");
    let upload_name = rt
        .block_on(shared.upload_content(
            payload.clone(),
            "application/octet-stream".to_owned(),
            Some(oversize_name.clone()),
        ))
        .expect_err("oversize filename must fail closed");
    let send_mime = rt
        .block_on(shared.send_room_attachment(
            room_id.to_owned(),
            filename.to_owned(),
            mime.clone(),
            payload.clone(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ))
        .expect_err("oversize send mime must fail closed");
    let send_name = rt
        .block_on(shared.send_room_attachment(
            room_id.to_owned(),
            oversize_name.clone(),
            "application/octet-stream".to_owned(),
            payload,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ))
        .expect_err("oversize send filename must fail closed");

    let upload_mime_text = upload_error_text(&upload_mime);
    let upload_name_text = upload_error_text(&upload_name);
    let send_mime_text = send_error_text(&send_mime);
    let send_name_text = send_error_text(&send_name);
    assert!(upload_mime_text.contains("p4-s9-media-upload-failed"));
    assert!(upload_name_text.contains("p4-s9-media-upload-failed"));
    assert!(send_mime_text.contains("p4-s9-send-room-attachment-failed"));
    assert!(send_name_text.contains("p4-s9-send-room-attachment-failed"));
    let text = format!("{upload_mime_text}{upload_name_text}{send_mime_text}{send_name_text}");
    assert!(!text.contains(&mime));
    assert!(!text.contains(&oversize_name));
    assert!(!text.contains(filename));
    assert!(!text.contains(room_id));
    assert!(!text.contains("s9MediaOversizeBytes"));
}

#[test]
fn media_send_without_started_sync_returns_owner_diagnostic_without_echo() {
    let access = "syt_s9_media_send_access";
    let refresh = "syr_s9_media_send_refresh";
    let identity = alice();
    let room_id = "!s9MediaSecretRoom:example.org";
    let filename = "s9MediaSecret.bin";
    let mime = "application/octet-stream";
    let marker = "s9MediaSecretBytes";
    let invalid_room = "s9-media-not-a-room-id";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("media-send-no-start");
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

    let upload = rt.block_on(shared.upload_content(
        marker.as_bytes().to_vec(),
        mime.to_owned(),
        Some(filename.to_owned()),
    ));
    let send = rt.block_on(shared.send_room_attachment(
        room_id.to_owned(),
        filename.to_owned(),
        mime.to_owned(),
        marker.as_bytes().to_vec(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    ));
    let invalid_room_send = rt.block_on(shared.send_room_attachment(
        invalid_room.to_owned(),
        filename.to_owned(),
        mime.to_owned(),
        marker.as_bytes().to_vec(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    ));
    let leftover = rt.block_on(shared.media_upload(
        marker.as_bytes().to_vec(),
        mime.to_owned(),
        filename.to_owned(),
    ));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let upload_text = upload
        .as_ref()
        .err()
        .map(upload_error_text)
        .expect("planted upload must fail on live SDK I/O without a live server");
    let send_text = send
        .as_ref()
        .err()
        .map(send_error_text)
        .expect("planted send must fail on local room lookup without a live server");
    let invalid_room_text = invalid_room_send
        .as_ref()
        .err()
        .map(send_error_text)
        .expect("planted send must fail on invalid room id without a live server");
    let leftover_text = leftover
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("leftover upload stays fail-closed");

    assert!(
        upload_text.contains("v-send."),
        "upload must return a registered owner diagnostic: {upload_text}"
    );
    assert!(
        send_text.contains("v-send."),
        "send must return a registered owner diagnostic: {send_text}"
    );
    assert!(
        invalid_room_text.contains("v-send."),
        "invalid room must return a registered owner diagnostic: {invalid_room_text}"
    );
    assert!(
        leftover_text.contains("p4-s10-leftover-unavailable")
            || leftover_text.contains("p4-s10-leftover-no-session"),
        "leftover upload must stay leftover: {leftover_text}"
    );
    assert!(
        !upload_text.contains("p4-s10-leftover-unavailable"),
        "live upload must not return leftover-unavailable: {upload_text}"
    );
    assert!(
        !send_text.contains("p4-s10-leftover-unavailable"),
        "live send must not return leftover-unavailable: {send_text}"
    );
    assert!(
        !upload_text.contains("p4-s9-media-upload-failed"),
        "upload must not hide a wrong envelope behind the generic fallback: {upload_text}"
    );
    assert!(
        !send_text.contains("p4-s9-send-room-attachment-failed"),
        "send must not hide a wrong envelope behind the generic fallback: {send_text}"
    );
    let text = format!("{upload_text}{send_text}{invalid_room_text}{leftover_text}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(filename));
    assert!(!text.contains(marker));
    assert!(!text.contains(invalid_room));
}
