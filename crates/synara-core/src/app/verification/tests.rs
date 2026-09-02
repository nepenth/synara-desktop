//! Unit tests for P8.3 verification inbox.

use super::*;
use crate::app::sync::{build_sync_service, SyncServiceConfig};
use crate::transport::MatrixIpcErrorCategory;
use matrix_sdk::{
    encryption::VerificationState,
    ruma::api::client::uiaa::{AuthData, AuthType, MatrixUserIdentifier, Password, UserIdentifier},
    store::RoomLoadSettings,
    Client,
};
use std::{path::PathBuf, time::Duration};

fn flow(id: &str, phase: VerificationPhase) -> VerificationFlow {
    VerificationFlow {
        flow_id: id.into(),
        other_user_id: "@alice:example.org".into(),
        other_device_id: "DEVICEA".into(),
        direction: VerificationDirection::Incoming,
        phase,
        started_ts: Some(1),
        sas_emoji: None,
        failure_diagnostic_id: None,
    }
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_verification_markers(), MATRIX_VERIFICATION_MARKER);
}

#[test]
fn upsert_list_open_order() {
    let mut inbox = VerificationInbox::new(1);
    inbox
        .upsert(flow("f-ready", VerificationPhase::Ready))
        .unwrap();
    inbox
        .upsert(flow("f-req", VerificationPhase::Requested))
        .unwrap();
    inbox
        .upsert(flow("f-done", VerificationPhase::Done))
        .unwrap();
    let open = inbox.list_open();
    assert_eq!(open.len(), 2);
    assert_eq!(open[0].flow_id, "f-req");
    assert_eq!(open[1].flow_id, "f-ready");
    assert!(inbox.has_pending_attention());
    assert_eq!(inbox.open_count(), 2);
}

#[test]
fn sas_confirm_complete() {
    let mut inbox = VerificationInbox::new(1);
    inbox
        .upsert(flow("f1", VerificationPhase::Requested))
        .unwrap();
    inbox
        .mark_ready(
            "f1",
            Some(vec![
                "dog".into(),
                "cat".into(),
                "tree".into(),
                "book".into(),
                "clock".into(),
                "smiley".into(),
                "heart".into(),
            ]),
        )
        .unwrap();
    let f = inbox.get("f1").unwrap();
    assert_eq!(f.phase, VerificationPhase::Ready);
    assert_eq!(f.sas_emoji.as_ref().unwrap().len(), 7);
    inbox.confirm("f1").unwrap();
    assert_eq!(inbox.get("f1").unwrap().phase, VerificationPhase::Confirmed);
    inbox.complete("f1").unwrap();
    let f = inbox.get("f1").unwrap();
    assert_eq!(f.phase, VerificationPhase::Done);
    assert!(f.sas_emoji.is_none());
    assert!(!inbox.has_pending_attention());
}

#[test]
fn mismatch_cancel_fail() {
    let mut inbox = VerificationInbox::new(1);
    inbox
        .upsert(flow("a", VerificationPhase::Requested))
        .unwrap();
    inbox.mark_ready("a", None).unwrap();
    inbox.mismatch("a").unwrap();
    assert_eq!(inbox.get("a").unwrap().phase, VerificationPhase::Mismatched);
    inbox.cancel("a").unwrap();
    assert_eq!(inbox.get("a").unwrap().phase, VerificationPhase::Cancelled);

    inbox
        .upsert(flow("b", VerificationPhase::Requested))
        .unwrap();
    inbox.fail("b", "p8.3-host-timeout").unwrap();
    let f = inbox.get("b").unwrap();
    assert_eq!(f.phase, VerificationPhase::Failed);
    assert_eq!(f.failure_diagnostic_id, Some("p8.3-host-timeout"));
}

#[test]
fn invalid_ids_and_phase() {
    let mut inbox = VerificationInbox::new(1);
    let err = inbox
        .upsert(VerificationFlow {
            flow_id: "".into(),
            other_user_id: "@a:ex".into(),
            other_device_id: "D".into(),
            direction: VerificationDirection::Outgoing,
            phase: VerificationPhase::Requested,
            started_ts: None,
            sas_emoji: None,
            failure_diagnostic_id: None,
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.3-invalid-flow-id");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);

    let err = inbox
        .upsert(VerificationFlow {
            flow_id: "ok".into(),
            other_user_id: "not-a-mxid".into(),
            other_device_id: "D".into(),
            direction: VerificationDirection::Outgoing,
            phase: VerificationPhase::Requested,
            started_ts: None,
            sas_emoji: None,
            failure_diagnostic_id: None,
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.3-invalid-user-id");

    inbox
        .upsert(flow("f", VerificationPhase::Requested))
        .unwrap();
    let err = inbox.confirm("f").unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.3-invalid-phase-transition");
}

#[test]
fn open_cap() {
    let mut inbox = VerificationInbox::new(1);
    for i in 0..MAX_OPEN_FLOWS {
        inbox
            .upsert(flow(&format!("f{i}"), VerificationPhase::Requested))
            .unwrap();
    }
    let err = inbox
        .upsert(flow("overflow", VerificationPhase::Requested))
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.3-open-flow-cap");
    // Terminal flows do not count toward open cap.
    inbox
        .upsert(flow("done-extra", VerificationPhase::Done))
        .unwrap();
}

#[test]
fn retire_generation() {
    let mut inbox = VerificationInbox::new(2);
    inbox.upsert(flow("x", VerificationPhase::Ready)).unwrap();
    inbox.retire_generation(3);
    assert_eq!(inbox.session_generation(), 3);
    assert!(inbox.is_empty());
    assert!(!inbox.has_pending_attention());
}

#[test]
fn self_verification_start_uses_the_own_identity_and_never_substitutes_peer_trust() {
    let source = include_str!("live.rs");
    assert!(source.contains("fn start_self_verification"));
    assert!(source.contains("query_own_identity"));
    let helper = source
        .split("async fn start_self_verification")
        .nth(1)
        .and_then(|rest| rest.split("async fn register_incoming_request").next())
        .expect("start_self_verification helper");
    assert!(helper.contains("query_own_identity"));
    assert!(helper.contains("v-crypto.1-own-identity-not-found"));
    assert!(helper.contains("request_verification_with_methods"));
    assert!(!helper.contains("get_user_devices"));
}

#[test]
fn transitioned_sas_is_owner_accepted_for_both_directions_and_confirmed_is_stable() {
    let source = include_str!("live.rs");
    let watcher = source
        .split("async fn watch_request")
        .nth(1)
        .and_then(|rest| rest.split("fn project_request").next())
        .expect("verification watcher");
    assert!(watcher.contains("accept_transitioned_sas(&flow_id, sas).await"));
    assert!(watcher.contains("managed.owner_failed |= accept_failed"));
    assert!(watcher.contains("sas_owner_accept_failed"));
    assert!(!watcher.contains("NativeVerificationDirection"));

    let projection = source
        .split("fn project_request")
        .nth(1)
        .and_then(|rest| rest.split("fn refresh_sas").next())
        .expect("verification projection");
    let confirmed = projection
        .find("SasState::Confirmed")
        .expect("confirmed projection");
    let presentable = projection
        .find("sas.can_be_presented()")
        .expect("SAS projection");
    assert!(
        confirmed < presentable,
        "Confirmed must not regress to Started"
    );
}

#[test]
fn verification_update_signal_is_session_wake_only() {
    assert_eq!(VERIFICATION_UPDATED_EVENT, "matrix-verification-updated");
    let encoded = serde_json::to_string(&NativeVerificationUpdateSignal {
        session_generation: 7,
    })
    .expect("serialize wake-up");
    assert_eq!(encoded, "{\"sessionGeneration\":7}");
    for forbidden in ["key", "token", "mac", "secret", "recovery", "ciphertext"] {
        assert!(!encoded.to_ascii_lowercase().contains(forbidden));
    }
}

#[test]
fn live_verification_diagnostics_use_an_opaque_flow_tag() {
    let flow_id = "sensitive-transaction-id";
    let tag = live::verification_flow_tag(flow_id);
    assert_eq!(tag.len(), 12);
    assert!(tag.chars().all(|character| character.is_ascii_hexdigit()));
    assert!(!tag.contains(flow_id));
    assert_eq!(tag, live::verification_flow_tag(flow_id));
    assert_ne!(tag, live::verification_flow_tag("another-flow"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires SYNARA_LIVE_HOMESERVER, SYNARA_LIVE_USERNAME, and SYNARA_LIVE_PASSWORD"]
async fn live_direct_peer_sas_transport_completes_through_product_owner_and_sync() {
    let homeserver =
        std::env::var("SYNARA_LIVE_HOMESERVER").expect("SYNARA_LIVE_HOMESERVER is required");
    let username = std::env::var("SYNARA_LIVE_USERNAME").expect("SYNARA_LIVE_USERNAME is required");
    let password = std::env::var("SYNARA_LIVE_PASSWORD").expect("SYNARA_LIVE_PASSWORD is required");
    let store_passphrase = std::env::var("SYNARA_LIVE_VERIFICATION_STORE_PASSPHRASE")
        .expect("SYNARA_LIVE_VERIFICATION_STORE_PASSPHRASE is required");
    let proof_root = live_proof_root();
    let initiator =
        live_proof_client(&homeserver, proof_root.join("initiator"), &store_passphrase).await;
    let responder =
        live_proof_client(&homeserver, proof_root.join("responder"), &store_passphrase).await;

    initiator
        .matrix_auth()
        .login_username(&username, &password)
        .initial_device_display_name("Synara SAS proof initiator")
        .send()
        .await
        .expect("initiator login");
    responder
        .matrix_auth()
        .login_username(&username, &password)
        .initial_device_display_name("Synara SAS proof responder")
        .send()
        .await
        .expect("responder login");

    let initiator_owner = NativeVerificationOwner::new(&initiator, 1);
    let responder_owner = NativeVerificationOwner::new(&responder, 1);
    let initiator_sync = build_sync_service(&initiator, 1, SyncServiceConfig::default())
        .await
        .expect("initiator sync build");
    let responder_sync = build_sync_service(&responder, 1, SyncServiceConfig::default())
        .await
        .expect("responder sync build");
    initiator_sync.start().await.expect("initiator sync start");
    responder_sync.start().await.expect("responder sync start");

    let responder_device = responder
        .device_id()
        .expect("responder device id")
        .to_string();
    let initiator_device = initiator
        .device_id()
        .expect("initiator device id")
        .to_string();
    // The crypto state machine must know both endpoints before the request.
    // Seeing only the target from the initiator is insufficient: the responder
    // drops an incoming request whose sender device key is not in its store.
    wait_for_live_device(&initiator, &responder_device).await;
    wait_for_live_device(&responder, &initiator_device).await;
    let started = initiator_owner
        .start(Some(responder_device.clone()))
        .await
        .expect("direct peer verification request");
    let flow_id = started.flow_id;
    wait_for_live_phase(
        &responder_owner,
        &flow_id,
        NativeVerificationPhase::Requested,
    )
    .await;
    responder_owner
        .accept(&flow_id)
        .await
        .expect("responder accepts request");
    wait_for_live_phase(&initiator_owner, &flow_id, NativeVerificationPhase::Ready).await;
    initiator_owner
        .begin_sas(&flow_id)
        .await
        .expect("initiator starts SAS");

    // Owner acceptance can coalesce Started into Accepted/KeysExchanged before
    // a polling client samples it. SasReady is the first required stable
    // presentation state; do not turn an unobservable transient into a gate.
    let initiator_sas = wait_for_live_phase(
        &initiator_owner,
        &flow_id,
        NativeVerificationPhase::SasReady,
    )
    .await;
    let responder_sas = wait_for_live_phase(
        &responder_owner,
        &flow_id,
        NativeVerificationPhase::SasReady,
    )
    .await;
    assert_eq!(
        initiator_sas.sas, responder_sas.sas,
        "both devices must present identical SAS"
    );

    initiator_owner
        .confirm(&flow_id)
        .await
        .expect("initiator confirms SAS");
    responder_owner
        .confirm(&flow_id)
        .await
        .expect("responder confirms SAS");
    wait_for_live_phase(&initiator_owner, &flow_id, NativeVerificationPhase::Done).await;
    wait_for_live_phase(&responder_owner, &flow_id, NativeVerificationPhase::Done).await;

    let user_id = initiator.user_id().expect("signed-in user");
    let peer = initiator
        .encryption()
        .get_device(user_id, responder.device_id().expect("responder device id"))
        .await
        .expect("peer device query")
        .expect("peer device");
    assert!(
        peer.is_verified(),
        "SDK trust readback must report the peer verified"
    );
    let device_snapshot = crate::app::devices::snapshot(&initiator, 1)
        .await
        .expect("project verified device snapshot");
    let projected_peer = device_snapshot
        .devices
        .iter()
        .find(|device| device.device_id == responder_device)
        .expect("verified peer in product device snapshot");
    assert_eq!(
        projected_peer.trust,
        crate::app::devices::NativeDeviceTrust::Verified,
        "product trust projection must preserve direct SAS verification"
    );
    // This fixture intentionally proves only direct-peer SAS transport. Two
    // fresh sessions are not eligible authorities for the SDK own-identity
    // route, and peer trust must never be reported as proof that the current
    // device became cross-signed. The production proof separately requires
    // `DeviceSnapshot::own_verification == Verified` after a nil-target request
    // to an already cross-signed authority session.

    initiator_sync.stop().await.expect("initiator sync stop");
    responder_sync.stop().await.expect("responder sync stop");
    initiator
        .matrix_auth()
        .logout()
        .await
        .expect("initiator logout");
    responder
        .matrix_auth()
        .logout()
        .await
        .expect("responder logout");
    std::fs::remove_dir_all(&proof_root).expect("remove disposable proof stores");
}

/// Full production-route proof for current-device verification.
///
/// The responder store is intentionally durable across proof runs. If the
/// account has no published cross-signing identity, the first run bootstraps
/// one through password UIA and persists its private identity. If an identity
/// already exists but this store does not contain its matching private keys,
/// the proof fails rather than replacing the account's real authority.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires authorized SYNARA_LIVE_* credentials and mutates test-account cross-signing/device state"]
async fn live_own_device_verification_is_authoritative_and_durable() {
    let homeserver =
        std::env::var("SYNARA_LIVE_HOMESERVER").expect("SYNARA_LIVE_HOMESERVER is required");
    let username = std::env::var("SYNARA_LIVE_USERNAME").expect("SYNARA_LIVE_USERNAME is required");
    let password = std::env::var("SYNARA_LIVE_PASSWORD").expect("SYNARA_LIVE_PASSWORD is required");
    let store_passphrase = std::env::var("SYNARA_LIVE_VERIFICATION_STORE_PASSPHRASE")
        .expect("SYNARA_LIVE_VERIFICATION_STORE_PASSPHRASE is required");
    let authority_root = live_authority_root(&homeserver, &username);
    let authority_store = authority_root.join("responder-store");
    let authority_device_file = authority_root.join("responder-device-id");
    let authority_store_preexisted = authority_store.exists();
    let persisted_device_id = std::fs::read_to_string(&authority_device_file)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    assert!(
        !authority_store_preexisted || persisted_device_id.is_some(),
        "persisted authority store is missing its non-secret device metadata"
    );
    create_private_proof_directory(&authority_root);

    let responder = live_proof_client(&homeserver, authority_store, &store_passphrase).await;
    let responder_login = responder
        .matrix_auth()
        .login_username(&username, &password)
        .initial_device_display_name("Synara persisted verification authority");
    let responder_login = match persisted_device_id.as_deref() {
        Some(device_id) => responder_login.device_id(device_id),
        None => responder_login,
    };
    responder_login
        .send()
        .await
        .expect("persisted responder login");
    let responder_device_id = responder
        .device_id()
        .expect("persisted responder device id")
        .to_string();
    std::fs::write(&authority_device_file, &responder_device_id)
        .expect("persist responder device metadata");
    set_private_file_permissions(&authority_device_file);

    let responder_owner = NativeVerificationOwner::new(&responder, 1);
    let responder_sync = build_sync_service(&responder, 1, SyncServiceConfig::default())
        .await
        .expect("responder sync build");
    responder_sync.start().await.expect("responder sync start");
    ensure_live_responder_authority(&responder, &password).await;
    eprintln!("synara_own_device_proof checkpoint=responder_authority_verified");

    let initiator_root = live_proof_root();
    let initiator_store = initiator_root.join("initiator-store");
    let initiator =
        live_proof_client(&homeserver, initiator_store.clone(), &store_passphrase).await;
    initiator
        .matrix_auth()
        .login_username(&username, &password)
        .initial_device_display_name("Synara fresh OwnIdentity proof initiator")
        .send()
        .await
        .expect("fresh initiator login");
    let initiator_session = initiator
        .matrix_auth()
        .session()
        .expect("fresh initiator Matrix session");
    let initiator_owner = NativeVerificationOwner::new(&initiator, 1);
    let initiator_sync = build_sync_service(&initiator, 1, SyncServiceConfig::default())
        .await
        .expect("initiator sync build");
    initiator_sync.start().await.expect("initiator sync start");

    let initiator_device_id = initiator
        .device_id()
        .expect("fresh initiator device id")
        .to_string();
    wait_for_live_device(&initiator, &responder_device_id).await;
    wait_for_live_device(&responder, &initiator_device_id).await;
    wait_for_fresh_initiator_authority(&initiator).await;
    eprintln!("synara_own_device_proof checkpoint=fresh_initiator_eligible");

    // The nil target is the actual product route. Supplying the responder
    // device id here would prove only direct peer trust and is disqualifying.
    let started = initiator_owner
        .start(None)
        .await
        .expect("OwnUserIdentity verification request");
    let flow_id = started.flow_id;
    wait_for_live_phase(
        &responder_owner,
        &flow_id,
        NativeVerificationPhase::Requested,
    )
    .await;
    responder_owner
        .accept(&flow_id)
        .await
        .expect("authority accepts own-device request");
    wait_for_live_phase(&initiator_owner, &flow_id, NativeVerificationPhase::Ready).await;
    initiator_owner
        .begin_sas(&flow_id)
        .await
        .expect("initiator starts SAS");

    let initiator_sas = wait_for_live_phase(
        &initiator_owner,
        &flow_id,
        NativeVerificationPhase::SasReady,
    )
    .await;
    let responder_sas = wait_for_live_phase(
        &responder_owner,
        &flow_id,
        NativeVerificationPhase::SasReady,
    )
    .await;
    assert_eq!(
        initiator_sas.sas, responder_sas.sas,
        "OwnIdentity participants must present identical SAS"
    );
    eprintln!("synara_own_device_proof checkpoint=sas_matched");

    initiator_owner
        .confirm(&flow_id)
        .await
        .expect("initiator confirms SAS");
    responder_owner
        .confirm(&flow_id)
        .await
        .expect("authority confirms SAS");
    wait_for_live_phase(&initiator_owner, &flow_id, NativeVerificationPhase::Done).await;
    wait_for_live_phase(&responder_owner, &flow_id, NativeVerificationPhase::Done).await;
    wait_for_live_verification_state(&initiator, VerificationState::Verified).await;
    assert_eq!(
        crate::app::devices::snapshot(&initiator, 1)
            .await
            .expect("initiator authoritative device snapshot")
            .own_verification,
        crate::app::devices::NativeOwnDeviceVerification::Verified
    );
    eprintln!("synara_own_device_proof checkpoint=initiator_verified");

    initiator_sync.stop().await.expect("initiator sync stop");
    drop(initiator_owner);
    drop(initiator_sync);
    drop(initiator);

    // Rebuild the same product crypto store and restore the exact Matrix
    // session. This is the durable owner readback, independent of the flow.
    let rebuilt = live_proof_client(&homeserver, initiator_store, &store_passphrase).await;
    rebuilt
        .matrix_auth()
        .restore_session(initiator_session, RoomLoadSettings::default())
        .await
        .expect("restore initiator session into rebuilt client");
    let rebuilt_sync = build_sync_service(&rebuilt, 2, SyncServiceConfig::default())
        .await
        .expect("rebuilt initiator sync build");
    rebuilt_sync
        .start()
        .await
        .expect("rebuilt initiator sync start");
    wait_for_live_verification_state(&rebuilt, VerificationState::Verified).await;
    assert_eq!(
        crate::app::devices::snapshot(&rebuilt, 2)
            .await
            .expect("rebuilt authoritative device snapshot")
            .own_verification,
        crate::app::devices::NativeOwnDeviceVerification::Verified
    );
    eprintln!("synara_own_device_proof checkpoint=rebuilt_store_verified");

    rebuilt_sync
        .stop()
        .await
        .expect("rebuilt initiator sync stop");
    responder_sync.stop().await.expect("responder sync stop");
    rebuilt
        .matrix_auth()
        .logout()
        .await
        .expect("dispose fresh initiator session");
    std::fs::remove_dir_all(&initiator_root).expect("remove disposable initiator store");
}

async fn ensure_live_responder_authority(client: &Client, password: &str) {
    let encryption = client.encryption();
    let user_id = client.user_id().expect("responder user id");
    let own_identity = encryption
        .request_user_identity(user_id)
        .await
        .expect("query responder own identity");
    let private_status = encryption.cross_signing_status().await;

    if own_identity.is_some() {
        assert!(
            private_status.as_ref().is_some_and(|status| status.is_complete()),
            "published cross-signing identity exists but the persisted responder lacks its private authority; refusing to replace it"
        );
    } else {
        match encryption.bootstrap_cross_signing_if_needed(None).await {
            Ok(()) => {}
            Err(error) => {
                let info = error
                    .as_uiaa_response()
                    .expect("cross-signing bootstrap did not return a UIA challenge");
                assert!(
                    info.flows.iter().any(|flow| {
                        flow.stages.iter().all(|stage| {
                            info.completed.contains(stage) || stage == &AuthType::Password
                        })
                    }),
                    "cross-signing bootstrap does not offer a supported password UIA flow"
                );
                let session = info
                    .session
                    .clone()
                    .expect("cross-signing password UIA session");
                let mut auth = Password::new(
                    UserIdentifier::Matrix(MatrixUserIdentifier::new(user_id.to_string())),
                    password.to_owned(),
                );
                auth.session = Some(session);
                encryption
                    .bootstrap_cross_signing(Some(AuthData::Password(auth)))
                    .await
                    .expect("password-authorized cross-signing bootstrap");
            }
        }
    }

    wait_for_live_verification_state(client, VerificationState::Verified).await;
    let status = encryption
        .cross_signing_status()
        .await
        .expect("responder cross-signing private status");
    assert!(
        status.is_complete(),
        "responder private authority is incomplete"
    );
    let identity = encryption
        .request_user_identity(user_id)
        .await
        .expect("refresh responder own identity")
        .expect("responder own identity after bootstrap");
    assert!(
        identity.is_verified(),
        "responder own identity is not verified"
    );
}

async fn wait_for_fresh_initiator_authority(client: &Client) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let user_id = client.user_id().expect("fresh initiator user id");
    loop {
        let _ = client.encryption().request_user_identity(user_id).await;
        let state = client.encryption().verification_state().get();
        assert_ne!(
            state,
            VerificationState::Verified,
            "fresh initiator was already verified before the product action"
        );
        if state == VerificationState::Unverified
            && matches!(
                client.encryption().has_devices_to_verify_against().await,
                Ok(true)
            )
        {
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "fresh initiator did not discover an eligible verified authority"
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

async fn wait_for_live_verification_state(client: &Client, expected: VerificationState) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let user_id = client.user_id().expect("signed-in user");
    loop {
        let _ = client.encryption().request_user_identity(user_id).await;
        if client.encryption().verification_state().get() == expected {
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "authoritative current-device verification state did not converge"
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

fn live_authority_root(homeserver: &str, username: &str) -> PathBuf {
    let base = std::env::var_os("SYNARA_LIVE_VERIFICATION_STORE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/live-own-device-verification")
        });
    let account_tag = live::verification_flow_tag(&format!("{homeserver}\n{username}"));
    base.join(account_tag)
}

async fn wait_for_live_device(client: &Client, device_id: &str) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    let user_id = client.user_id().expect("signed-in user");
    let device_id = matrix_sdk::ruma::OwnedDeviceId::from(device_id);
    loop {
        if client
            .encryption()
            .get_device(user_id, &device_id)
            .await
            .expect("device query")
            .is_some()
        {
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "peer device did not become visible"
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

async fn live_proof_client(
    homeserver: &str,
    store_path: PathBuf,
    store_passphrase: &str,
) -> Client {
    assert!(
        is_nonblank_proof_store_passphrase(store_passphrase),
        "SYNARA_LIVE_VERIFICATION_STORE_PASSPHRASE must not be empty or whitespace"
    );
    assert!(
        !is_legacy_unencrypted_proof_store(&store_path),
        "legacy unencrypted verification proof store detected; refusing to open or modify it. Preserve the store and use the documented authority-recovery/rotation procedure before opting into encrypted proof storage"
    );
    create_private_proof_directory(&store_path);
    // Declare the encryption route before the SDK can create any SQLite
    // files. If the process stops during `build`, the next run must retry the
    // encrypted open instead of misclassifying that partially-created store
    // as legacy plaintext. The marker is not trusted as proof that opening
    // succeeded: the SDK still opens every store with this passphrase and is
    // the authority for wrong-passphrase/plaintext failures.
    write_encrypted_proof_store_marker(&store_path);
    let client = Client::builder()
        .homeserver_url(homeserver)
        .sqlite_store(&store_path, Some(store_passphrase))
        .build()
        .await
        .expect("build encrypted live proof client");
    client
}

const ENCRYPTED_PROOF_STORE_MARKER: &str = ".synara-encrypted-proof-store-v1";
const MATRIX_SQLITE_STORE_FILES: [&str; 4] = [
    "matrix-sdk-state.sqlite3",
    "matrix-sdk-crypto.sqlite3",
    "matrix-sdk-event-cache.sqlite3",
    "matrix-sdk-media.sqlite3",
];

fn is_nonblank_proof_store_passphrase(passphrase: &str) -> bool {
    passphrase
        .chars()
        .any(|character| !character.is_whitespace())
}

fn is_legacy_unencrypted_proof_store(path: &std::path::Path) -> bool {
    is_legacy_unencrypted_proof_store_state(
        path.join(ENCRYPTED_PROOF_STORE_MARKER).is_file(),
        MATRIX_SQLITE_STORE_FILES
            .iter()
            .any(|filename| path.join(filename).exists()),
    )
}

fn is_legacy_unencrypted_proof_store_state(
    encrypted_marker_exists: bool,
    sqlite_store_exists: bool,
) -> bool {
    sqlite_store_exists && !encrypted_marker_exists
}

fn write_encrypted_proof_store_marker(path: &std::path::Path) {
    let marker = path.join(ENCRYPTED_PROOF_STORE_MARKER);
    std::fs::write(&marker, b"encrypted-proof-store-v1\n")
        .expect("write encrypted proof-store marker");
    set_private_file_permissions(&marker);
}

#[test]
fn legacy_verification_store_detection_is_non_destructive_and_fail_closed() {
    assert!(is_legacy_unencrypted_proof_store_state(false, true));
    assert!(!is_legacy_unencrypted_proof_store_state(true, true));
    assert!(!is_legacy_unencrypted_proof_store_state(false, false));
}

#[test]
fn verification_store_passphrase_rejects_blank_values() {
    assert!(!is_nonblank_proof_store_passphrase(""));
    assert!(!is_nonblank_proof_store_passphrase(" \t\n"));
    assert!(is_nonblank_proof_store_passphrase(
        "operator-supplied secret"
    ));
}

fn create_private_proof_directory(path: &std::path::Path) {
    std::fs::create_dir_all(path).expect("create private verification proof directory");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .expect("restrict verification proof directory permissions");
    }
}

fn set_private_file_permissions(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .expect("restrict verification proof metadata permissions");
    }
}

fn live_proof_root() -> PathBuf {
    let mut entropy = [0_u8; 8];
    getrandom::fill(&mut entropy).expect("proof path entropy");
    std::env::temp_dir().join(format!(
        "synara-sas-proof-{:016x}",
        u64::from_be_bytes(entropy)
    ))
}

async fn wait_for_live_phase(
    owner: &NativeVerificationOwner,
    flow_id: &str,
    expected: NativeVerificationPhase,
) -> NativeVerificationRequest {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    loop {
        if let Some(request) = owner
            .list()
            .await
            .requests
            .into_iter()
            .find(|request| request.flow_id == flow_id)
        {
            if request.phase == expected {
                return request;
            }
            assert!(
                !matches!(
                    request.phase,
                    NativeVerificationPhase::Cancelled
                        | NativeVerificationPhase::Mismatched
                        | NativeVerificationPhase::Failed
                ),
                "verification became {:?} before {expected:?}",
                request.phase
            );
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "verification did not reach {expected:?}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
