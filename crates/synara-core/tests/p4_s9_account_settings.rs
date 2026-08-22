//! Typed SharedCore consume of ignored-users, push-rules, 3PID, and avatar-upload.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Password and client_secret stay off JSON. Failed errors stay static and
//! must not echo user ids, emails, keywords, or passwords.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-account-it-{tag}-{nanos}"));
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
fn account_settings_surface_exposes_the_registered_families() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("ignored_users_snapshot"));
    assert!(udl.contains("ignored_users_ignore"));
    assert!(udl.contains("ignored_users_unignore"));
    assert!(udl.contains("push_rules_snapshot"));
    assert!(udl.contains("push_rules_set_default"));
    assert!(udl.contains("push_rules_set_mention"));
    assert!(udl.contains("push_rules_add_keyword"));
    assert!(udl.contains("push_rules_remove_keyword"));
    assert!(udl.contains("room_notification_snapshot"));
    assert!(udl.contains("room_notification_set"));
    assert!(udl.contains("room_notifications_snapshot"));
    assert!(udl.contains("threepid_snapshot"));
    assert!(udl.contains("threepid_delete"));
    assert!(udl.contains("threepid_request_email_token"));
    assert!(udl.contains("threepid_add_email"));
    assert!(udl.contains("threepid_add_email_password"));
    assert!(udl.contains("upload_avatar"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_upload_media"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("ignored_users_snapshot"));
    assert!(shared_core.contains("push_rules_snapshot"));
    assert!(shared_core.contains("room_notification_snapshot"));
    assert!(shared_core.contains("threepid_snapshot"));
    assert!(shared_core.contains("upload_avatar"));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("matrix_backup_status"));
}

#[test]
fn account_settings_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let user_id = "@spam:example.org";
    let email = "alice-secret@example.org";
    let keyword = "secret-keyword";
    let password = "syt_not_a_real_password";
    let room_id = "!secret-room:example.org";
    let mode = "mute";

    let ignored = rt
        .block_on(shared.ignored_users_snapshot())
        .expect_err("no ignored-users owner");
    let ignore = rt
        .block_on(shared.ignored_users_ignore(user_id.to_owned()))
        .expect_err("no ignored-users owner");
    let push = rt
        .block_on(shared.push_rules_snapshot())
        .expect_err("no push-rules owner");
    let keyword_add = rt
        .block_on(shared.push_rules_add_keyword(keyword.to_owned()))
        .expect_err("no push-rules owner");
    let room_notification = rt
        .block_on(shared.room_notification_snapshot(room_id.to_owned()))
        .expect_err("no room-notification owner");
    let room_notification_set = rt
        .block_on(shared.room_notification_set(room_id.to_owned(), mode.to_owned()))
        .expect_err("no room-notification owner");
    let room_notifications = rt
        .block_on(shared.room_notifications_snapshot())
        .expect_err("no room-notification owner");
    let threepid = rt
        .block_on(shared.threepid_snapshot())
        .expect_err("no 3PID owner");
    let token = rt
        .block_on(shared.threepid_request_email_token(email.to_owned()))
        .expect_err("no 3PID owner");
    let add_password = rt
        .block_on(shared.threepid_add_email_password(password.to_owned()))
        .expect_err("no 3PID owner");
    let upload = rt
        .block_on(shared.upload_avatar(vec![1, 2, 3], "image/jpeg".to_owned()))
        .expect_err("no avatar owner");

    let ignored_text = format!("{ignored:?}{ignored}");
    let ignore_text = format!("{ignore:?}{ignore}");
    let push_text = format!("{push:?}{push}");
    let keyword_text = format!("{keyword_add:?}{keyword_add}");
    let room_notification_text = format!("{room_notification:?}{room_notification}");
    let room_notification_set_text = format!("{room_notification_set:?}{room_notification_set}");
    let room_notifications_text = format!("{room_notifications:?}{room_notifications}");
    let threepid_text = format!("{threepid:?}{threepid}");
    let token_text = format!("{token:?}{token}");
    let password_text = format!("{add_password:?}{add_password}");
    let upload_text = format!("{upload:?}{upload}");

    assert!(ignored_text.contains("p2-ignored-users-snapshot-no-session"));
    assert!(ignore_text.contains("p2-ignored-users-ignore-no-session"));
    assert!(push_text.contains("p2-push-rules-snapshot-no-session"));
    assert!(keyword_text.contains("p2-push-rules-add-keyword-no-session"));
    assert!(room_notification_text.contains("p2-room-notification-snapshot-no-session"));
    assert!(room_notification_set_text.contains("p2-room-notification-set-no-session"));
    assert!(room_notifications_text.contains("p2-room-notifications-snapshot-no-session"));
    assert!(threepid_text.contains("p2-threepid-snapshot-no-session"));
    assert!(token_text.contains("p2-threepid-request-email-token-no-session"));
    assert!(password_text.contains("p2-threepid-add-email-password-no-session"));
    assert!(upload_text.contains("p2-upload-avatar-no-session"));

    let text = format!(
        "{ignored_text}{ignore_text}{push_text}{keyword_text}{room_notification_text}{room_notification_set_text}{room_notifications_text}{threepid_text}{token_text}{password_text}{upload_text}"
    );
    assert!(!text.contains(user_id));
    assert!(!text.contains(email));
    assert!(!text.contains(keyword));
    assert!(!text.contains(room_id));
    assert!(!text.contains(mode));
    assert!(!text.contains(password));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("client_secret"));
}

#[test]
fn account_settings_oversize_payload_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let user_id = "x".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8);
    let error = test_runtime()
        .block_on(shared.ignored_users_ignore(user_id.clone()))
        .expect_err("oversize ignore payload must fail closed");
    let text = format!("{error:?}{error}");
    assert!(text.contains("p4-s9-ignored-users-failed"));
    assert!(!text.contains(&user_id));

    let room_id = "x".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8);
    let room_error = test_runtime()
        .block_on(shared.room_notification_set(room_id.clone(), "mute".to_owned()))
        .expect_err("oversize room-notification payload must fail closed");
    let room_text = format!("{room_error:?}{room_error}");
    assert!(room_text.contains("p4-s9-room-notification-failed"));
    assert!(!room_text.contains(&room_id));
    assert!(!room_text.contains("mute"));
}

#[test]
fn account_settings_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_account_settings_access";
    let refresh = "syr_s9_account_settings_refresh";
    let identity = alice();
    let user_id = "@spam:example.org";
    let email = "alice-secret@example.org";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("account-settings-no-start");
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

    let ignored = rt.block_on(shared.ignored_users_snapshot());
    let ignore = rt.block_on(shared.ignored_users_ignore(user_id.to_owned()));
    let push = rt.block_on(shared.push_rules_snapshot());
    let room_id = "!secret-room:example.org";
    let room_notification = rt.block_on(shared.room_notification_snapshot(room_id.to_owned()));
    let room_notifications = rt.block_on(shared.room_notifications_snapshot());
    let threepid = rt.block_on(shared.threepid_snapshot());
    let token = rt.block_on(shared.threepid_request_email_token(email.to_owned()));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let ignored_text = match &ignored {
        Ok(value) => format!("ok:{}", value.user_ids.len()),
        Err(error) => format!("{error:?}{error}"),
    };
    let ignore_text = ignore
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .unwrap_or_else(|| "ok".to_owned());
    let push_text = match &push {
        Ok(value) => format!("ok:{}", value.keywords.len()),
        Err(error) => format!("{error:?}{error}"),
    };
    let room_notification_text = match &room_notification {
        Ok(value) => format!("ok:{}", value.mode),
        Err(error) => format!("{error:?}{error}"),
    };
    let room_notifications_text = match &room_notifications {
        Ok(value) => format!("ok:{}", value.rooms.len()),
        Err(error) => format!("{error:?}{error}"),
    };
    let threepid_text = match &threepid {
        Ok(value) => format!("ok:{}", value.emails.len()),
        Err(error) => format!("{error:?}{error}"),
    };
    let token_text = token
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .unwrap_or_else(|| "ok".to_owned());

    assert!(
        ignored.is_ok() || ignored_text.contains("v-profile.ignore-"),
        "ignored snapshot must return a registered handler result: {ignored_text}"
    );
    assert!(
        !ignored_text.contains("p4-s9-ignored-users-failed"),
        "ignored snapshot must not hide a wrong envelope: {ignored_text}"
    );
    assert!(
        push.is_ok() || push_text.contains("v-push."),
        "push snapshot must return a registered handler result: {push_text}"
    );
    assert!(
        room_notification.is_ok() || room_notification_text.contains("v-push."),
        "room-notification snapshot must return a registered handler result: {room_notification_text}"
    );
    assert!(
        !room_notification_text.contains("p4-s9-room-notification-failed"),
        "room-notification snapshot must not hide a wrong envelope: {room_notification_text}"
    );
    assert!(
        room_notifications.is_ok() || room_notifications_text.contains("v-push."),
        "room-notifications snapshot must return a registered handler result: {room_notifications_text}"
    );
    assert!(
        threepid.is_ok() || threepid_text.contains("v-threepid."),
        "threepid snapshot must return a registered handler result: {threepid_text}"
    );
    let text = format!("{ignored_text}{ignore_text}{push_text}{room_notification_text}{room_notifications_text}{threepid_text}{token_text}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!ignore_text.contains(user_id));
    assert!(!room_notification_text.contains(room_id));
    assert!(!token_text.contains(email));
}
