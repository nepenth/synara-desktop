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

fn close_plain(rt: &tokio::runtime::Runtime, shared: &SharedCore) -> Result<(), NseStoreError> {
    rt.block_on(shared.nse_close_read_only_store())
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
    assert!(udl.contains("void nse_close_read_only_store()"));
    assert!(udl.contains("NseEventPreviewDto nse_resolve_event_preview("));
    assert!(udl.contains("NseEventPreviewDto nse_event_preview("));
    assert!(udl.contains("constructor();"));
    assert!(udl.contains("[Name=\"new_with_secret_store\"]"));
    assert!(!udl.contains("SharedCore(store:)"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(crate::ffi_surface::shared_core_declares(
        shared_core,
        "nse_open_read_only_store"
    ));
    assert!(crate::ffi_surface::shared_core_declares(
        shared_core,
        "nse_store_status"
    ));
    assert!(crate::ffi_surface::shared_core_declares(
        shared_core,
        "nse_close_read_only_store"
    ));
    assert!(crate::ffi_surface::shared_core_declares(
        shared_core,
        "nse_resolve_event_preview"
    ));
    assert!(crate::ffi_surface::shared_core_declares(
        shared_core,
        "nse_event_preview"
    ));
    assert!(crate::ffi_surface::shared_core_declares(
        shared_core,
        "secret_storage_status"
    ));
    assert!(!crate::ffi_surface::shared_core_declares(
        shared_core,
        "command"
    ));
    assert!(shared_core.contains("start_sync()"));
    assert!(shared_core.contains("poll_timeline_view_updates()"));
    assert!(shared_core.contains("poll_owner_updates()"));
    assert!(shared_core.contains("poll_room_list_updates()"));
    assert!(!shared_core.contains("sync_start"));
    assert!(!shared_core.contains("build_sync_service"));
    assert!(!shared_core.contains("matrix_login_password"));
    assert!(!shared_core.contains("matrix_send_attachment"));
    assert!(!shared_core.contains("matrix_backup_status"));
    assert!(!shared_core.contains("matrix_crypto_status"));

    let (ffi, _) = include_str!("../src/shared_core_ffi.rs")
        .split_once("\n#[cfg(test)]\nmod tests;")
        .expect("SharedCore FFI production/test module boundary");
    assert!(ffi.contains("mod nse_preview;\npub use nse_preview::*;"));
    assert!(ffi.contains("mod session_lifecycle;\npub use session_lifecycle::*;"));
    let nse_domain = include_str!("../src/shared_core_ffi/nse_preview.rs");
    let lifecycle = include_str!("../src/shared_core_ffi/session_lifecycle.rs");
    // These exact wired domains contain production only. Fail closed if inline
    // test code is later added rather than treating its tokens as implementation.
    assert!(!nse_domain.contains("#[cfg(test)]"));
    assert!(!lifecycle.contains("#[cfg(test)]"));
    let (_, nse) = nse_domain
        .split_once("pub async fn nse_open_read_only_store(")
        .expect("NSE domain methods");
    assert!(nse.contains("nse_store_status"));
    assert!(nse.contains("nse_close_read_only_store"));
    assert!(nse.contains("nse_event_preview"));
    assert!(!nse_domain.contains("build_sync_service"));
    assert!(!nse_domain.contains("attach_session_owners"));
    assert!(!nse_domain.contains("start_sync"));
    assert!(!nse_domain.contains("poll_timeline_view_updates"));
    assert!(!nse_domain.contains("poll_owner_updates"));
    assert!(!nse_domain.contains("poll_room_list_updates"));
    assert!(!nse_domain.contains(".start()"));
    assert!(nse.contains("NotificationClient::new"));
    assert!(nse.contains("NotificationProcessSetup::MultipleProcesses"));
    assert!(lifecycle.contains("with_cross_process_store_lock_holder"));
    assert!(
        lifecycle.contains("config.with_cross_process_store_lock_holder(NSE_STORE_LOCK_HOLDER)")
    );
    assert!(ffi.contains("NSE_STORE_LOCK_HOLDER: &str = \"synara-nse-parent\""));
    assert!(nse.contains("get_notification"));
    assert!(nse.contains("NSE_RESOLUTION_TIMEOUT"));
    assert!(ffi.contains("p4-s11-nse-restore-failed"));
    // Initialization and fetch now preserve closed typed causes. Verify the
    // real NSE call sites are wired to that production mapper, rather than
    // requiring obsolete generic constants in the broad FFI module.
    let (diagnostics, _) = include_str!("../src/app/notifications/nse_error.rs")
        .split_once("\n#[cfg(test)]\n")
        .expect("NSE diagnostic production/test module boundary");
    assert!(nse.contains("nse_notification_initialization_error_code("));
    assert!(nse.contains("nse_notification_error_code(&error)"));
    assert!(diagnostics.contains("p4-s11-nse-client-init-failed"));
    assert!(diagnostics.contains("p4-s11-nse-event-fetch-failed"));
    assert!(diagnostics.contains("p4-s11-nse-room-unavailable"));
    assert!(ffi.contains("p4-s11-nse-resolution-timeout"));
    assert!(ffi.contains("p4-s11-nse-close-failed"));
    assert!(nse.contains("nse_resolve_event_preview"));
    assert!(nse.contains("self.nse_open_read_only_store_with_room("));
    assert!(nse.contains("room_id.map(RoomLoadSettings::One)"));
    assert!(nse.contains("Some(parsed_room)"));
    assert!(nse.contains("self.nse_event_preview_unbounded(room_id, event_id)"));
    assert!(nse.contains("NotificationEvent::Timeline"));
    assert!(!nse_domain.contains("RawNotificationEvent::Timeline"));
    assert!(nse_domain.contains("store_key_for_read_only"));
    assert!(lifecycle.contains("store_key_for_read_only(&self.secret_store, &identity)?"));
    assert!(lifecycle.contains("handle_refresh_tokens = false"));
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
fn nse_store_planted_session_cannot_start_product_sync_and_fails_closed() {
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
    let closed = close_plain(&rt, &shared);
    let status_after_close = status_plain(&rt, &shared);
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

    let preview_error = preview
        .as_ref()
        .expect_err("planted store has no notification event");
    // The pinned SDK cannot resolve the planted room and returns UnknownRoom.
    // That may mask an earlier notification-sync failure; it must remain an
    // exact static cause, never a preview or a claim about account membership.
    let NseStoreError::Failed { code, description } = preview_error;
    assert_eq!(code, "p4-s11-nse-room-unavailable");
    assert_eq!(
        description,
        "The NSE notification event could not be fetched."
    );
    let preview_err = error_text(preview_error);

    let attach_err = attach.expect_err("NSE read-only store must refuse owner attach");
    let attach_text = format!("{attach_err:?}{attach_err}");
    assert!(
        attach_text.contains("p4-s11-nse-read-only-forbids-attach"),
        "attach must return the NSE read-only diagnostic: {attach_text}"
    );
    closed.expect("NSE close must drop the retained client on the async runtime");
    assert!(status_after_close.is_err());

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
