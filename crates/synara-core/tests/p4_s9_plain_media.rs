//! Typed SharedCore consume of live `download_plain_media` / `thumbnail_plain_media`.
//!
//! Bytes are method returns, mxc is a method argument, never `Core::command`
//! JSON. Leftover `media_download` / `media_thumbnail` stay on SharedCore.
//! Failed errors stay static and must not echo mxc or tokens.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
use synara_core::{IosSecretVault, IosSecretVaultError, PlainMediaError, SharedCore};

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-plain-media-it-{tag}-{nanos}"));
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

fn plain_error_text(error: &PlainMediaError) -> String {
    format!("{error:?}{error}")
}

#[test]
fn plain_media_surface_exposes_live_owners_and_keeps_leftover_download() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("download_plain_media("));
    assert!(udl.contains("thumbnail_plain_media("));
    assert!(udl.contains("dictionary MediaBytesDto"));
    assert!(udl.contains("interface PlainMediaError"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(!udl.contains("matrix_media_download"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("download_plain_media("));
    assert!(shared_core.contains("thumbnail_plain_media("));
    assert!(shared_core.contains("media_download("));
    assert!(shared_core.contains("media_thumbnail("));
    assert!(shared_core.contains("timeline_media_bytes("));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("matrix_backup_status"));
}

#[test]
fn plain_media_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let mxc = "mxc://example.org/s9PlainSecretMedia";

    let download = rt
        .block_on(shared.download_plain_media(mxc.to_owned()))
        .expect_err("no attached plain-media owner");
    let thumbnail = rt
        .block_on(shared.thumbnail_plain_media(mxc.to_owned(), 96, 96))
        .expect_err("no attached plain-media owner");
    let leftover = rt
        .block_on(shared.media_download(mxc.to_owned()))
        .expect_err("leftover download stays fail-closed");

    let download_text = plain_error_text(&download);
    let thumbnail_text = plain_error_text(&thumbnail);
    let leftover_text = format!("{leftover:?}{leftover}");
    assert!(download_text.contains("p2-download-plain-media-no-session"));
    assert!(thumbnail_text.contains("p2-thumbnail-plain-media-no-session"));
    assert!(leftover_text.contains("p4-s10-leftover-no-session"));
    let text = format!("{download_text}{thumbnail_text}{leftover_text}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(mxc));
    assert!(!text.contains("@alice"));
}

#[test]
fn plain_media_without_started_sync_returns_owner_diagnostic_without_echo() {
    let access = "syt_s9_plain_media_access";
    let refresh = "syr_s9_plain_media_refresh";
    let identity = alice();
    let mxc = "mxc://example.org/s9PlainSecretMedia";
    let handle = "timeline-media-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("plain-media-no-start");
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

    let download = rt.block_on(shared.download_plain_media(mxc.to_owned()));
    let thumbnail = rt.block_on(shared.thumbnail_plain_media(mxc.to_owned(), 96, 96));
    let handle_download = rt.block_on(shared.download_plain_media(handle.to_owned()));
    let leftover = rt.block_on(shared.media_download(mxc.to_owned()));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let download_text = download
        .as_ref()
        .err()
        .map(plain_error_text)
        .expect("planted download must fail on live SDK I/O without a live server");
    let thumbnail_text = thumbnail
        .as_ref()
        .err()
        .map(plain_error_text)
        .expect("planted thumbnail must fail on live SDK I/O without a live server");
    let handle_text = handle_download
        .as_ref()
        .err()
        .map(plain_error_text)
        .expect("timeline-media handles must stay off this path");
    let leftover_text = leftover
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("leftover download stays fail-closed");

    assert!(
        download_text.contains("v-send.r-media-"),
        "download must return a registered owner diagnostic: {download_text}"
    );
    assert!(
        thumbnail_text.contains("v-send.r-media-"),
        "thumbnail must return a registered owner diagnostic: {thumbnail_text}"
    );
    assert!(
        handle_text.contains("v-send.r-media-"),
        "timeline-media handle must return a registered owner diagnostic: {handle_text}"
    );
    assert!(
        leftover_text.contains("p4-s10-leftover-unavailable")
            || leftover_text.contains("p4-s10-leftover-no-session"),
        "leftover download must stay leftover: {leftover_text}"
    );
    assert!(
        !download_text.contains("p4-s10-leftover-unavailable"),
        "live download must not return leftover-unavailable: {download_text}"
    );
    assert!(
        !thumbnail_text.contains("p4-s10-leftover-unavailable"),
        "live thumbnail must not return leftover-unavailable: {thumbnail_text}"
    );
    assert!(
        !download_text.contains("p4-s9-plain-media-failed"),
        "download must not hide a wrong envelope behind the generic fallback: {download_text}"
    );
    assert!(
        !thumbnail_text.contains("p4-s9-plain-media-failed"),
        "thumbnail must not hide a wrong envelope behind the generic fallback: {thumbnail_text}"
    );
    let text = format!("{download_text}{thumbnail_text}{handle_text}{leftover_text}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(mxc));
    assert!(!text.contains(handle));
}
