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
    assert!(shared.retained_client().is_err());
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
fn session_status_oversize_payload_fails_closed_without_truncate_or_echo() {
    let marker = "s931OversizeMarker";
    let payload = serde_json::json!({
        "pad": format!("{marker}{}", "x".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8))
    });
    let error = session_status_envelope_payload(payload)
        .expect_err("oversize session/status payload must fail closed");
    let text = format!("{error:?}{error}");
    assert!(text.contains(SESSION_STATUS_FAILED_CODE));
    assert!(!text.contains(marker));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("@alice"));
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
    let client = shared.retained_client().expect("retained Client");

    let stopped = rt
        .block_on(shared.stop_sync())
        .expect("stop must complete the full store quiescence boundary");
    assert!(stopped.stopped);
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
fn nse_store_key_lookup_never_mints_a_missing_key() {
    let values = std::sync::Arc::new(Mutex::new(HashMap::new()));
    let store: Arc<dyn SecretVault + Send + Sync> = Arc::new(CallbackSecretVault {
        inner: Box::new(MemoryCallbackVault(std::sync::Arc::clone(&values))),
    });

    let error = store_key_for_read_only(&store, &alice()).expect_err("missing key");

    assert!(matches!(
        error,
        SessionRestoreError::Failed { ref code, .. } if code == RESTORE_FAILED_CODE
    ));
    assert!(values.lock().expect("vault").is_empty());
}

#[test]
fn nse_store_key_lookup_returns_the_existing_current_key() {
    let values = std::sync::Arc::new(Mutex::new(HashMap::new()));
    let store: Arc<dyn SecretVault + Send + Sync> = Arc::new(CallbackSecretVault {
        inner: Box::new(MemoryCallbackVault(std::sync::Arc::clone(&values))),
    });
    let identity = alice();
    let expected = StoreKeyMaterial::from_bytes([7; STORE_KEY_LEN]);
    values.lock().expect("vault").insert(
        StoreKeyId::from_identity(&identity).account().to_owned(),
        expected.as_bytes().to_vec(),
    );

    let actual = store_key_for_read_only(&store, &identity).expect("existing key");

    assert_eq!(actual.as_bytes(), expected.as_bytes());
    assert_eq!(values.lock().expect("vault").len(), 1);
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
    assert!(matches!(
        *shared.restored_client.lock().expect("client"),
        RestoredClientSlot::Ready(..)
    ));
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
    assert!(matches!(
        *shared.restored_client.lock().expect("client"),
        RestoredClientSlot::Ready(..)
    ));
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
    assert!(format!("{error:?}").contains(RESTORE_FAILED_CODE));
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
            transaction_id: None,
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
    let text = format!("{dto:?}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("password"));
    assert!(text.contains("mxc://example.org/alice"));
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
                transaction_id: None,
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
            transaction_id: None,
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
        transaction_id: None,
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
    let lease = match &*shared.restored_client.lock().expect("client") {
        RestoredClientSlot::Ready(_, persistence) => persistence.callback_lease(),
        _ => panic!("planted session is retained"),
    };
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
