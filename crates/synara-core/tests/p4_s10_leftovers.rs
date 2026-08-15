//! P4-S10 leftover UniFFI: retire product MatrixRustSDK callers.
//!
//! Status wrappers call already-registered Core commands. Dedicated leftover
//! methods take secrets/bytes as arguments only. Failed errors stay static
//! and must not echo recovery keys, event bodies, MXC URLs, or tokens.
//! Planted leftover I/O does not hit a live homeserver.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::transport::MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
use synara_core::{IosSecretVault, IosSecretVaultError, LeftoverCommandError, SharedCore};

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

fn test_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn error_text(error: &LeftoverCommandError) -> String {
    format!("{error:?}{error}")
}

fn temp_root(tag: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("synara-p4-s10-it-{tag}-{nanos}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn leftover_surface_exposes_the_authorized_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("dictionary BackupStatusDto"));
    assert!(udl.contains("dictionary CryptoStatusDto"));
    assert!(udl.contains("dictionary CrossSigningStatusDto"));
    assert!(udl.contains("dictionary RoomKeyTransferStatusDto"));
    assert!(udl.contains("dictionary LeftoverAckDto"));
    assert!(udl.contains("dictionary LeftoverBytesDto"));
    assert!(udl.contains("interface LeftoverCommandError"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("backup_status("));
    assert!(shared_core.contains("crypto_status("));
    assert!(shared_core.contains("cross_signing_status("));
    assert!(shared_core.contains("room_key_transfer_status("));
    assert!(shared_core.contains("wipe_persisted_stores("));
    assert!(shared_core.contains("logout("));
    assert!(shared_core.contains("recover("));
    assert!(shared_core.contains("send_raw_room_event("));
    assert!(shared_core.contains("set_notification_mode("));
    assert!(shared_core.contains("media_download("));
    assert!(shared_core.contains("media_thumbnail("));
    assert!(shared_core.contains("media_upload("));
    assert!(shared_core.contains("room_avatar_bytes("));
    assert!(shared_core.contains("pusher_set("));
    assert!(shared_core.contains("pusher_delete("));
    assert!(!shared_core.contains("matrix_login_password"));
    assert!(!shared_core.contains("matrix_send_attachment"));
    assert!(!shared_core.contains("command("));
}

#[test]
fn leftover_commands_without_session_fail_closed_without_echo() {
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::new(
        Mutex::new(HashMap::new()),
    ))));
    let rt = test_runtime();
    let recovery_key = "s10-it-recovery-key";
    let room_id = "!s10ItRoom:example.org";
    let event_body = "s10-it-secret-body";
    let mxc = "mxc://example.org/s10ItMedia";
    let push_key = "s10-it-push-key";

    let recover = rt
        .block_on(shared.recover(recovery_key.to_owned()))
        .expect_err("recover");
    let recover_text = error_text(&recover);
    assert!(recover_text.contains("p4-s10-leftover-unavailable"));
    assert!(!recover_text.contains(recovery_key));

    let raw = rt
        .block_on(shared.send_raw_room_event(
            room_id.to_owned(),
            "m.room.message".to_owned(),
            event_body.to_owned(),
        ))
        .expect_err("raw send");
    let raw_text = error_text(&raw);
    assert!(raw_text.contains("p4-s10-leftover-no-session"));
    assert!(!raw_text.contains(room_id));
    assert!(!raw_text.contains(event_body));

    let media = rt
        .block_on(shared.media_download(mxc.to_owned()))
        .expect_err("media");
    let media_text = error_text(&media);
    assert!(media_text.contains("p4-s10-leftover-no-session"));
    assert!(!media_text.contains(mxc));

    let pusher = rt
        .block_on(shared.pusher_set(
            push_key.to_owned(),
            "com.whylandcreative.synara".to_owned(),
            "https://push.example.org".to_owned(),
            "Synara".to_owned(),
            "DEVICE".to_owned(),
            "en-US".to_owned(),
        ))
        .expect_err("pusher");
    let pusher_text = error_text(&pusher);
    assert!(pusher_text.contains("p4-s10-leftover-no-session"));
    assert!(!pusher_text.contains(push_key));
}

#[test]
fn leftover_oversize_media_upload_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let marker = "s10ItOversize";
    let payload = format!(
        "{marker}{}",
        "x".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    )
    .into_bytes();
    let error = rt
        .block_on(shared.media_upload(
            payload,
            "application/octet-stream".to_owned(),
            "secret.bin".to_owned(),
        ))
        .expect_err("oversize upload");
    let text = error_text(&error);
    assert!(text.contains("p4-s10-leftover-oversize"));
    assert!(!text.contains(marker));
    assert!(!text.contains("secret.bin"));
}

#[test]
fn leftover_wipe_and_logout_are_local_only() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let root = temp_root("wipe");
    fs::create_dir_all(root.join("data")).unwrap();
    let wipe = rt
        .block_on(shared.wipe_persisted_stores(root.to_string_lossy().into_owned()))
        .expect("wipe");
    assert_eq!(wipe.status, "wiped");
    assert!(!root.exists());
    let logout = rt.block_on(shared.logout()).expect("logout");
    assert_eq!(logout.status, "logged_out");
}
