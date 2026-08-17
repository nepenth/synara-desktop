//! P4-S11: NSE read-only store API on caller-owned SharedCore.
//!
//! Opens the persisted store and looks up a local event preview. Does not
//! start SyncService, attach owners, or boot leftover Client sync.
//! Failed errors stay static and must not echo room id, event id, user id,
//! homeserver, or tokens.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
use synara_core::transport::MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
use synara_core::{
    IosSecretVault, IosSecretVaultError, NseEventPreviewDto, NseStoreDto, NseStoreError, SharedCore,
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
    let root = std::env::temp_dir().join(format!("synara-p4-s11-it-{tag}-{nanos}"));
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

fn open_plain(
    rt: &tokio::runtime::Runtime,
    shared: &SharedCore,
    user_id: String,
    homeserver: String,
    store_root: String,
) -> Result<NseStoreDto, NseStoreError> {
    rt.block_on(shared.nse_open_read_only_store(user_id, homeserver, store_root))
}

fn status_plain(
    rt: &tokio::runtime::Runtime,
    shared: &SharedCore,
) -> Result<NseStoreDto, NseStoreError> {
    rt.block_on(shared.nse_store_status())
}

fn preview_plain(
    rt: &tokio::runtime::Runtime,
    shared: &SharedCore,
    room_id: String,
    event_id: String,
) -> Result<NseEventPreviewDto, NseStoreError> {
    rt.block_on(shared.nse_event_preview(room_id, event_id))
}

fn error_text(error: &NseStoreError) -> String {
    format!("{error:?}{error}")
}

#[test]
fn nse_store_surface_is_read_only_and_cannot_start_sync() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("dictionary NseStoreDto"));
    assert!(udl.contains("dictionary NseEventPreviewDto"));
    assert!(udl.contains("interface NseStoreError"));
    assert!(udl.contains("NseStoreDto nse_open_read_only_store("));
    assert!(udl.contains("NseStoreDto nse_store_status()"));
    assert!(udl.contains("NseEventPreviewDto nse_event_preview("));
    assert!(udl.contains("constructor();"));
    assert!(udl.contains("[Name=\"new_with_secret_store\"]"));
    assert!(!udl.contains("SharedCore(store:)"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("nse_open_read_only_store("));
    assert!(shared_core.contains("nse_store_status("));
    assert!(shared_core.contains("nse_event_preview("));
    assert!(shared_core.contains("secret_storage_status("));
    assert!(!shared_core.contains("command("));
    assert!(shared_core.contains("start_sync()"));
    assert!(!shared_core.contains("sync_start"));
    assert!(!shared_core.contains("build_sync_service"));
    assert!(!shared_core.contains("matrix_login_password"));
    assert!(!shared_core.contains("matrix_send_attachment"));
    assert!(!shared_core.contains("matrix_backup_status"));
    assert!(!shared_core.contains("matrix_crypto_status"));

    let ffi = include_str!("../src/shared_core_ffi.rs");
    let nse = ffi
        .split("pub async fn nse_open_read_only_store(")
        .nth(1)
        .and_then(|rest| rest.split("async fn space_null_command(").next())
        .expect("nse methods");
    assert!(nse.contains("nse_store_status"));
    assert!(nse.contains("nse_event_preview"));
    assert!(!nse.contains("build_sync_service"));
    assert!(!nse.contains("attach_session_owners"));
    assert!(!nse.contains("start_sync"));
    assert!(!nse.contains(".start()"));
    assert!(!nse.contains("sliding_sync"));
    assert!(nse.contains("Room::event"));
    assert!(nse.contains("Never fetches"));
}

#[test]
fn nse_store_without_session_returns_owner_diagnostics_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let user_id = "@alice:example.org";
    let homeserver = "https://matrix.example.org";
    let device_id = "DEVICEABC";
    let room_id = "!s11SecretRoom:example.org";
    let event_id = "$s11SecretEvent:example.org";
    let root = temp_root("nse-no-session");

    let status = status_plain(&rt, &shared).expect_err("status requires an open NSE store");
    let preview = preview_plain(&rt, &shared, room_id.to_owned(), event_id.to_owned())
        .expect_err("preview requires an open NSE store");
    let opened = open_plain(
        &rt,
        &shared,
        user_id.to_owned(),
        homeserver.to_owned(),
        root.to_string_lossy().into_owned(),
    )
    .expect_err("fail-closed vault cannot open an NSE store");
    drop(shared);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let status_err = error_text(&status);
    let preview_err = error_text(&preview);
    let open_err = error_text(&opened);
    assert!(status_err.contains("p4-s11-nse-store-not-open"));
    assert!(preview_err.contains("p4-s11-nse-store-not-open"));
    assert!(open_err.contains("p4-s3b-secret-vault-unavailable"));
    let combined = format!("{status_err}{preview_err}{open_err}");
    assert!(!combined.contains("syt_"));
    assert!(!combined.contains("token"));
    assert!(!combined.contains(user_id));
    assert!(!combined.contains(homeserver));
    assert!(!combined.contains(device_id));
    assert!(!combined.contains(room_id));
    assert!(!combined.contains(event_id));
    assert!(!combined.contains("p4-s11-nse-store-failed"));
}

#[test]
fn nse_store_oversize_payload_fails_closed_without_truncate_or_echo() {
    let marker = "s11OversizeMarker";
    let oversized = format!(
        "{marker}{}",
        "x".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    assert!(oversized.len() > MAX_ENVELOPE_PAYLOAD_JSON_BYTES);
    let shared = SharedCore::new();
    let rt = test_runtime();
    let preview = preview_plain(&rt, &shared, oversized.clone(), oversized.clone())
        .expect_err("oversize NSE preview must fail closed");
    let text = error_text(&preview);
    assert!(text.contains("p4-s11-nse-payload-oversize"));
    assert!(!text.contains(&oversized));
    assert!(!text.contains(marker));
    assert!(!text.contains("syt_"));
    assert!(preview.to_string().len() < MAX_ENVELOPE_PAYLOAD_JSON_BYTES);
}

#[test]
fn nse_store_planted_session_cannot_start_sync_and_returns_owner_diagnostics() {
    let access = "syt_s11_nse_store_access";
    let refresh = "syr_s11_nse_store_refresh";
    let identity = alice();
    let user_id = identity.user_id().to_owned();
    let homeserver = identity.homeserver_url().to_owned();
    let device_id = "DEVICEABC";
    let room_id = "!s11PlantedRoom:example.org";
    let event_id = "$s11PlantedEvent:example.org";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("nse-planted");
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

    let opened = open_plain(
        &rt,
        &shared,
        user_id.clone(),
        homeserver.clone(),
        root.to_string_lossy().into_owned(),
    );
    let status = status_plain(&rt, &shared);
    let preview = preview_plain(&rt, &shared, room_id.to_owned(), event_id.to_owned());
    let attach = rt.block_on(shared.attach_session_owners());
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let opened = opened.expect("planted NSE open must adopt the retained client");
    assert!(opened.read_only);
    assert!(!opened.owners_attached);
    assert!(!opened.sync_started);

    let status = status.expect("open NSE store must report status");
    assert!(status.read_only);
    assert!(!status.owners_attached);
    assert!(!status.sync_started);

    let preview_err = preview
        .as_ref()
        .err()
        .map(error_text)
        .expect("planted store has no notification event");
    assert!(
        preview_err.contains("p4-s11-nse-event-not-in-store"),
        "preview must return the registered not-in-store diagnostic: {preview_err}"
    );
    assert!(
        !preview_err.contains("p4-s11-nse-store-failed"),
        "preview must not hide a wrong path behind the generic fallback: {preview_err}"
    );

    let attach_err = attach.expect_err("NSE read-only store must refuse owner attach");
    let attach_text = format!("{attach_err:?}{attach_err}");
    assert!(
        attach_text.contains("p4-s11-nse-read-only-forbids-attach"),
        "attach must return the NSE read-only diagnostic: {attach_text}"
    );

    let combined = format!("{preview_err}{attach_text}");
    assert!(!combined.contains(access));
    assert!(!combined.contains(refresh));
    assert!(!combined.contains("syt_"));
    assert!(!combined.contains(&user_id));
    assert!(!combined.contains(&homeserver));
    assert!(!combined.contains(device_id));
    assert!(!combined.contains(room_id));
    assert!(!combined.contains(event_id));
}
