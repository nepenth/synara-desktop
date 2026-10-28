//! SharedCore facade regression tests.

use super::*;
use crate::app::lifecycle::persist_session_material;
use crate::app::store::StoreKeyId;
use crate::transport::MatrixIpcErrorCategory;
use std::collections::HashMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn account_data_wakeup_does_not_misroute_approval_history_to_image_packs() {
    assert_eq!(
        super::account_data_owner_update_family(
            crate::app::account_data::NativeAccountDataWakeupKind::ImagePacks
        ),
        Some("image_packs")
    );
    assert_eq!(
        super::account_data_owner_update_family(
            crate::app::account_data::NativeAccountDataWakeupKind::AgentApprovalHistory
        ),
        Some("agent_approval_history")
    );
    assert_ne!(
        super::account_data_owner_update_family(
            crate::app::account_data::NativeAccountDataWakeupKind::AgentApprovalHistory
        ),
        Some("image_packs")
    );
}

struct MemoryCallbackVault(std::sync::Arc<Mutex<HashMap<String, Vec<u8>>>>);

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

#[tokio::test]
async fn session_logout_forgets_credentials_when_restore_is_unavailable() {
    let entries = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(entries.clone())));
    let identity = alice();
    let credential_key = SessionMaterialId::from_identity(&identity)
        .account()
        .to_owned();
    let history_key = StoreKeyId::from_identity(&identity).account().to_owned();
    {
        let mut vault = entries.lock().unwrap();
        vault.insert(credential_key.clone(), b"unrestorable-session".to_vec());
        vault.insert(history_key.clone(), vec![7; 32]);
        vault.insert("other-account".into(), b"other-credential".to_vec());
    }
    assert!(!shared
        .revoke_server_session(
            identity.user_id().into(),
            "DEVICE".into(),
            identity.homeserver_url().into()
        )
        .await
        .unwrap());
    let result = shared
        .forget_session(identity.user_id().into(), identity.homeserver_url().into())
        .await
        .unwrap();
    assert_eq!(result.status, "forgotten");
    let vault = entries.lock().unwrap();
    assert!(!vault.contains_key(&credential_key));
    assert_eq!(vault.get(&history_key), Some(&vec![7; 32]));
    assert!(vault.contains_key("other-account"));
}

#[tokio::test]
async fn session_logout_revokes_exact_device_and_cannot_restore_after_forget() {
    use matrix_sdk::test_utils::mocks::MatrixMockServer;
    let server = MatrixMockServer::new().await;
    server.mock_versions().ok().mount().await;
    server
        .mock_logout()
        .expect_access_token("fixture-token")
        .ok()
        .expect(1)
        .mount()
        .await;
    let entries = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(entries.clone())));
    let root = temp_root("logout");
    let root_path = root.to_string_lossy().into_owned();
    shared
        .persist_planted_session_for_test(
            "@alice:example.org".into(),
            server.server().uri(),
            root_path.clone(),
            "DEVICE".into(),
            "fixture-token".into(),
            None,
        )
        .await
        .unwrap();
    assert!(shared
        .revoke_server_session(
            "@alice:example.org".into(),
            "OTHER".into(),
            server.server().uri()
        )
        .await
        .is_err());
    assert!(shared
        .forget_session("@bob:example.org".into(), server.server().uri())
        .await
        .is_err());
    assert!(shared
        .revoke_server_session(
            "@alice:example.org".into(),
            "DEVICE".into(),
            server.server().uri()
        )
        .await
        .unwrap());
    shared
        .forget_session("@alice:example.org".into(), server.server().uri())
        .await
        .unwrap();
    assert!(shared.core.session_snapshot().unwrap().is_none());
    assert!(shared.retained_client().await.is_err());
    let relaunched = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(entries)));
    let error = relaunched
        .restore_persisted_session(
            "@alice:example.org".into(),
            server.server().uri(),
            root_path,
        )
        .await
        .unwrap_err();
    assert!(
        matches!(error, SessionRestoreError::Failed { code, .. } if code == MATERIAL_MISSING_CODE)
    );
    server.verify_and_reset().await;
    fs::remove_dir_all(root).unwrap();
}

fn alice() -> AccountIdentity {
    AccountIdentity::new("@alice:example.org", "https://matrix.example.org").unwrap()
}

fn temp_root(tag: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("synara-p4-s3b-{tag}-{nanos}"));
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
fn shared_core_constructs_and_retains_the_built_in_core() {
    let shared_core = SharedCore::new();
    assert!(
        !shared_core.core.registered_commands().is_empty(),
        "P4-S2 must retain a real Core with its built-in registry"
    );
}

#[test]
fn verification_list_projection_preserves_display_only_sas() {
    let dto = verification_request_dto_with_sas(NativeVerificationRequest {
        flow_id: "flow".to_owned(),
        other_user_id: "@alice:example.org".to_owned(),
        other_device_id: Some("DEVICE".to_owned()),
        direction: NativeVerificationDirection::Incoming,
        phase: NativeVerificationPhase::SasReady,
        started_ts: Some(1),
        sas: Some(NativeVerificationSas {
            emoji: Some(vec![NativeVerificationEmoji {
                symbol: "🐶".to_owned(),
                description: "Dog".to_owned(),
            }]),
            decimals: Some([1234, 5678, 9012]),
        }),
        qr: None,
    });

    let sas = dto.sas.expect("sas_ready list row must carry display SAS");
    assert_eq!(sas.emoji.expect("emoji")[0].symbol, "🐶");
    assert_eq!(sas.decimals, Some(vec![1234, 5678, 9012]));
}

#[test]
fn shared_core_with_secret_store_round_trips_through_the_callback() {
    let store = Box::new(MemoryCallbackVault(std::sync::Arc::new(Mutex::new(
        HashMap::new(),
    ))));
    let shared = SharedCore::new_with_secret_store(store);
    assert!(
        !shared.core.registered_commands().is_empty(),
        "P4-S3a must still retain a real Core"
    );
}

#[test]
fn sync_stop_closes_retained_client_stores_and_start_reopens_them() {
    let identity = alice();
    let values = std::sync::Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(
        std::sync::Arc::clone(&values),
    )));
    let root = temp_root("sync-store-quiescence");
    let rt = test_runtime();
    let _enter = rt.enter();

    rt.block_on(shared.persist_planted_session_for_test(
        identity.user_id().to_owned(),
        identity.homeserver_url().to_owned(),
        root.to_string_lossy().into_owned(),
        "DEVICEABC".to_owned(),
        "syt_sync_store_quiescence_access".to_owned(),
        None,
    ))
    .expect("planted persist retains a SQLite-backed Client");
    rt.block_on(shared.attach_session_owners())
        .expect("attach retained session owners");
    let client = rt
        .block_on(shared.retained_client())
        .expect("retained Client");
    let _serial = crate::app::media::HOT_MEDIA_CACHE_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let other_account = "@sync-stop-other:example.org";
    crate::app::media::hot_media_cache_insert_for_test(identity.user_id(), "photo", vec![1; 8]);
    crate::app::media::hot_media_cache_insert_for_test(other_account, "photo", vec![2; 8]);

    let stopped = rt
        .block_on(shared.stop_sync())
        .expect("stop must complete the full store quiescence boundary");
    assert!(stopped.stopped);
    // L14: suspension drops this account's decrypted media, nobody else's.
    assert!(!crate::app::media::hot_media_cache_contains_for_test(
        identity.user_id(),
        "photo"
    ));
    assert!(crate::app::media::hot_media_cache_contains_for_test(
        other_account,
        "photo"
    ));
    // A memory warning drops every account's.
    shared.release_cached_memory();
    assert!(!crate::app::media::hot_media_cache_contains_for_test(
        other_account,
        "photo"
    ));
    let paused_store_access = rt.block_on(client.event_cache_store().lock());
    assert!(
        paused_store_access.is_err(),
        "stop_sync returned before the retained event-cache store was closed"
    );

    rt.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(15), shared.start_sync())
            .await
            .expect("start_sync timed out")
    })
    .expect("start must resume stores before restarting SyncService");
    let resumed_store_access = rt.block_on(client.event_cache_store().lock());
    assert!(
        resumed_store_access.is_ok(),
        "start_sync did not reopen the retained event-cache store"
    );
    drop(resumed_store_access);

    rt.block_on(shared.stop_sync())
        .expect("final stop releases store resources before teardown");
    drop(client);
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn store_calls_while_suspended_wait_for_resume_or_fail_retryably() {
    let identity = alice();
    let values = std::sync::Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(
        std::sync::Arc::clone(&values),
    )));
    let root = temp_root("suspended-store-calls");
    let rt = test_runtime();
    let _enter = rt.enter();

    rt.block_on(shared.persist_planted_session_for_test(
        identity.user_id().to_owned(),
        identity.homeserver_url().to_owned(),
        root.to_string_lossy().into_owned(),
        "DEVICEABC".to_owned(),
        "syt_suspended_store_calls_access".to_owned(),
        None,
    ))
    .expect("planted persist retains a SQLite-backed Client");
    rt.block_on(shared.attach_session_owners())
        .expect("attach retained session owners");
    let status = rt.block_on(shared.sync_status()).expect("status");
    assert!(
        !status.suspended,
        "an attached, unstarted session is not suspended"
    );

    rt.block_on(shared.stop_sync())
        .expect("stop closes the stores");
    let status = rt.block_on(shared.sync_status()).expect("status");
    assert!(status.suspended);
    assert_eq!(status.readiness, SyncReadinessDto::Idle);

    // Past the bound, every gated family fails with the one retryable code
    // instead of a generic open or store failure.
    shared
        .suspension
        .set_wait_bound_for_test(std::time::Duration::from_millis(50));
    let suspended = |text: String| text.contains(SESSION_SUSPENDED_CODE);
    let room_list = rt.block_on(shared.room_list_snapshot());
    assert!(suspended(format!(
        "{:?}",
        room_list.expect_err("room list")
    )));
    let invites = rt.block_on(shared.invites_snapshot());
    assert!(suspended(format!("{:?}", invites.expect_err("invites"))));
    let open = rt.block_on(shared.timeline_open(
        "!room:example.org".to_owned(),
        TimelineOpenPositionDto {
            kind: TimelineOpenKindDto::LiveBottom,
            at_bottom: true,
            restored_anchor_event_id: None,
            live_tail_event_id: None,
            updated_at_ms: None,
            event_id: None,
        },
    ));
    assert!(suspended(format!("{:?}", open.expect_err("timeline open"))));
    let readback = rt.block_on(
        shared.timeline_event_readback("!room:example.org".to_owned(), "$event".to_owned()),
    );
    assert!(suspended(format!("{:?}", readback.expect_err("readback"))));
    let send = rt.block_on(shared.send_text(
        "!room:example.org".to_owned(),
        "hello".to_owned(),
        None,
        None,
        None,
        None,
        None,
        None,
        Some("txn-suspended".to_owned()),
    ));
    assert!(suspended(format!("{:?}", send.expect_err("send"))));
    // A network change while suspended must not start sync on closed stores.
    let restarted = rt
        .block_on(shared.restart_sync())
        .expect("restart is a no-op while suspended");
    assert_eq!(restarted.readiness, SyncReadinessDto::Idle);
    assert!(rt.block_on(shared.sync_status()).expect("status").suspended);

    // Within the bound, a call made while suspended waits for the resume and
    // then runs against the reopened stores.
    shared
        .suspension
        .set_wait_bound_for_test(std::time::Duration::from_secs(20));
    let (room_list, started) = rt.block_on(async {
        tokio::join!(shared.room_list_snapshot(), async {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            tokio::time::timeout(std::time::Duration::from_secs(15), shared.start_sync())
                .await
                .expect("start_sync timed out")
        })
    });
    started.expect("start reopens the stores");
    let room_list = room_list.expect("a room-list read parked on the suspension succeeds");
    assert!(room_list.rooms.is_empty());
    assert!(!rt.block_on(shared.sync_status()).expect("status").suspended);

    rt.block_on(shared.stop_sync())
        .expect("final stop releases store resources before teardown");
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn callback_vault_maps_foreign_failure_to_static_store_unavailable() {
    struct FailingVault;
    impl IosSecretVault for FailingVault {
        fn get(&self, _: String) -> Result<Option<Vec<u8>>, IosSecretVaultError> {
            Err(IosSecretVaultError::Unavailable {
                code: "p4-s3-secret-vault-unavailable".to_owned(),
                description: "The secret store is unavailable.".to_owned(),
            })
        }
        fn put(&self, _: String, _: Vec<u8>) -> Result<(), IosSecretVaultError> {
            unreachable!("put")
        }
        fn delete(&self, _: String) -> Result<(), IosSecretVaultError> {
            unreachable!("delete")
        }
    }

    let vault = CallbackSecretVault {
        inner: Box::new(FailingVault),
    };
    let error = vault.get("session").expect_err("must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::StoreUnavailable);
    assert!(!format!("{error:?}").contains("session"));
}

#[test]
fn restore_without_vault_fails_closed_without_echoing_identity() {
    let shared = SharedCore::new();
    let root = temp_root("no-vault");
    let rt = test_runtime();
    let error = rt
        .block_on(shared.restore_persisted_session(
            "@alice:example.org".to_owned(),
            "https://matrix.example.org".to_owned(),
            root.to_string_lossy().into_owned(),
        ))
        .expect_err("fail-closed vault cannot restore");
    let text = format!("{error:?}");
    assert!(text.contains(VAULT_UNAVAILABLE_CODE));
    assert!(!text.contains(MATERIAL_MISSING_CODE));
    assert!(!text.contains("@alice"));
    assert!(!text.contains("matrix.example.org"));
    assert!(!text.contains(root.to_string_lossy().as_ref()));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_restore_or_login_in_flight_answers_a_second_attempt_at_once() {
    let shared = SharedCore::new();
    let root = temp_root("install-in-flight");
    let rt = test_runtime();
    let restore = || {
        rt.block_on(shared.restore_persisted_session(
            "@alice:example.org".to_owned(),
            "https://matrix.example.org".to_owned(),
            root.to_string_lossy().into_owned(),
        ))
        .expect_err("no restorable session")
    };
    // Another install holds the lifecycle owner's transition gate.
    let in_flight = shared
        .lifecycle
        .try_begin_detached_install()
        .expect("gate free");
    assert!(format!("{:?}", restore()).contains(RESTORE_FAILED_CODE));
    let login = rt
        .block_on(shared.login_with_password(
            "@alice:example.org".to_owned(),
            "https://matrix.example.org".to_owned(),
            root.to_string_lossy().into_owned(),
            "password".to_owned(),
            None,
        ))
        .expect_err("sign-in waits for nothing");
    assert!(format!("{login:?}").contains(LOGIN_FAILED_CODE));
    // Ordinary commands keep failing closed on the empty slot meanwhile.
    assert!(!rt.block_on(shared.has_retained_client()));
    drop(in_flight);
    assert!(format!("{:?}", restore()).contains(VAULT_UNAVAILABLE_CODE));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn restore_rejects_hostile_identity_without_echo() {
    let store = Box::new(MemoryCallbackVault(std::sync::Arc::new(Mutex::new(
        HashMap::new(),
    ))));
    let shared = SharedCore::new_with_secret_store(store);
    let root = temp_root("hostile");
    let rt = test_runtime();
    let hostile = "https://user:secret@evil.example/?password=hunter2";
    let error = rt
        .block_on(shared.restore_persisted_session(
            "not-a-user".to_owned(),
            hostile.to_owned(),
            root.to_string_lossy().into_owned(),
        ))
        .expect_err("invalid identity");
    let text = format!("{error:?}{error}");
    assert!(text.contains(IDENTITY_INVALID_CODE));
    assert!(!text.contains("secret"));
    assert!(!text.contains("hunter2"));
    assert!(!text.contains("evil.example"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn restore_from_vault_installs_session_without_password_or_token_leak() {
    let access = "syt_s3b_access_token_value";
    let refresh = "syr_s3b_refresh_token_value";
    let identity = alice();
    let material =
        SessionMaterial::from_matrix_tokens(&identity, "DEVICEABC", access, Some(refresh)).unwrap();
    let map = std::sync::Arc::new(Mutex::new(HashMap::new()));
    let persist_vault = SecretStoreSessionVault {
        store: Arc::new(CallbackSecretVault {
            inner: Box::new(MemoryCallbackVault(std::sync::Arc::clone(&map))),
        }),
    };
    persist_session_material(&persist_vault, &identity, &material).unwrap();
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(
        std::sync::Arc::clone(&map),
    )));
    let root = temp_root("restore");
    let rt = test_runtime();
    let _enter = rt.enter();
    let dto = rt
        .block_on(shared.restore_persisted_session(
            identity.user_id().to_owned(),
            identity.homeserver_url().to_owned(),
            root.to_string_lossy().into_owned(),
        ))
        .expect("restore");
    assert_eq!(dto.user_id, "@alice:example.org");
    assert_eq!(dto.device_id, "DEVICEABC");
    assert_eq!(dto.homeserver_url, "https://matrix.example.org");
    let dbg = format!("{dto:?}");
    assert!(!dbg.contains(access));
    assert!(!dbg.contains(refresh));
    assert!(!dbg.contains("password"));
    let snapshot = shared.core.session_snapshot().expect("projection");
    assert!(snapshot.is_some());
    assert!(rt.block_on(shared.has_retained_client()));
    let keys: Vec<String> = map.lock().expect("vault").keys().cloned().collect();
    assert!(keys.iter().any(|key| key.starts_with("store-key:")));
    assert!(keys.iter().any(|key| key.starts_with("matrix-session:")));
    assert!(!keys.iter().any(|key| key.contains("p4-s3b-store-key")));
    let second = rt
        .block_on(shared.restore_persisted_session(
            identity.user_id().to_owned(),
            identity.homeserver_url().to_owned(),
            root.to_string_lossy().into_owned(),
        ))
        .expect_err("second restore");
    assert!(format!("{second:?}").contains(ALREADY_RESTORED_CODE));
    assert!(!format!("{second:?}").contains(RESTORE_FAILED_CODE));
    assert!(rt.block_on(shared.has_retained_client()));
    // "Already restored" means success to the shells, so another account,
    // or the same user id on another homeserver, must not get it.
    for (user_id, homeserver_url) in [
        ("@bob:example.org", "https://matrix.example.org"),
        ("@alice:example.org", "https://other.example.org"),
    ] {
        let other = rt
            .block_on(shared.restore_persisted_session(
                user_id.to_owned(),
                homeserver_url.to_owned(),
                root.to_string_lossy().into_owned(),
            ))
            .expect_err("another account's restore");
        assert!(format!("{other:?}").contains(RESTORED_IDENTITY_MISMATCH_CODE));
        assert!(!format!("{other:?}").contains(ALREADY_RESTORED_CODE));
    }
    // Normalization differences of the same account still match.
    let same = rt
        .block_on(shared.restore_persisted_session(
            identity.user_id().to_owned(),
            format!("{}/", identity.homeserver_url()),
            root.to_string_lossy().into_owned(),
        ))
        .expect_err("same account, trailing slash");
    assert!(format!("{same:?}").contains(ALREADY_RESTORED_CODE));
    assert!(rt.block_on(shared.has_retained_client()));
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn restore_rejects_wrong_length_store_key_without_replacing_it() {
    let identity = alice();
    let material = SessionMaterial::from_matrix_tokens(
        &identity,
        "DEVICEABC",
        "syt_s3b_corrupt_key_access",
        None,
    )
    .unwrap();
    let map = std::sync::Arc::new(Mutex::new(HashMap::new()));
    let persist_vault = SecretStoreSessionVault {
        store: Arc::new(CallbackSecretVault {
            inner: Box::new(MemoryCallbackVault(std::sync::Arc::clone(&map))),
        }),
    };
    persist_session_material(&persist_vault, &identity, &material).unwrap();
    let store_key_account = StoreKeyId::from_identity(&identity).account().to_owned();
    map.lock()
        .expect("vault")
        .insert(store_key_account.clone(), vec![0u8; 8]);
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(
        std::sync::Arc::clone(&map),
    )));
    let root = temp_root("corrupt-key");
    let rt = test_runtime();
    let error = rt
        .block_on(shared.restore_persisted_session(
            identity.user_id().to_owned(),
            identity.homeserver_url().to_owned(),
            root.to_string_lossy().into_owned(),
        ))
        .expect_err("corrupt store key");
    // A key that cannot open the store asks for a reset; it is never replaced.
    assert!(format!("{error:?}").contains(STORE_RESET_REQUIRED_CODE));
    let stored = map
        .lock()
        .expect("vault")
        .get(&store_key_account)
        .cloned()
        .expect("key remains");
    assert_eq!(stored.len(), 8);
    assert!(!map
        .lock()
        .expect("vault")
        .keys()
        .any(|key| key.contains("p4-s3b-store-key")));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn leftover_oversize_fails_closed_without_truncate_or_echo() {
    let marker = "s10OversizeMarker";
    let error = leftover_reject_oversize(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
        .expect_err("oversize leftover payload must fail closed");
    let text = format!("{error:?}{error}");
    assert!(text.contains(LEFTOVER_OVERSIZE_CODE));
    assert!(!text.contains(marker));
    assert!(!text.contains("syt_"));
}

#[test]
fn leftover_commands_without_session_fail_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let recovery_key = "s10-secret-recovery-key";
    let room_id = "!s10SecretRoom:example.org";
    let action_title = "s10-secret-action-title";
    let mxc = "mxc://example.org/s10SecretMedia";

    let recover = rt
        .block_on(shared.recover(recovery_key.to_owned()))
        .expect_err("recover must fail closed");
    let recover_text = format!("{recover:?}{recover}");
    assert!(recover_text.contains(LEFTOVER_UNAVAILABLE_CODE));
    assert!(!recover_text.contains(recovery_key));

    let approval = rt
        .block_on(shared.send_agent_approval(
            room_id.to_owned(),
            "approve-s10".to_owned(),
            action_title.to_owned(),
            "approve".to_owned(),
            Some("$source:example.org".to_owned()),
            1,
        ))
        .expect_err("agent approval must fail closed");
    let approval_text = format!("{approval:?}{approval}");
    assert!(approval_text.contains(AGENT_APPROVAL_NO_SESSION_CODE));
    assert!(!approval_text.contains(room_id));
    assert!(!approval_text.contains(action_title));

    let media = rt
        .block_on(shared.media_download(mxc.to_owned()))
        .expect_err("media download must fail closed");
    let media_text = format!("{media:?}{media}");
    assert!(media_text.contains(LEFTOVER_NO_SESSION_CODE));
    assert!(!media_text.contains(mxc));

    let crypto = rt
        .block_on(shared.crypto_status())
        .expect_err("crypto status must fail closed without a platform session");
    let crypto_text = format!("{crypto:?}{crypto}");
    assert!(
        crypto_text.contains("p2-crypto-status-platform-unavailable")
            || crypto_text.contains(LEFTOVER_FAILED_CODE)
    );
    assert!(!crypto_text.contains(room_id));
}

#[test]
fn leftover_wipe_removes_only_the_validated_store_root() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let root = temp_root("s10-wipe");
    fs::create_dir_all(root.join("data")).unwrap();
    let ack = rt
        .block_on(shared.wipe_persisted_stores(root.to_string_lossy().into_owned()))
        .expect("validated leftover wipe");
    assert_eq!(ack.status, "wiped");
    assert!(!root.exists());
}

#[test]
fn timeline_view_row_dto_maps_message_without_token_echo() {
    use crate::app::timeline::{
        TimelineEventRowBase, TimelineForwardTransport, TimelineMessageRow, TimelineReaction,
        TimelineReplyPreview, TimelineRowCapabilities, TimelineThreadSummary,
    };
    let row = TimelineViewRow::Message(Box::new(TimelineMessageRow {
        event: TimelineEventRowBase {
            item_id: "item-1".to_owned(),
            event_id: Some("$evt:example.org".to_owned()),
            sender_id: "@alice:example.org".to_owned(),
            sender_name: "Alice Example".to_owned(),
            sender_avatar_url: Some("mxc://example.org/alice".to_owned()),
            origin_server_ts: 1_700_000_000_000,
            local_echo_state: None,
            send_failure: None,
            send_failure_retryable: None,
            transaction_id: None,
            encryption_shield: None,
            capabilities: TimelineRowCapabilities {
                react: true,
                reply: true,
                edit: false,
                redact: true,
                report: true,
                pin: true,
                forward: true,
                vote: false,
                decline_call: false,
            },
        },
        body: "hello".to_owned(),
        formatted_body: None,
        agent_card_json: Some(r#"{"title":"Approval"}"#.to_owned()),
        is_agent_approval: true,
        agent_approval_decision: Some(crate::app::timeline::TimelineAgentApprovalDecision {
            decision: crate::app::account_data::SynaraAgentApprovalHistoryDecision::ApproveOnce,
            decided_at: Some(1_700_000_060_000),
        }),
        message_type: Some("m.text".to_owned()),
        forward_transport: Some(TimelineForwardTransport::Text),
        media_filename: None,
        media_caption: None,
        edited: false,
        reply: Some(TimelineReplyPreview {
            event_id: "$reply:example.org".to_owned(),
            sender_id: Some("@bob:example.org".to_owned()),
            sender_name: "Bob".to_owned(),
            body: "earlier body".to_owned(),
        }),
        thread_root: Some("$root:example.org".to_owned()),
        thread: Some(TimelineThreadSummary {
            root_event_id: "$evt:example.org".to_owned(),
            reply_count: 3,
            latest_event_id: Some("$latest:example.org".to_owned()),
        }),
        reactions: vec![
            TimelineReaction {
                key: "👍".to_owned(),
                count: 2,
                own: Some(true),
                senders: vec![],
            },
            TimelineReaction {
                key: "🎉".to_owned(),
                count: 1,
                own: None,
                senders: vec![],
            },
        ],
        media: None,
    }));
    let dto = timeline_view_row_dto(row);
    assert_eq!(dto.kind, "message");
    assert_eq!(dto.item_id, "item-1");
    assert_eq!(dto.event_id, "$evt:example.org");
    assert_eq!(dto.sender_name, "Alice Example");
    assert_eq!(
        dto.sender_avatar_url.as_deref(),
        Some("mxc://example.org/alice")
    );
    assert_eq!(dto.body, "hello");
    assert_eq!(dto.message_type.as_deref(), Some("m.text"));
    assert_eq!(dto.forward_transport.as_deref(), Some("text"));
    assert_eq!(
        dto.agent_card_json.as_deref(),
        Some(r#"{"title":"Approval"}"#)
    );
    assert!(dto.is_agent_approval);
    assert_eq!(dto.agent_approval_decision.as_deref(), Some("approve_once"));
    assert_eq!(dto.agent_approval_decided_at, Some(1_700_000_060_000));
    assert_eq!(dto.reply_to_event_id.as_deref(), Some("$reply:example.org"));
    assert_eq!(
        dto.thread_root_event_id.as_deref(),
        Some("$root:example.org")
    );
    assert_eq!(
        dto.reply_preview.as_ref().map(|reply| (
            reply.sender_id.as_deref(),
            reply.sender_name.as_str(),
            reply.body.as_str()
        )),
        Some((Some("@bob:example.org"), "Bob", "earlier body"))
    );
    assert_eq!(
        dto.thread_summary.as_ref().map(|thread| (
            thread.root_event_id.as_str(),
            thread.reply_count,
            thread.latest_event_id.as_deref()
        )),
        Some(("$evt:example.org", 3, Some("$latest:example.org")))
    );
    assert_eq!(dto.reactions[0].own, Some(true));
    assert_eq!(dto.reactions[1].own, None);
    let capabilities = dto.capabilities.as_ref().expect("event capabilities");
    assert!(capabilities.reply);
    assert!(capabilities.react);
    assert!(!capabilities.vote);
    assert!(dto.poll.is_none());
    assert!(dto.media_handle_id.is_none());
    assert!(dto.transaction_id.is_none(), "remote rows carry no txn id");
    let text = format!("{dto:?}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("password"));
    assert!(text.contains("mxc://example.org/alice"));
}

#[test]
fn timeline_view_row_dto_carries_local_echo_transaction_id() {
    use crate::app::timeline::{TimelineEventRowBase, TimelineMessageRow, TimelineRowCapabilities};
    use crate::dto::LocalEchoState;

    // A queued attachment: the SDK local echo has no event id yet. The shell
    // matches its own pending row to this one by transaction id, so the id
    // must cross the FFI even though `event_id` is empty.
    let row = TimelineViewRow::Message(Box::new(TimelineMessageRow {
        event: TimelineEventRowBase {
            item_id: "local-item".to_owned(),
            event_id: None,
            sender_id: "@alice:example.org".to_owned(),
            sender_name: "Alice".to_owned(),
            sender_avatar_url: None,
            origin_server_ts: 1_700_000_000_001,
            local_echo_state: Some(LocalEchoState::Sending),
            send_failure: None,
            send_failure_retryable: None,
            transaction_id: Some("synara-attachment-1".to_owned()),
            encryption_shield: None,
            capabilities: TimelineRowCapabilities {
                react: false,
                reply: false,
                edit: false,
                redact: false,
                report: false,
                pin: false,
                forward: false,
                vote: false,
                decline_call: false,
            },
        },
        body: "photo.jpg".to_owned(),
        formatted_body: None,
        agent_card_json: None,
        is_agent_approval: false,
        agent_approval_decision: None,
        message_type: Some("m.image".to_owned()),
        forward_transport: None,
        media_filename: Some("photo.jpg".to_owned()),
        media_caption: None,
        edited: false,
        reply: None,
        thread_root: None,
        thread: None,
        reactions: vec![],
        media: None,
    }));
    let dto = timeline_view_row_dto(row);
    assert_eq!(dto.item_id, "local-item");
    assert!(dto.event_id.is_empty());
    assert_eq!(dto.transaction_id.as_deref(), Some("synara-attachment-1"));
    assert_eq!(dto.local_echo_state.as_deref(), Some("sending"));
    assert_eq!(dto.send_failure, None);
}

#[test]
fn timeline_view_row_dto_preserves_open_and_closed_poll_semantics() {
    use crate::app::timeline::{
        TimelineEventRowBase, TimelinePollAnswer, TimelinePollRow, TimelineRowCapabilities,
    };

    let make_poll = |closed: bool| {
        TimelineViewRow::Poll(TimelinePollRow {
            event: TimelineEventRowBase {
                item_id: if closed { "closed-poll" } else { "open-poll" }.to_owned(),
                event_id: Some("$poll:example.org".to_owned()),
                sender_id: "@alice:example.org".to_owned(),
                sender_name: "Alice".to_owned(),
                sender_avatar_url: None,
                origin_server_ts: 1_700_000_000_002,
                local_echo_state: None,
                send_failure: None,
                send_failure_retryable: None,
                transaction_id: None,
                encryption_shield: None,
                capabilities: TimelineRowCapabilities {
                    react: true,
                    reply: false,
                    edit: false,
                    redact: true,
                    report: true,
                    pin: true,
                    forward: false,
                    vote: !closed,
                    decline_call: false,
                },
            },
            question: "Choose two".to_owned(),
            closed,
            max_selections: 2,
            answers: vec![
                TimelinePollAnswer {
                    id: "a".to_owned(),
                    text: "Alpha".to_owned(),
                    vote_count: 4,
                    own: true,
                },
                TimelinePollAnswer {
                    id: "b".to_owned(),
                    text: "Beta".to_owned(),
                    vote_count: 1,
                    own: false,
                },
            ],
            reply: Some(TimelineReplyPreview {
                event_id: "$poll-reply:example.org".to_owned(),
                sender_id: None,
                sender_name: "Message".to_owned(),
                body: "Jump to original".to_owned(),
            }),
            thread_root: Some("$poll-root:example.org".to_owned()),
            thread: None,
            reactions: vec![TimelineReaction {
                key: "👍".to_owned(),
                count: 2,
                own: Some(true),
                senders: vec![],
            }],
        })
    };

    let open = timeline_view_row_dto(make_poll(false));
    let open_poll = open.poll.expect("open poll presentation");
    assert_eq!(open.body, "Choose two");
    assert!(!open_poll.closed);
    assert_eq!(open_poll.max_selections, 2);
    assert_eq!(open_poll.answers.len(), 2);
    assert!(open_poll.answers[0].own);
    assert_eq!(open_poll.answers[0].vote_count, 4);
    assert!(open.capabilities.expect("open capabilities").vote);
    assert_eq!(
        open.reply_to_event_id.as_deref(),
        Some("$poll-reply:example.org")
    );
    assert_eq!(
        open.thread_root_event_id.as_deref(),
        Some("$poll-root:example.org")
    );
    assert_eq!(open.reactions[0].own, Some(true));
    assert!(open.thread_summary.is_none());

    let closed = timeline_view_row_dto(make_poll(true));
    assert!(closed.poll.expect("closed poll presentation").closed);
    assert!(!closed.capabilities.expect("closed capabilities").vote);
}

#[test]
fn timeline_view_row_dto_preserves_incoming_sticker_media() {
    use crate::app::timeline::{
        TimelineEventRowBase, TimelineForwardTransport, TimelineMediaHandle,
        TimelineRowCapabilities,
    };

    let row = TimelineViewRow::Sticker {
        event: TimelineEventRowBase {
            item_id: "sticker-item".to_owned(),
            event_id: Some("$sticker:example.org".to_owned()),
            sender_id: "@alice:example.org".to_owned(),
            sender_name: "Alice".to_owned(),
            sender_avatar_url: Some("mxc://example.org/alice".to_owned()),
            origin_server_ts: 1_700_000_000_001,
            local_echo_state: None,
            send_failure: None,
            send_failure_retryable: None,
            transaction_id: None,
            encryption_shield: None,
            capabilities: TimelineRowCapabilities {
                react: true,
                reply: true,
                edit: false,
                redact: true,
                report: true,
                pin: true,
                forward: true,
                vote: false,
                decline_call: false,
            },
        },
        media: TimelineMediaHandle {
            handle_id: "incoming-sticker-handle".to_owned(),
            mime_type: Some("image/webp".to_owned()),
            width: Some(256),
            height: Some(128),
            duration_ms: None,
        },
        forward_transport: TimelineForwardTransport::Media,
        reply: None,
        thread_root: Some("$sticker-root:example.org".to_owned()),
        thread: None,
        reactions: vec![TimelineReaction {
            key: "🎉".to_owned(),
            count: 3,
            own: Some(false),
            senders: vec![],
        }],
    };

    let dto = timeline_view_row_dto(row);
    assert_eq!(dto.kind, "sticker");
    assert_eq!(dto.event_id, "$sticker:example.org");
    assert_eq!(dto.message_type.as_deref(), Some("m.sticker"));
    assert_eq!(dto.forward_transport.as_deref(), Some("media"));
    assert_eq!(
        dto.thread_root_event_id.as_deref(),
        Some("$sticker-root:example.org")
    );
    assert_eq!(dto.reactions[0].key, "🎉");
    assert_eq!(dto.reactions[0].own, Some(false));
    assert_eq!(
        dto.media_handle_id.as_deref(),
        Some("incoming-sticker-handle")
    );
    assert_eq!(dto.media_mime_type.as_deref(), Some("image/webp"));
    assert_eq!(dto.media_width, Some(256));
    assert_eq!(dto.media_height, Some(128));
    assert_eq!(dto.sender, "@alice:example.org");
    assert_eq!(
        dto.sender_avatar_url.as_deref(),
        Some("mxc://example.org/alice")
    );
}

#[test]
fn timeline_view_row_dto_preserves_base_metadata_for_non_message_events() {
    use crate::app::timeline::{
        TimelineEncryptedUnavailableRow, TimelineEventRowBase, TimelineOtherRow,
        TimelineRedactedRow, TimelineRowCapabilities,
    };

    let base = |item_id: &str, event_id: &str| TimelineEventRowBase {
        item_id: item_id.to_owned(),
        event_id: Some(event_id.to_owned()),
        sender_id: "@alice:example.org".to_owned(),
        sender_name: "Alice".to_owned(),
        sender_avatar_url: Some("mxc://example.org/alice".to_owned()),
        origin_server_ts: 1_700_000_000_003,
        local_echo_state: None,
        send_failure: None,
        send_failure_retryable: None,
        transaction_id: None,
        encryption_shield: None,
        capabilities: TimelineRowCapabilities {
            react: false,
            reply: false,
            edit: false,
            redact: true,
            report: true,
            pin: true,
            forward: false,
            vote: false,
            decline_call: false,
        },
    };
    let assert_base = |dto: &TimelineViewRowDto, event_id: &str| {
        assert_eq!(dto.event_id, event_id);
        assert_eq!(dto.sender, "@alice:example.org");
        assert_eq!(dto.sender_name, "Alice");
        assert_eq!(
            dto.sender_avatar_url.as_deref(),
            Some("mxc://example.org/alice")
        );
        assert_eq!(dto.origin_server_ts, 1_700_000_000_003);
        let capabilities = dto.capabilities.as_ref().expect("event capabilities");
        assert!(capabilities.redact);
        assert!(capabilities.report);
    };

    let redacted = timeline_view_row_dto(TimelineViewRow::Redacted(TimelineRedactedRow {
        event: base("redacted-item", "$redacted:example.org"),
        summary: "Message removed".to_owned(),
    }));
    assert_eq!(redacted.kind, "redacted");
    assert_base(&redacted, "$redacted:example.org");

    let encrypted = timeline_view_row_dto(TimelineViewRow::EncryptedUnavailable(
        TimelineEncryptedUnavailableRow {
            event: base("encrypted-item", "$encrypted:example.org"),
            reason_code: "unable_to_decrypt".to_owned(),
        },
    ));
    assert_eq!(encrypted.kind, "encrypted");
    assert_base(&encrypted, "$encrypted:example.org");

    let other_base = base("other-item", "$other:example.org");
    let other = timeline_view_row_dto(TimelineViewRow::Other(TimelineOtherRow {
        item_id: other_base.item_id.clone(),
        event_id: other_base.event_id.clone(),
        event: Some(other_base),
        event_type: Some("org.example.unknown".to_owned()),
        forward_transport: None,
        summary: "Unsupported timeline event".to_owned(),
    }));
    assert_eq!(other.kind, "other");
    assert_base(&other, "$other:example.org");
    assert_eq!(other.message_type.as_deref(), Some("org.example.unknown"));
}

#[test]
fn session_generation_is_monotonic_per_instance() {
    // A re-login in the same process must install a new generation, so a
    // shell fence keyed on a retired generation cannot match its successor.
    let shared = SharedCore::new();
    let first = shared.allocate_session_generation();
    let second = shared.allocate_session_generation();
    assert_eq!(first, 1);
    assert!(second > first);
    // A fresh instance starts again at 1 and never yields 0 (attach rejects 0).
    assert_eq!(SharedCore::new().allocate_session_generation(), 1);
}

#[test]
fn logout_fences_late_token_rotation_saves_out_of_the_vault() {
    let identity = alice();
    let map = std::sync::Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(
        std::sync::Arc::clone(&map),
    )));
    let root = temp_root("rotation-fence");
    let rt = test_runtime();
    let _enter = rt.enter();
    rt.block_on(shared.persist_planted_session_for_test(
        identity.user_id().to_owned(),
        identity.homeserver_url().to_owned(),
        root.to_string_lossy().into_owned(),
        "DEVICEABC".to_owned(),
        "syt_rotation_fence_access".to_owned(),
        Some("syr_rotation_fence_refresh".to_owned()),
    ))
    .expect("planted session");

    // The SDK save callback holds this lease; capture it like the callback does.
    let lease = rt
        .block_on(shared.with_installed_session(|active| {
            active.map(|active| active.persistence.callback_lease())
        }))
        .expect("planted session is retained");
    assert!(!lease.is_revoked());

    rt.block_on(shared.logout()).expect("logout");
    assert!(lease.is_revoked(), "logout revokes the rotation fence");

    map.lock().expect("vault").clear();
    let late = lease.save(|| {
        map.lock()
            .expect("vault")
            .insert("matrix-session:late".to_owned(), b"tokens".to_vec());
        Ok::<(), crate::app::lifecycle::session::SessionFault>(())
    });
    assert_eq!(
        late.unwrap_err().diagnostic_id,
        "d0.1-session-persistence-retired"
    );
    assert!(
        map.lock().expect("vault").is_empty(),
        "a refresh after logout never writes credentials"
    );
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}

struct FlakyCallbackVault {
    map: std::sync::Arc<Mutex<HashMap<String, Vec<u8>>>>,
    fail_puts: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl IosSecretVault for FlakyCallbackVault {
    fn get(&self, key: String) -> Result<Option<Vec<u8>>, IosSecretVaultError> {
        Ok(self.map.lock().expect("vault").get(&key).cloned())
    }

    fn put(&self, key: String, value: Vec<u8>) -> Result<(), IosSecretVaultError> {
        if self.fail_puts.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(IosSecretVaultError::Unavailable {
                code: "test".to_owned(),
                description: "test".to_owned(),
            });
        }
        self.map.lock().expect("vault").insert(key, value);
        Ok(())
    }

    fn delete(&self, key: String) -> Result<(), IosSecretVaultError> {
        self.map.lock().expect("vault").remove(&key);
        Ok(())
    }
}

#[test]
fn failed_rotation_save_is_retried_with_backoff_and_stops_after_logout() {
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};

    let identity = alice();
    let map = std::sync::Arc::new(Mutex::new(HashMap::new()));
    let fail_puts = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let shared = SharedCore::new_with_secret_store(Box::new(FlakyCallbackVault {
        map: std::sync::Arc::clone(&map),
        fail_puts: std::sync::Arc::clone(&fail_puts),
    }));
    let root = temp_root("rotation-retry");
    let rt = test_runtime();
    let _enter = rt.enter();
    rt.block_on(shared.persist_planted_session_for_test(
        identity.user_id().to_owned(),
        identity.homeserver_url().to_owned(),
        root.to_string_lossy().into_owned(),
        "DEVICEABC".to_owned(),
        "syt_rotation_retry_access".to_owned(),
        Some("syr_rotation_retry_refresh".to_owned()),
    ))
    .expect("planted session");
    let start = Instant::now();
    let retry = |now| rt.block_on(shared.retry_failed_session_save(now));
    assert_eq!(retry(start), None, "nothing to retry after a good save");

    let lease = rt
        .block_on(shared.with_installed_session(|active| {
            active.map(|active| active.persistence.callback_lease())
        }))
        .expect("planted session is retained");
    // A rotation save fails (vault unavailable).
    let _ = lease.save(|| {
        Err::<(), _>(crate::app::lifecycle::session::SessionFault::unavailable(
            "session-rotation-persist-failed",
        ))
    });
    assert!(lease.save_failed());

    fail_puts.store(true, Ordering::Relaxed);
    assert_eq!(retry(start), Some(false));
    assert_eq!(
        retry(start + Duration::from_secs(4)),
        None,
        "backoff holds the next attempt"
    );

    fail_puts.store(false, Ordering::Relaxed);
    map.lock().expect("vault").clear();
    assert_eq!(retry(start + Duration::from_secs(5)), Some(true));
    assert!(!lease.save_failed());
    assert!(map
        .lock()
        .expect("vault")
        .keys()
        .any(|key| key.starts_with("matrix-session:")));

    let _ = lease.save(|| {
        Err::<(), _>(crate::app::lifecycle::session::SessionFault::unavailable(
            "session-rotation-persist-failed",
        ))
    });
    rt.block_on(shared.logout()).expect("logout");
    assert_eq!(
        retry(start + Duration::from_secs(120)),
        None,
        "logout ends retries"
    );
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn rejected_session_retirement_forgets_credentials_locally_in_either_order() {
    for stop_first in [false, true] {
        let identity = alice();
        let map = std::sync::Arc::new(Mutex::new(HashMap::new()));
        let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(
            std::sync::Arc::clone(&map),
        )));
        let root = temp_root(if stop_first {
            "retire-rejected-after-logout"
        } else {
            "retire-rejected-live"
        });
        let rt = test_runtime();
        let _enter = rt.enter();
        rt.block_on(shared.persist_planted_session_for_test(
            identity.user_id().to_owned(),
            identity.homeserver_url().to_owned(),
            root.to_string_lossy().into_owned(),
            "DEVICEABC".to_owned(),
            "syt_retire_success_access".to_owned(),
            Some("syr_retire_success_refresh".to_owned()),
        ))
        .expect("planted session");
        let generation = shared
            .core
            .session_snapshot()
            .expect("projection")
            .expect("session")
            .session_generation;
        let session_keys = |map: &Mutex<HashMap<String, Vec<u8>>>| {
            map.lock()
                .expect("vault")
                .keys()
                .filter(|key| key.starts_with("matrix-session:"))
                .count()
        };
        assert_eq!(session_keys(&map), 1);

        // The sync status poll observed the rejected refresh for this generation.
        shared.note_authentication_rejection(Some(
            crate::app::sync::SYNC_AUTHENTICATION_FAILURE_DIAGNOSTIC_ID,
        ));
        if stop_first {
            // The shell already tore the session down locally; the latch, not
            // the live sync owner, still identifies what to forget.
            rt.block_on(shared.logout()).expect("local teardown");
            assert_eq!(session_keys(&map), 1, "teardown alone keeps the vault");
        }

        // The homeserver in this fixture is unreachable, so success also shows
        // retirement needs no remote call.
        let ack = rt
            .block_on(shared.retire_rejected_session(generation))
            .expect("retire the rejected generation");
        assert_eq!(ack.status, "retired");
        assert_eq!(session_keys(&map), 0, "session material is forgotten");
        assert!(shared.core.session_snapshot().unwrap().is_none());
        assert!(
            map.lock()
                .expect("vault")
                .keys()
                .any(|key| key.starts_with("store-key:")),
            "the store key and encrypted history stay"
        );
        let again = rt
            .block_on(shared.retire_rejected_session(generation))
            .expect("idempotent");
        assert_eq!(again.status, "retired");
        drop(shared);
        drop(_enter);
        drop(rt);
        let _ = fs::remove_dir_all(&root);
    }
}

#[test]
fn rejected_session_retirement_is_generation_fenced_and_idempotent() {
    let identity = alice();
    let map = std::sync::Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(
        std::sync::Arc::clone(&map),
    )));
    let root = temp_root("retire-rejected");
    let rt = test_runtime();
    let _enter = rt.enter();
    rt.block_on(shared.persist_planted_session_for_test(
        identity.user_id().to_owned(),
        identity.homeserver_url().to_owned(),
        root.to_string_lossy().into_owned(),
        "DEVICEABC".to_owned(),
        "syt_retire_rejected_access".to_owned(),
        Some("syr_retire_rejected_refresh".to_owned()),
    ))
    .expect("planted session");
    let generation = shared
        .core
        .session_snapshot()
        .expect("projection")
        .expect("session")
        .session_generation;
    let session_keys = |map: &Mutex<HashMap<String, Vec<u8>>>| {
        map.lock()
            .expect("vault")
            .keys()
            .filter(|key| key.starts_with("matrix-session:"))
            .count()
    };
    assert_eq!(session_keys(&map), 1);

    // A different generation, or one whose sync never reported the rejection,
    // is refused and leaves the session and vault alone.
    assert!(rt
        .block_on(shared.retire_rejected_session(generation + 1))
        .is_err());
    assert!(rt
        .block_on(shared.retire_rejected_session(generation))
        .is_err());
    assert_eq!(session_keys(&map), 1);
    assert!(shared.core.session_snapshot().unwrap().is_some());

    // Once the session is gone, retirement is a no-op success.
    rt.block_on(shared.logout()).expect("logout");
    let ack = rt
        .block_on(shared.retire_rejected_session(generation))
        .expect("idempotent after retirement");
    assert_eq!(ack.status, "retired");
    assert!(
        map.lock()
            .expect("vault")
            .keys()
            .any(|key| key.starts_with("store-key:")),
        "the store key and encrypted history stay"
    );
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn ios_store_preparation_records_the_revision_and_refuses_a_newer_layout() {
    use crate::app::store::{StorePaths, StoreRevisionManifest, STORE_LAYOUT_VERSION};
    let root = temp_root("store-revision");
    let identity =
        AccountIdentity::new("@alice:example.org", "https://matrix.example.org").unwrap();
    let paths = StorePaths::derive(&root, &identity).unwrap();
    let manifest_path = paths.account_root().join("revision.json");

    // A legacy store with no manifest keeps its data and gains the baseline.
    fs::create_dir_all(paths.account_root()).unwrap();
    let marker = paths.account_root().join("legacy-marker");
    fs::write(&marker, b"kept").unwrap();
    super::session_lifecycle::prepare_session_store(&root, &identity).unwrap();
    let manifest: StoreRevisionManifest =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    assert_eq!(manifest.layout_version, STORE_LAYOUT_VERSION);
    assert_eq!(fs::read(&marker).unwrap(), b"kept");
    // Preparing again is a no-op.
    super::session_lifecycle::prepare_session_store(&root, &identity).unwrap();

    // A store written by a newer layout fails closed before the SDK opens it.
    let ahead = StoreRevisionManifest {
        layout_version: STORE_LAYOUT_VERSION + 1,
        ..manifest
    };
    fs::write(&manifest_path, serde_json::to_vec(&ahead).unwrap()).unwrap();
    assert!(super::session_lifecycle::prepare_session_store(&root, &identity).is_err());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn ios_logout_and_attach_use_the_shared_core_policies() {
    let source = include_str!("session_lifecycle.rs");
    let logout = source
        .split("pub async fn logout(&self)")
        .nth(1)
        .and_then(|rest| rest.split("pub async fn recover(").next())
        .expect("logout body");
    assert!(logout.contains("session_policy::LogoutPolicy::IOS"));
    assert!(logout.contains("session_policy::run_local_logout("));
    assert!(!logout.contains("matrix_auth().logout()"));
    // One slot, transition gate and recover gate with desktop (review L21).
    assert!(logout.contains("self.lifecycle\n            .logout_in_place("));
    let restart = source
        .split("pub async fn restart_sync(&self)")
        .nth(1)
        .and_then(|rest| rest.split("pub async fn wipe_persisted_stores(").next())
        .expect("restart body");
    assert!(restart.contains(".recover_live("));
    for install in [
        "restore_persisted_session_with_policy(",
        "login_into_identity(",
        "persist_planted_session_for_test(",
    ] {
        let body = source
            .split(&format!("async fn {install}"))
            .nth(1)
            .expect("install body");
        let body = body.split("\n    }\n").next().expect("install body end");
        assert!(
            body.contains("begin_session_install(&self.lifecycle, &identity)")
                || body.contains("begin_session_install(&self.lifecycle, identity)")
        );
    }
    let attach = source
        .split("pub async fn attach_session_owners(&self)")
        .nth(1)
        .and_then(|rest| rest.split("pub async fn start_sync(&self)").next())
        .expect("attach body");
    assert!(attach.contains("session_policy::attach_owner_set("));
    for restore_or_login in [
        "restore_persisted_session_with_policy",
        "login_into_identity",
    ] {
        let body = source
            .split(&format!("async fn {restore_or_login}("))
            .nth(1)
            .expect("restore/login body");
        // Restore migrates inline on the blocking pool; sign-in through
        // `prepare_login_store`, which does the same off the async threads.
        let prepare = body
            .find("prepare_session_store(&root, &identity)")
            .or_else(|| body.find("self.prepare_login_store(root, identity)"))
            .unwrap();
        let build = body.find("build_unauthenticated_client(&config)").unwrap();
        assert!(
            prepare < build,
            "{restore_or_login} migrates before the SDK opens the store"
        );
    }
}

fn vault_session_secrets(
    entries: &Mutex<HashMap<String, Vec<u8>>>,
    identity: &AccountIdentity,
) -> crate::app::lifecycle::HostMatrixSessionSecrets {
    let key = SessionMaterialId::from_identity(identity)
        .account()
        .to_owned();
    let bytes = entries
        .lock()
        .expect("vault")
        .get(&key)
        .cloned()
        .expect("session material");
    SessionMaterial::from_sealed_blob(bytes)
        .decode_host_secrets()
        .expect("decodes")
}

#[tokio::test]
async fn ios_password_login_requests_no_refresh_token_and_restores_without_one() {
    use matrix_sdk::test_utils::mocks::MatrixMockServer;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};
    let server = MatrixMockServer::new().await;
    server
        .mock_versions()
        .with_simplified_sliding_sync()
        .ok()
        .mount()
        .await;
    Mock::given(method("POST"))
        .and(path("/_matrix/client/v3/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user_id": "@alice:example.org",
            "access_token": "syt_ios_login_access",
            "device_id": "IOSDEVICE",
        })))
        .expect(1)
        .mount(server.server())
        .await;
    let identity = AccountIdentity::new("@alice:example.org", &server.uri()).unwrap();
    let entries = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(entries.clone())));
    let root = temp_root("ios-login-no-refresh");
    let root_path = root.to_string_lossy().into_owned();
    let dto = shared
        .login_with_password_inner(
            identity.user_id(),
            identity.homeserver_url(),
            &root_path,
            "fixture-password",
            None,
        )
        .await
        .expect("login");
    assert_eq!(dto.device_id, "IOSDEVICE");

    let requests = server.server().received_requests().await.unwrap();
    let login = requests
        .iter()
        .find(|request| request.url.path() == "/_matrix/client/v3/login")
        .expect("login request");
    let body: serde_json::Value = serde_json::from_slice(&login.body).unwrap();
    assert_eq!(body["type"], "m.login.password");
    assert_ne!(
        body.get("refresh_token"),
        Some(&serde_json::Value::Bool(true)),
        "iOS password login does not opt into refresh tokens"
    );
    let saved = vault_session_secrets(&entries, &identity);
    assert_eq!(saved.access_token, "syt_ios_login_access");
    assert!(!saved.has_refresh_token());

    // Relaunch: restore from the vault without a refresh token.
    drop(shared);
    let relaunched = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(entries)));
    let restored = relaunched
        .restore_persisted_session(
            identity.user_id().to_owned(),
            identity.homeserver_url().to_owned(),
            root_path,
        )
        .await
        .expect("restore without refresh token");
    assert_eq!(restored.device_id, "IOSDEVICE");
    let tokens = relaunched
        .retained_client()
        .await
        .unwrap()
        .session_tokens()
        .expect("tokens");
    assert_eq!(tokens.access_token, "syt_ios_login_access");
    assert!(tokens.refresh_token.is_none());
    drop(relaunched);
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn ios_restore_keeps_refreshing_and_saving_an_existing_refresh_token_session() {
    use matrix_sdk::test_utils::mocks::MatrixMockServer;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, ResponseTemplate};
    let server = MatrixMockServer::new().await;
    server.mock_versions().ok().mount().await;
    let identity = AccountIdentity::new("@alice:example.org", &server.uri()).unwrap();
    // A session saved by a build that still requested refresh tokens.
    let material = SessionMaterial::from_matrix_tokens(
        &identity,
        "DEVICEABC",
        "syt_legacy_expired",
        Some("syr_legacy_refresh"),
    )
    .unwrap();
    let entries = Arc::new(Mutex::new(HashMap::new()));
    persist_session_material(
        &SecretStoreSessionVault {
            store: Arc::new(CallbackSecretVault {
                inner: Box::new(MemoryCallbackVault(entries.clone())),
            }),
        },
        &identity,
        &material,
    )
    .unwrap();
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(entries.clone())));
    let root = temp_root("ios-legacy-refresh");
    shared
        .restore_persisted_session(
            identity.user_id().to_owned(),
            identity.homeserver_url().to_owned(),
            root.to_string_lossy().into_owned(),
        )
        .await
        .expect("restore with refresh token");

    Mock::given(method("GET"))
        .and(path("/_matrix/client/v3/account/whoami"))
        .and(header("authorization", "Bearer syt_legacy_expired"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "errcode": "M_UNKNOWN_TOKEN",
            "error": "Access token has expired",
            "soft_logout": true,
        })))
        .expect(1)
        .mount(server.server())
        .await;
    Mock::given(method("POST"))
        .and(path("/_matrix/client/v3/refresh"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "syt_legacy_rotated",
            "refresh_token": "syr_legacy_rotated",
            "expires_in_ms": 300000,
        })))
        .expect(1)
        .mount(server.server())
        .await;
    Mock::given(method("GET"))
        .and(path("/_matrix/client/v3/account/whoami"))
        .and(header("authorization", "Bearer syt_legacy_rotated"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user_id": "@alice:example.org",
            "device_id": "DEVICEABC",
        })))
        .expect(1)
        .mount(server.server())
        .await;

    shared
        .retained_client()
        .await
        .unwrap()
        .whoami()
        .await
        .expect("the app refreshes the expired token and retries");
    let saved = vault_session_secrets(&entries, &identity);
    assert_eq!(saved.access_token, "syt_legacy_rotated");
    assert_eq!(saved.refresh_token.as_deref(), Some("syr_legacy_rotated"));
    server.server().verify().await;
    drop(shared);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn ios_password_login_never_opts_into_refresh_tokens() {
    let source = include_str!("session_lifecycle.rs");
    let login = source
        .split("async fn login_with_password_inner(")
        .nth(1)
        .and_then(|rest| {
            rest.split("pub async fn persist_planted_session_for_test(")
                .next()
        })
        .expect("login body");
    assert!(login.contains("request_refresh_token: false"));
    assert!(!login.contains("request_refresh_token: true"));
}

#[tokio::test]
async fn ios_password_login_refuses_a_server_without_simplified_sliding_sync() {
    use matrix_sdk::test_utils::mocks::MatrixMockServer;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};
    let server = MatrixMockServer::new().await;
    server.mock_versions().ok().mount().await;
    Mock::given(method("POST"))
        .and(path("/_matrix/client/v3/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user_id": "@alice:example.org",
            "access_token": "syt_never_issued",
            "device_id": "NEVER",
        })))
        .expect(0)
        .mount(server.server())
        .await;
    let identity = AccountIdentity::new("@alice:example.org", &server.uri()).unwrap();
    let entries = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(entries.clone())));
    let root = temp_root("ios-login-no-sliding-sync");
    let error = shared
        .login_with_password_inner(
            identity.user_id(),
            identity.homeserver_url(),
            &root.to_string_lossy(),
            "fixture-password",
            None,
        )
        .await
        .expect_err("no device is created on a server Synara cannot sync with");
    assert_eq!(
        error,
        login_failed(
            LOGIN_SLIDING_SYNC_UNSUPPORTED_CODE,
            LOGIN_SLIDING_SYNC_UNSUPPORTED_DESCRIPTION
        )
    );
    assert!(
        vault_session_entries_are_empty(&entries, &identity),
        "no session saved"
    );
    assert!(!shared.has_retained_client().await);
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn ios_password_login_ignores_a_login_response_well_known() {
    use matrix_sdk::test_utils::mocks::MatrixMockServer;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};
    let server = MatrixMockServer::new().await;
    server
        .mock_versions()
        .with_simplified_sliding_sync()
        .ok()
        .mount()
        .await;
    // Synapse with `serve_client_wellknown` names its public base URL here,
    // which can differ from the client-API URL discovery already resolved.
    Mock::given(method("POST"))
        .and(path("/_matrix/client/v3/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user_id": "@alice:example.org",
            "access_token": "syt_ios_well_known_access",
            "device_id": "IOSWELLKNOWN",
            "well_known": {"m.homeserver": {"base_url": "https://matrix-client.example.org/"}},
        })))
        .expect(1)
        .mount(server.server())
        .await;
    let identity = AccountIdentity::new("@alice:example.org", &server.uri()).unwrap();
    let entries = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(entries.clone())));
    let root = temp_root("ios-login-well-known");
    let dto = shared
        .login_with_password_inner(
            identity.user_id(),
            identity.homeserver_url(),
            &root.to_string_lossy(),
            "fixture-password",
            None,
        )
        .await
        .expect("a login-response well_known does not trip the identity check");
    assert_eq!(dto.homeserver_url, identity.homeserver_url());
    assert_eq!(dto.device_id, "IOSWELLKNOWN");
    assert_eq!(
        shared
            .retained_client()
            .await
            .unwrap()
            .homeserver()
            .as_str(),
        format!("{}/", server.uri())
    );
    drop(shared);
    let _ = fs::remove_dir_all(root);
}

/// `/login` bodies the mock server received, in order.
async fn login_bodies(server: &matrix_sdk::test_utils::mocks::MatrixMockServer) -> Vec<serde_json::Value> {
    server
        .server()
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| request.url.path() == "/_matrix/client/v3/login")
        .map(|request| serde_json::from_slice(&request.body).unwrap())
        .collect()
}

/// The homeserver is reached through `matrix-v4.example.org` (an IPv4 proxy)
/// but is named `matrix.example.org`. A bare username goes out as typed and
/// the account is the `user_id` the server returns, never
/// `@Pete:matrix-v4.example.org`.
#[tokio::test]
async fn ios_localpart_login_behind_a_proxy_host_uses_the_returned_user_id() {
    use matrix_sdk::test_utils::mocks::MatrixMockServer;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};
    let server = MatrixMockServer::new().await;
    server
        .mock_versions()
        .with_simplified_sliding_sync()
        .ok()
        .mount()
        .await;
    Mock::given(method("POST"))
        .and(path("/_matrix/client/v3/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user_id": "@pete:matrix.example.org",
            "access_token": "syt_proxy_localpart_access",
            "device_id": "PETEDEVICE",
        })))
        .expect(1)
        .mount(server.server())
        .await;
    let entries = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(entries.clone())));
    let root = temp_root("ios-login-proxy-localpart");
    let dto = shared
        .login_with_password_inner(
            "Pete",
            &server.uri(),
            &root.to_string_lossy(),
            "fixture-password",
            Some("matrix-v4.example.org"),
        )
        .await
        .expect("login");
    assert_eq!(dto.user_id, "@pete:matrix.example.org");
    assert_eq!(dto.device_id, "PETEDEVICE");

    let bodies = login_bodies(&server).await;
    assert_eq!(
        bodies[0]["identifier"],
        serde_json::json!({"type": "m.id.user", "user": "Pete"})
    );
    let identity = AccountIdentity::new("@pete:matrix.example.org", &server.uri()).unwrap();
    assert_eq!(
        vault_session_secrets(&entries, &identity).access_token,
        "syt_proxy_localpart_access"
    );
    assert_eq!(
        shared
            .retained_client()
            .await
            .unwrap()
            .user_id()
            .unwrap()
            .as_str(),
        "@pete:matrix.example.org"
    );
    let guessed = AccountIdentity::new("@pete:matrix-v4.example.org", &server.uri()).unwrap();
    assert!(!has_account_store(&root, &guessed), "no store for a guessed id");
    assert!(has_account_store(&root, &identity));
    drop(shared);
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn ios_matrix_id_login_behind_a_proxy_host_is_sent_as_typed() {
    use matrix_sdk::test_utils::mocks::MatrixMockServer;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};
    let server = MatrixMockServer::new().await;
    server
        .mock_versions()
        .with_simplified_sliding_sync()
        .ok()
        .mount()
        .await;
    // The server canonicalizes the typed `Pete` to `pete`.
    Mock::given(method("POST"))
        .and(path("/_matrix/client/v3/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user_id": "@pete:matrix.example.org",
            "access_token": "syt_proxy_mxid_access",
            "device_id": "PETEMXID",
        })))
        .expect(1)
        .mount(server.server())
        .await;
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::new(
        Mutex::new(HashMap::new()),
    ))));
    let root = temp_root("ios-login-proxy-mxid");
    let dto = shared
        .login_with_password_inner(
            "@Pete:matrix.example.org",
            &server.uri(),
            &root.to_string_lossy(),
            "fixture-password",
            Some("matrix-v4.example.org"),
        )
        .await
        .expect("login");
    assert_eq!(dto.user_id, "@pete:matrix.example.org");
    let bodies = login_bodies(&server).await;
    assert_eq!(bodies[0]["identifier"]["user"], "@Pete:matrix.example.org");
    drop(shared);
    let _ = fs::remove_dir_all(root);
}

/// A returning user behind the proxy: the guess finds no store, the server
/// names an account that has one, so the new device is dropped and the
/// sign-in repeats with the stored device.
#[tokio::test]
async fn ios_localpart_login_reuses_the_device_of_the_returned_account() {
    use matrix_sdk::test_utils::mocks::MatrixMockServer;
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, ResponseTemplate};
    let server = MatrixMockServer::new().await;
    server
        .mock_versions()
        .with_simplified_sliding_sync()
        .ok()
        .mount()
        .await;
    let login_response = |device: &str, token: &str| {
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user_id": "@pete:matrix.example.org",
            "access_token": token,
            "device_id": device,
        }))
    };
    Mock::given(method("POST"))
        .and(path("/_matrix/client/v3/login"))
        .and(body_partial_json(serde_json::json!({"device_id": "PETEFIRST"})))
        .respond_with(login_response("PETEFIRST", "syt_reused"))
        .with_priority(1)
        .expect(1)
        .mount(server.server())
        .await;
    Mock::given(method("POST"))
        .and(path("/_matrix/client/v3/login"))
        .respond_with(login_response("PETEFIRST", "syt_first"))
        .up_to_n_times(1)
        .with_priority(2)
        .mount(server.server())
        .await;
    Mock::given(method("POST"))
        .and(path("/_matrix/client/v3/login"))
        .respond_with(login_response("PETEEXTRA", "syt_extra"))
        .with_priority(3)
        .mount(server.server())
        .await;
    Mock::given(method("POST"))
        .and(path("/_matrix/client/v3/logout"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .expect(1)
        .mount(server.server())
        .await;
    let entries = Arc::new(Mutex::new(HashMap::new()));
    let root = temp_root("ios-login-proxy-reuse");
    let login = |shared: SharedCore| {
        let root = root.to_string_lossy().into_owned();
        let uri = server.uri();
        async move {
            let dto = shared
                .login_with_password_inner(
                    "pete",
                    &uri,
                    &root,
                    "fixture-password",
                    Some("matrix-v4.example.org"),
                )
                .await;
            drop(shared);
            dto
        }
    };
    let first = login(SharedCore::new_with_secret_store(Box::new(
        MemoryCallbackVault(entries.clone()),
    )))
    .await
    .expect("first login");
    assert_eq!(first.device_id, "PETEFIRST");
    // The app quit; the account store and its device stay on disk.
    let second = login(SharedCore::new_with_secret_store(Box::new(
        MemoryCallbackVault(entries.clone()),
    )))
    .await
    .expect("second login");
    assert_eq!(second.user_id, "@pete:matrix.example.org");
    assert_eq!(second.device_id, "PETEFIRST");

    let bodies = login_bodies(&server).await;
    assert_eq!(bodies.len(), 3, "first, extra (revoked), reused");
    assert!(bodies[..2].iter().all(|body| body.get("device_id").is_none()));
    assert_eq!(bodies[2]["device_id"], "PETEFIRST");
    assert!(bodies.iter().all(|body| body["identifier"]["user"] == "pete"));
    let logout = server
        .server()
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .find(|request| request.url.path() == "/_matrix/client/v3/logout")
        .expect("extra device revoked");
    assert_eq!(
        logout.headers.get("authorization").unwrap(),
        "Bearer syt_extra"
    );
    let _ = fs::remove_dir_all(root);
}

fn vault_session_entries_are_empty(
    entries: &Mutex<HashMap<String, Vec<u8>>>,
    identity: &AccountIdentity,
) -> bool {
    let key = SessionMaterialId::from_identity(identity)
        .account()
        .to_owned();
    !entries.lock().expect("vault").contains_key(&key)
}

fn store_key_entry(
    entries: &Mutex<HashMap<String, Vec<u8>>>,
    identity: &AccountIdentity,
) -> Option<Vec<u8>> {
    let key = StoreKeyId::from_identity(identity).account().to_owned();
    entries.lock().expect("vault").get(&key).cloned()
}

/// L23: a store restored to a new device (app-group data backed up, Keychain
/// item `ThisDeviceOnly` not) must not get a fresh key that can never open
/// it. Sign-in and restore ask for a reset; the reset removes only that
/// account's store and key; a fresh key is created once and then reused.
#[tokio::test]
async fn ios_existing_store_without_its_key_asks_for_reset_instead_of_a_new_key() {
    let identity = alice();
    let bob = AccountIdentity::new("@bob:example.org", "https://matrix.example.org").unwrap();
    let entries = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(entries.clone())));
    let root = temp_root("store-key-missing");
    let root_path = root.to_string_lossy().into_owned();
    let alice_paths = StorePaths::derive(&root, &identity).unwrap();
    alice_paths.ensure_dirs().unwrap();
    let bob_paths = StorePaths::derive(&root, &bob).unwrap();
    bob_paths.ensure_dirs().unwrap();

    let login = shared
        .login_with_password_inner(
            identity.user_id(),
            identity.homeserver_url(),
            &root_path,
            "fixture-password",
            None,
        )
        .await
        .expect_err("no key for an existing store");
    match login {
        SessionLoginError::Failed { code, .. } => {
            assert_eq!(code, LOGIN_STORE_RESET_REQUIRED_CODE)
        }
    }
    assert!(store_key_entry(&entries, &identity).is_none());

    let material =
        SessionMaterial::from_matrix_tokens(&identity, "IOSDEVICE", "syt_s3b_restore", None)
            .unwrap();
    let session_vault = SecretStoreSessionVault {
        store: Arc::clone(&shared.secret_store),
    };
    persist_session_material(&session_vault, &identity, &material).unwrap();
    let restore = shared
        .restore_persisted_session(
            identity.user_id().to_owned(),
            identity.homeserver_url().to_owned(),
            root_path.clone(),
        )
        .await
        .expect_err("restore asks for the same reset");
    assert!(format!("{restore:?}").contains(STORE_RESET_REQUIRED_CODE));
    assert!(store_key_entry(&entries, &identity).is_none());

    let reset = shared
        .reset_local_account_store(
            identity.user_id().to_owned(),
            identity.homeserver_url().to_owned(),
            root_path.clone(),
        )
        .await
        .unwrap();
    assert_eq!(reset.status, LeftoverAckStatusDto::Wiped);
    assert!(!alice_paths.account_root().exists());
    assert!(bob_paths.account_root().exists(), "other accounts stay");

    let created = store_key_for(&shared.secret_store, &root, &identity).unwrap();
    let reused = store_key_for(&shared.secret_store, &root, &identity).unwrap();
    assert_eq!(created.as_bytes(), reused.as_bytes());
    assert_eq!(
        store_key_entry(&entries, &identity).as_deref(),
        Some(created.as_bytes().as_slice())
    );
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn ios_existing_store_reuses_its_stored_key() {
    let identity = alice();
    let entries = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(entries.clone())));
    let root = temp_root("store-key-reuse");
    let first = store_key_for(&shared.secret_store, &root, &identity).unwrap();
    StorePaths::derive(&root, &identity)
        .unwrap()
        .ensure_dirs()
        .unwrap();
    let again = store_key_for(&shared.secret_store, &root, &identity).unwrap();
    assert_eq!(first.as_bytes(), again.as_bytes());
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn ios_store_reset_is_refused_while_a_session_is_retained() {
    use matrix_sdk::test_utils::mocks::MatrixMockServer;
    let server = MatrixMockServer::new().await;
    let entries = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(entries.clone())));
    let root = temp_root("store-reset-live");
    let root_path = root.to_string_lossy().into_owned();
    shared
        .persist_planted_session_for_test(
            "@alice:example.org".into(),
            server.server().uri(),
            root_path.clone(),
            "DEVICE".into(),
            "fixture-token".into(),
            None,
        )
        .await
        .unwrap();
    let identity = AccountIdentity::new("@alice:example.org", &server.server().uri()).unwrap();
    assert!(shared
        .reset_local_account_store(
            identity.user_id().to_owned(),
            identity.homeserver_url().to_owned(),
            root_path,
        )
        .await
        .is_err());
    assert!(StorePaths::derive(&root, &identity)
        .unwrap()
        .account_root()
        .exists());
    assert!(store_key_entry(&entries, &identity).is_some());
    drop(shared);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn ios_login_errors_keep_their_cause_instead_of_wrong_credentials() {
    let code = |error: AuthError| match login_auth_error(&error) {
        SessionLoginError::Failed { code, .. } => code,
    };
    assert_eq!(
        code(AuthError::AuthenticationRejected {
            diagnostic_id: "p3.2-login-rejected"
        }),
        LOGIN_CREDENTIALS_REJECTED_CODE
    );
    assert_eq!(
        code(AuthError::Connectivity {
            diagnostic_id: "p3.2-login-http-connect"
        }),
        LOGIN_NETWORK_UNAVAILABLE_CODE
    );
    assert_eq!(
        code(AuthError::HomeserverUnavailable {
            diagnostic_id: "p3.2-login-homeserver-unavailable"
        }),
        LOGIN_SERVER_UNAVAILABLE_CODE
    );
    assert_eq!(
        code(AuthError::RateLimited {
            diagnostic_id: "p3.2-login-rate-limited",
            retry_after_ms: Some(2_000),
        }),
        LOGIN_RATE_LIMITED_CODE
    );
    assert_eq!(
        code(AuthError::UserDeactivated {
            diagnostic_id: "p3.2-login-user-deactivated"
        }),
        LOGIN_USER_DEACTIVATED_CODE
    );
    for unsupported in [
        AuthError::UnsupportedCapability {
            diagnostic_id: "p3.2-login-unrecognized",
        },
        AuthError::InteractiveAuthRequired {
            diagnostic_id: "p3.2-login-uiaa-required",
        },
    ] {
        assert_eq!(code(unsupported), LOGIN_UNSUPPORTED_CODE);
    }
    assert_eq!(
        code(AuthError::SdkInvariant {
            diagnostic_id: "p3.2-login-store-reset-required"
        }),
        LOGIN_STORE_RESET_REQUIRED_CODE
    );
    assert_eq!(
        code(AuthError::SdkInvariant {
            diagnostic_id: "p3.2-login-store-locked"
        }),
        LOGIN_STORE_LOCKED_CODE
    );
    for unclassified in [
        AuthError::SdkInvariant {
            diagnostic_id: "p3.2-login-olm-unavailable",
        },
        AuthError::Unknown {
            diagnostic_id: "p3.2-login-unknown",
        },
    ] {
        assert_eq!(code(unclassified), LOGIN_FAILED_CODE);
    }
}

#[tokio::test]
async fn ios_password_login_reports_rejection_rate_limit_and_outage_distinctly() {
    use matrix_sdk::test_utils::mocks::MatrixMockServer;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};
    let cases = [
        (
            ResponseTemplate::new(403).set_body_json(serde_json::json!({
                "errcode": "M_FORBIDDEN",
                "error": "Invalid password",
            })),
            LOGIN_CREDENTIALS_REJECTED_CODE,
        ),
        (
            ResponseTemplate::new(429).set_body_json(serde_json::json!({
                "errcode": "M_LIMIT_EXCEEDED",
                "error": "Too many requests",
                "retry_after_ms": 2000,
            })),
            LOGIN_RATE_LIMITED_CODE,
        ),
        (
            ResponseTemplate::new(403).set_body_json(serde_json::json!({
                "errcode": "M_USER_DEACTIVATED",
                "error": "This account has been deactivated",
            })),
            LOGIN_USER_DEACTIVATED_CODE,
        ),
    ];
    for (index, (response, expected)) in cases.into_iter().enumerate() {
        let server = MatrixMockServer::new().await;
        server
            .mock_versions()
            .with_simplified_sliding_sync()
            .ok()
            .mount()
            .await;
        Mock::given(method("POST"))
            .and(path("/_matrix/client/v3/login"))
            .respond_with(response)
            .mount(server.server())
            .await;
        let identity = AccountIdentity::new("@alice:example.org", &server.uri()).unwrap();
        let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::new(
            Mutex::new(HashMap::new()),
        ))));
        let root = temp_root(&format!("ios-login-error-{index}"));
        let error = shared
            .login_with_password_inner(
                identity.user_id(),
                identity.homeserver_url(),
                &root.to_string_lossy(),
                "fixture-password",
                None,
            )
            .await
            .expect_err("login fails");
        match error {
            SessionLoginError::Failed {
                code, description, ..
            } => {
                assert_eq!(code, expected, "case {index}");
                assert!(!description.contains("fixture-password"));
            }
        }
        drop(shared);
        let _ = fs::remove_dir_all(root);
    }

    // A proxy in front of a down homeserver: `/versions` answers 502.
    let server = MatrixMockServer::new().await;
    Mock::given(method("GET"))
        .and(path("/_matrix/client/versions"))
        .respond_with(ResponseTemplate::new(502).set_body_string("Bad Gateway"))
        .mount(server.server())
        .await;
    let identity = AccountIdentity::new("@alice:example.org", &server.uri()).unwrap();
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::new(
        Mutex::new(HashMap::new()),
    ))));
    let root = temp_root("ios-login-error-outage");
    let error = shared
        .login_with_password_inner(
            identity.user_id(),
            identity.homeserver_url(),
            &root.to_string_lossy(),
            "fixture-password",
            None,
        )
        .await
        .expect_err("server down");
    match error {
        SessionLoginError::Failed { code, .. } => {
            assert_eq!(code, LOGIN_SERVER_UNAVAILABLE_CODE)
        }
    }
    drop(shared);
    let _ = fs::remove_dir_all(root);

    // Nothing listens on the homeserver port: the `/versions` probe fails as
    // a connection error, not as wrong credentials.
    let unreachable = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        format!("http://127.0.0.1:{port}")
    };
    let identity = AccountIdentity::new("@alice:example.org", &unreachable).unwrap();
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::new(
        Mutex::new(HashMap::new()),
    ))));
    let root = temp_root("ios-login-error-unreachable");
    let error = shared
        .login_with_password_inner(
            identity.user_id(),
            identity.homeserver_url(),
            &root.to_string_lossy(),
            "fixture-password",
            None,
        )
        .await
        .expect_err("no server");
    match error {
        SessionLoginError::Failed { code, .. } => {
            assert_eq!(code, LOGIN_NETWORK_UNAVAILABLE_CODE)
        }
    }
    drop(shared);
    let _ = fs::remove_dir_all(root);
}
