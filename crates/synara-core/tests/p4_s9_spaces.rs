//! P4-S9-17: typed SharedCore consume of the six registered space commands.
//!
//! Calls the already-registered Core handlers. Does not start SyncService.
//! Child set/remove carry room ids, via, order, and suggested. No bytes.
//! Failed errors stay static and must not echo room ids. Invite
//! accept/decline stay off. Members snapshots are already S9-16.

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
    let root = std::env::temp_dir().join(format!("synara-p4-s9-17-it-{tag}-{nanos}"));
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
fn spaces_surface_exposes_only_the_registered_family() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("space_parents_snapshot("));
    assert!(udl.contains("space_hierarchy_snapshot("));
    assert!(udl.contains("space_children_snapshot("));
    assert!(udl.contains("space_child_set("));
    assert!(udl.contains("space_child_remove("));
    assert!(udl.contains("restricted_join_reparent("));
    assert!(udl.contains("dictionary SpaceParentsSnapshotDto"));
    assert!(udl.contains("dictionary SpaceHierarchySnapshotDto"));
    assert!(udl.contains("dictionary SpaceChildrenSnapshotDto"));
    assert!(udl.contains("dictionary SpaceChildMutationDto"));
    assert!(udl.contains("dictionary RestrictedJoinReparentDto"));
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
    assert!(shared_core.contains("space_parents_snapshot("));
    assert!(shared_core.contains("space_hierarchy_snapshot("));
    assert!(shared_core.contains("space_children_snapshot("));
    assert!(shared_core.contains("space_child_set("));
    assert!(shared_core.contains("space_child_remove("));
    assert!(shared_core.contains("restricted_join_reparent("));
    assert!(shared_core.contains("room_members_snapshot("));
    assert!(shared_core.contains("room_create("));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("edit_message"));
    assert!(!shared_core.contains("poll_respond"));
    assert!(!shared_core.contains("backup_status"));
}

#[test]
fn spaces_family_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let room_id = "!s917SecretRoom:example.org";
    let parent_id = "!s917SecretParent:example.org";
    let child_id = "!s917SecretChild:example.org";
    let via = "s917.secret.example.org";
    let order = "s917SecretOrder";
    let parents = rt
        .block_on(shared.space_parents_snapshot())
        .expect_err("no attached space-parents-snapshot owner");
    let hierarchy = rt
        .block_on(shared.space_hierarchy_snapshot(room_id.to_owned()))
        .expect_err("no attached space-hierarchy-snapshot owner");
    let children = rt
        .block_on(shared.space_children_snapshot())
        .expect_err("no attached space-children-snapshot owner");
    let child_set = rt
        .block_on(shared.space_child_set(
            parent_id.to_owned(),
            child_id.to_owned(),
            vec![via.to_owned()],
            Some(order.to_owned()),
            Some(true),
        ))
        .expect_err("no attached space-child-set owner");
    let child_remove = rt
        .block_on(shared.space_child_remove(parent_id.to_owned(), child_id.to_owned()))
        .expect_err("no attached space-child-remove owner");
    let reparent = rt
        .block_on(shared.restricted_join_reparent(
            room_id.to_owned(),
            Some(parent_id.to_owned()),
            child_id.to_owned(),
        ))
        .expect_err("no attached restricted-join-reparent owner");
    let parents_text = format!("{parents:?}{parents}");
    let hierarchy_text = format!("{hierarchy:?}{hierarchy}");
    let children_text = format!("{children:?}{children}");
    let child_set_text = format!("{child_set:?}{child_set}");
    let child_remove_text = format!("{child_remove:?}{child_remove}");
    let reparent_text = format!("{reparent:?}{reparent}");
    assert!(parents_text.contains("p2-space-parents-snapshot-no-session"));
    assert!(hierarchy_text.contains("p2-space-hierarchy-snapshot-no-session"));
    assert!(children_text.contains("p2-space-children-snapshot-no-session"));
    assert!(child_set_text.contains("p2-space-child-set-no-session"));
    assert!(child_remove_text.contains("p2-space-child-remove-no-session"));
    assert!(reparent_text.contains("p2-restricted-join-reparent-no-session"));
    let text = format!(
        "{parents_text}{hierarchy_text}{children_text}{child_set_text}{child_remove_text}{reparent_text}"
    );
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(parent_id));
    assert!(!text.contains(child_id));
    assert!(!text.contains(via));
    assert!(!text.contains(order));
    assert!(!text.contains("@alice"));
}

#[test]
fn spaces_oversize_payload_fails_closed_without_truncate_or_echo() {
    let shared = SharedCore::new();
    let room_id = format!(
        "!{}:example.org",
        "s".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let parent_id = format!(
        "!{}:example.org",
        "p".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let child_id = format!(
        "!{}:example.org",
        "c".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let via = format!(
        "{}.example.org",
        "v".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let rt = test_runtime();
    let hierarchy = rt
        .block_on(shared.space_hierarchy_snapshot(room_id.clone()))
        .expect_err("oversize hierarchy-snapshot payload must fail closed");
    let child_set = rt
        .block_on(shared.space_child_set(
            parent_id.clone(),
            child_id.clone(),
            vec![via.clone()],
            None,
            None,
        ))
        .expect_err("oversize child-set payload must fail closed");
    let child_remove = rt
        .block_on(shared.space_child_remove(parent_id.clone(), child_id.clone()))
        .expect_err("oversize child-remove payload must fail closed");
    let reparent = rt
        .block_on(shared.restricted_join_reparent(room_id.clone(), None, parent_id.clone()))
        .expect_err("oversize reparent payload must fail closed");
    let hierarchy_text = format!("{hierarchy:?}{hierarchy}");
    let child_set_text = format!("{child_set:?}{child_set}");
    let child_remove_text = format!("{child_remove:?}{child_remove}");
    let reparent_text = format!("{reparent:?}{reparent}");
    assert!(hierarchy_text.contains("p4-s9-17-spaces-failed"));
    assert!(child_set_text.contains("p4-s9-17-spaces-failed"));
    assert!(child_remove_text.contains("p4-s9-17-spaces-failed"));
    assert!(reparent_text.contains("p4-s9-17-spaces-failed"));
    assert!(!hierarchy_text.contains(&room_id));
    assert!(!child_set_text.contains(&parent_id));
    assert!(!child_set_text.contains(&child_id));
    assert!(!child_set_text.contains(&via));
    assert!(!child_remove_text.contains(&parent_id));
    assert!(!reparent_text.contains(&room_id));
}

#[test]
fn spaces_family_without_started_sync_returns_handler_result_without_echo() {
    let access = "syt_s9_17_spaces_access";
    let refresh = "syr_s9_17_spaces_refresh";
    let identity = alice();
    let room_id = "!s917SecretRoom:example.org";
    let parent_id = "!s917SecretParent:example.org";
    let child_id = "!s917SecretChild:example.org";
    let invalid_hierarchy = "s917-not-a-room-id";
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("spaces-no-start");
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

    let parents = rt.block_on(shared.space_parents_snapshot());
    let children = rt.block_on(shared.space_children_snapshot());
    let hierarchy = rt.block_on(shared.space_hierarchy_snapshot(invalid_hierarchy.to_owned()));
    let child_set = rt.block_on(shared.space_child_set(
        parent_id.to_owned(),
        child_id.to_owned(),
        Vec::new(),
        None,
        None,
    ));
    let child_remove =
        rt.block_on(shared.space_child_remove(parent_id.to_owned(), child_id.to_owned()));
    let reparent = rt.block_on(shared.restricted_join_reparent(
        room_id.to_owned(),
        None,
        parent_id.to_owned(),
    ));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let parents_ok = parents
        .as_ref()
        .ok()
        .expect("planted parents snapshot is a local joined-room walk");
    assert!(parents_ok.entries.is_empty());
    let children_ok = children
        .as_ref()
        .ok()
        .expect("planted children snapshot is a local joined-room walk");
    assert!(children_ok.edges.is_empty());

    let hierarchy_text = hierarchy
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted hierarchy must fail on invalid room id without a live server");
    let child_set_text = child_set
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted child-set must fail on local room lookup");
    let child_remove_text = child_remove
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted child-remove must fail on local room lookup");
    let reparent_text = reparent
        .as_ref()
        .err()
        .map(|error| format!("{error:?}{error}"))
        .expect("planted reparent must fail on local room lookup");

    assert!(
        hierarchy_text.contains("v-rooms.2b-space-hierarchy-invalid-room"),
        "hierarchy must return the registered invalid-room diagnostic: {hierarchy_text}"
    );
    for (label, text) in [
        ("child_set", &child_set_text),
        ("child_remove", &child_remove_text),
        ("reparent", &reparent_text),
    ] {
        assert!(
            text.contains("v-rooms.2c-"),
            "{label} must return a registered owner diagnostic: {text}"
        );
        assert!(
            !text.contains("p4-s9-17-spaces-failed"),
            "{label} must not hide a wrong envelope behind the generic fallback: {text}"
        );
    }
    assert!(!hierarchy_text.contains("p4-s9-17-spaces-failed"));
    let text = format!("{hierarchy_text}{child_set_text}{child_remove_text}{reparent_text}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
    assert!(!text.contains(room_id));
    assert!(!text.contains(parent_id));
    assert!(!text.contains(child_id));
    assert!(!text.contains(invalid_hierarchy));
}
