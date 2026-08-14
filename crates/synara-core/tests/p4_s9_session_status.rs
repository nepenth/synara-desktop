//! P4-S9-31: typed SharedCore consume of the registered session/status
//! read commands.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Failed errors stay static and must not echo user id, homeserver, or device id.
//! Backup/crypto/cross-signing/room-key status stay off.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
use synara_core::transport::MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
use synara_core::{
    IosSecretVault, IosSecretVaultError, MediaConfigDto, SecretStorageStatusDto,
    SessionSnapshotDto, SessionStatusError, SharedCore, SyncStatusDto,
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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-31-it-{tag}-{nanos}"));
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

fn snapshot_plain(
    rt: &tokio::runtime::Runtime,
    shared: &SharedCore,
) -> Result<SessionSnapshotDto, SessionStatusError> {
    rt.block_on(shared.session_snapshot())
}

fn sync_plain(
    rt: &tokio::runtime::Runtime,
    shared: &SharedCore,
) -> Result<SyncStatusDto, SessionStatusError> {
    rt.block_on(shared.sync_status())
}

fn media_plain(
    rt: &tokio::runtime::Runtime,
    shared: &SharedCore,
) -> Result<MediaConfigDto, SessionStatusError> {
    rt.block_on(shared.media_config())
}

fn secret_plain(
    rt: &tokio::runtime::Runtime,
    shared: &SharedCore,
) -> Result<SecretStorageStatusDto, SessionStatusError> {
    rt.block_on(shared.secret_storage_status())
}

fn error_text(error: &SessionStatusError) -> String {
    format!("{error:?}{error}")
}

#[test]
fn session_status_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("dictionary SessionSnapshotDto"));
    assert!(udl.contains("dictionary SyncStatusDto"));
    assert!(udl.contains("dictionary MediaConfigDto"));
    assert!(udl.contains("dictionary SecretStorageStatusDto"));
    assert!(udl.contains("interface SessionStatusError"));
    assert!(udl.contains("timeline_forward_text("));
    assert!(udl.contains("timeline_forward_media("));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(!udl.contains("matrix_backup_status"));
    assert!(!udl.contains("matrix_crypto_status"));
    assert!(!udl.contains("matrix_cross_signing_status"));
    assert!(!udl.contains("matrix_cross_signing_setup"));
    assert!(!udl.contains("matrix_room_key_transfer_status"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("session_snapshot("));
    assert!(shared_core.contains("sync_status("));
    assert!(shared_core.contains("media_config("));
    assert!(shared_core.contains("secret_storage_status("));
    assert!(shared_core.contains("timeline_forward_text("));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("backup_status"));
    assert!(!shared_core.contains("crypto_status"));
    assert!(!shared_core.contains("cross_signing_status"));
    assert!(!shared_core.contains("cross_signing_setup"));
    assert!(!shared_core.contains("room_key_transfer_status"));
}

#[test]
fn session_status_family_without_session_returns_handler_result_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let user_id = "@alice:example.org";
    let homeserver = "https://matrix.example.org";
    let device_id = "DEVICEABC";
    let snapshot = snapshot_plain(&rt, &shared).expect("logged-out snapshot is a registered read");
    let sync = sync_plain(&rt, &shared).expect_err("fail-closed platform has no sync status");
    let media = media_plain(&rt, &shared).expect_err("fail-closed platform has no media config");
    let secret =
        secret_plain(&rt, &shared).expect_err("fail-closed platform has no secret-storage status");
    assert_eq!(snapshot.status, "logged_out");
    assert!(snapshot.user_id.is_none());
    assert!(snapshot.device_id.is_none());
    assert!(snapshot.homeserver_url.is_none());
    assert!(snapshot.session_generation.is_none());
    let sync_err = error_text(&sync);
    let media_err = error_text(&media);
    let secret_err = error_text(&secret);
    assert!(sync_err.contains("p2-sync-status-platform-unavailable"));
    assert!(media_err.contains("p2-media-config-no-session"));
    assert!(secret_err.contains("v-crypto.4-secret-storage-requires-session"));
    let combined = format!("{sync_err}{media_err}{secret_err}");
    assert!(!combined.contains("syt_"));
    assert!(!combined.contains("token"));
    assert!(!combined.contains(user_id));
    assert!(!combined.contains(homeserver));
    assert!(!combined.contains(device_id));
    assert!(!combined.contains("p4-s9-31-session-status-failed"));
}

#[test]
fn session_status_oversize_payload_fails_closed_without_truncate_or_echo() {
    let marker = "s931OversizeMarker";
    let oversized = format!(
        "{marker}{}",
        "x".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    assert!(oversized.len() > MAX_ENVELOPE_PAYLOAD_JSON_BYTES);
    let shared = SharedCore::new();
    let rt = test_runtime();
    let snapshot = snapshot_plain(&rt, &shared).expect("null session snapshot stays under 1 MiB");
    let sync =
        sync_plain(&rt, &shared).expect_err("null sync status stays a closed platform error");
    let media =
        media_plain(&rt, &shared).expect_err("null media config stays a closed platform error");
    let secret = secret_plain(&rt, &shared)
        .expect_err("null secret-storage status stays a closed platform error");
    assert_eq!(snapshot.status, "logged_out");
    let combined = format!(
        "{}{}{}",
        error_text(&sync),
        error_text(&media),
        error_text(&secret)
    );
    assert!(!combined.contains(&oversized));
    assert!(!combined.contains(marker));
    assert!(!combined.contains("syt_"));
    assert!(sync.to_string().len() < MAX_ENVELOPE_PAYLOAD_JSON_BYTES);
    assert!(media.to_string().len() < MAX_ENVELOPE_PAYLOAD_JSON_BYTES);
    assert!(secret.to_string().len() < MAX_ENVELOPE_PAYLOAD_JSON_BYTES);
}

#[test]
fn session_status_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_31_session_status_access";
    let refresh = "syr_s9_31_session_status_refresh";
    let identity = alice();
    let user_id = identity.user_id().to_owned();
    let homeserver = identity.homeserver_url().to_owned();
    let device_id = "DEVICEABC";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("session-status-no-start");
    let rt = test_runtime();
    let _enter = rt.enter();
    rt.block_on(shared.persist_planted_session_for_test(
        user_id.clone(),
        homeserver.clone(),
        root.to_string_lossy().into_owned(),
        device_id.to_owned(),
        access.to_owned(),
        Some(refresh.to_owned()),
    ))
    .expect("planted persist");
    rt.block_on(shared.attach_session_owners())
        .expect("owners attached");

    let snapshot = snapshot_plain(&rt, &shared);
    let sync = sync_plain(&rt, &shared);
    let media = media_plain(&rt, &shared);
    let secret = secret_plain(&rt, &shared);
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let snapshot =
        snapshot.expect("planted session_snapshot must return the registered logged-in readback");
    assert_eq!(snapshot.status, "logged_in");
    assert_eq!(snapshot.user_id.as_deref(), Some(user_id.as_str()));
    assert_eq!(snapshot.device_id.as_deref(), Some(device_id));
    assert_eq!(
        snapshot.homeserver_url.as_deref(),
        Some(homeserver.as_str())
    );
    assert_eq!(snapshot.session_generation, Some(1));

    let sync_err = sync
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted sync_status must return the registered iOS platform diagnostic");
    let media_err = media
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted media_config must return the registered iOS platform diagnostic");
    let secret_err =
        secret.as_ref().err().map(error_text).expect(
            "planted secret_storage_status must return the registered iOS platform diagnostic",
        );

    assert!(
        sync_err.contains("p2-sync-status-platform-unavailable"),
        "sync_status must return the registered platform-unavailable diagnostic: {sync_err}"
    );
    assert!(
        media_err.contains("p2-media-config-no-session"),
        "media_config must return the registered no-session diagnostic: {media_err}"
    );
    assert!(
        secret_err.contains("v-crypto.4-secret-storage-requires-session"),
        "secret_storage_status must return the registered requires-session diagnostic: {secret_err}"
    );
    for (label, text) in [
        ("sync", &sync_err),
        ("media", &media_err),
        ("secret", &secret_err),
    ] {
        assert!(
            !text.contains("p4-s9-31-session-status-failed"),
            "{label} must not hide a wrong envelope behind the generic fallback: {text}"
        );
    }
    let combined = format!("{sync_err}{media_err}{secret_err}");
    assert!(!combined.contains(access));
    assert!(!combined.contains(refresh));
    assert!(!combined.contains("syt_"));
    assert!(!combined.contains(&user_id));
    assert!(!combined.contains(&homeserver));
    assert!(!combined.contains(device_id));
}
