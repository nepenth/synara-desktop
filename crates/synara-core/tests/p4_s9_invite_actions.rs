//! P4-S9-18: typed SharedCore consume of the four registered invite actions.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Does not re-wrap S5 `invites_snapshot`. Room id may cross as the method
//! argument. Failed errors stay static and must not echo room id or sender
//! id. Timeline jump and read-state stay off.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-18-it-{tag}-{nanos}"));
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
fn invite_actions_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("invites_accept("));
    assert!(udl.contains("invites_decline("));
    assert!(udl.contains("invites_report_spam("));
    assert!(udl.contains("invites_block_sender("));
    assert!(udl.contains("dictionary InviteSnapshotDto"));
    assert!(udl.contains("interface InviteActionError"));
    assert!(udl.contains("invites_snapshot()"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(!udl.contains("matrix_send_sticker"));
    assert!(!udl.contains("matrix_send_poll"));
    assert!(!udl.contains("matrix_edit_message"));
    assert!(!udl.contains("matrix_poll_respond"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("invites_accept("));
    assert!(shared_core.contains("invites_decline("));
    assert!(shared_core.contains("invites_report_spam("));
    assert!(shared_core.contains("invites_block_sender("));
    assert!(shared_core.contains("invites_snapshot()"));
    assert!(shared_core.contains("space_parents_snapshot("));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("matrix_backup_status"));
}

#[test]
fn invite_actions_family_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s918SecretRoom:example.org";
    let sender_id = "@s918SecretSender:example.org";
    let accept = rt
        .block_on(shared.invites_accept(room_id.to_owned()))
        .expect_err("no attached invites-accept owner");
    let decline = rt
        .block_on(shared.invites_decline(room_id.to_owned()))
        .expect_err("no attached invites-decline owner");
    let spam = rt
        .block_on(shared.invites_report_spam(room_id.to_owned()))
        .expect_err("no attached invites-report-spam owner");
    let block = rt
        .block_on(shared.invites_block_sender(room_id.to_owned()))
        .expect_err("no attached invites-block-sender owner");
    let accept_text = format!("{accept:?}{accept}");
    let decline_text = format!("{decline:?}{decline}");
    let spam_text = format!("{spam:?}{spam}");
    let block_text = format!("{block:?}{block}");
    assert!(accept_text.contains("p2-invites-accept-no-session"));
    assert!(decline_text.contains("p2-invites-decline-no-session"));
    assert!(spam_text.contains("p2-invites-report-spam-no-session"));
    assert!(block_text.contains("p2-invites-block-sender-no-session"));
    let text = format!("{accept_text}{decline_text}{spam_text}{block_text}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(sender_id));
    assert!(!text.contains("@alice"));
}

#[test]
fn invite_actions_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let room_id = format!(
        "!{}:example.org",
        "s".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let rt = test_runtime();
    let accept = rt
        .block_on(shared.invites_accept(room_id.clone()))
        .expect_err("oversize accept payload must fail closed");
    let decline = rt
        .block_on(shared.invites_decline(room_id.clone()))
        .expect_err("oversize decline payload must fail closed");
    let spam = rt
        .block_on(shared.invites_report_spam(room_id.clone()))
        .expect_err("oversize report-spam payload must fail closed");
    let block = rt
        .block_on(shared.invites_block_sender(room_id.clone()))
        .expect_err("oversize block-sender payload must fail closed");
    let accept_text = format!("{accept:?}{accept}");
    let decline_text = format!("{decline:?}{decline}");
    let spam_text = format!("{spam:?}{spam}");
    let block_text = format!("{block:?}{block}");
    assert!(accept_text.contains("p4-s9-18-invite-actions-failed"));
    assert!(decline_text.contains("p4-s9-18-invite-actions-failed"));
    assert!(spam_text.contains("p4-s9-18-invite-actions-failed"));
    assert!(block_text.contains("p4-s9-18-invite-actions-failed"));
    assert!(!accept_text.contains(&room_id));
    assert!(!decline_text.contains(&room_id));
    assert!(!spam_text.contains(&room_id));
    assert!(!block_text.contains(&room_id));
}

#[test]
fn invite_actions_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_18_invite_actions_access";
    let refresh = "syr_s9_18_invite_actions_refresh";
    let identity = alice();
    let room_id = "!s918SecretRoom:example.org";
    let sender_id = "@s918SecretSender:example.org";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("invite-actions-no-start");
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

    let accept = rt.block_on(shared.invites_accept(room_id.to_owned()));
    let decline = rt.block_on(shared.invites_decline(room_id.to_owned()));
    let spam = rt.block_on(shared.invites_report_spam(room_id.to_owned()));
    let block = rt.block_on(shared.invites_block_sender(room_id.to_owned()));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let accept_text = accept
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted accept must fail on local invite lookup without a live server");
    let decline_text = decline
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted decline must fail on local invite lookup without a live server");
    let spam_text = spam
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted report-spam must fail on local invite lookup without a live server");
    let block_text = block
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted block-sender must fail on local invite lookup without a live server");

    for (label, text) in [
        ("accept", &accept_text),
        ("decline", &decline_text),
        ("spam", &spam_text),
        ("block", &block_text),
    ] {
        assert!(
            text.contains("v-rooms.1-invite-not-found"),
            "{label} must return the registered invite-not-found diagnostic: {text}"
        );
        assert!(
            !text.contains("p4-s9-18-invite-actions-failed"),
            "{label} must not hide a wrong envelope behind the generic fallback: {text}"
        );
    }
    let text = format!("{accept_text}{decline_text}{spam_text}{block_text}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(sender_id));
}
