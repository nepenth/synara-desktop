//! Unit tests for P8.3 verification inbox.

use super::*;
use crate::app::sync::{build_sync_service, SyncServiceConfig};
use crate::transport::MatrixIpcErrorCategory;
use matrix_sdk::Client;
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
fn self_verification_start_prefers_a_trusted_peer_then_identity_then_any_peer() {
    let source = include_str!("live.rs");
    assert!(source.contains("fn start_self_verification"));
    assert!(source.contains("query_own_identity"));
    assert!(source.contains("get_user_devices"));
    assert!(source.contains("is_verified_with_cross_signing"));
    assert!(source.contains("v-crypto.1-no-peer-device"));
    let helper = source
        .split("async fn start_self_verification")
        .nth(1)
        .and_then(|rest| rest.split("async fn register_incoming_request").next())
        .expect("start_self_verification helper");
    let trusted_peer = helper
        .find("is_verified_with_cross_signing")
        .expect("trusted peer first");
    let identity = helper
        .find("query_own_identity")
        .expect("identity fallback");
    let any_peer = helper
        .find("peers.first()")
        .expect("unverified peer fallback");
    assert!(
        trusted_peer < identity && identity < any_peer,
        "trusted peer, own identity, and unverified peer must stay in authority order"
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires SYNARA_LIVE_HOMESERVER, SYNARA_LIVE_USERNAME, and SYNARA_LIVE_PASSWORD"]
async fn live_two_device_sas_completes_through_product_owner_and_sync() {
    let homeserver =
        std::env::var("SYNARA_LIVE_HOMESERVER").expect("SYNARA_LIVE_HOMESERVER is required");
    let username = std::env::var("SYNARA_LIVE_USERNAME").expect("SYNARA_LIVE_USERNAME is required");
    let password = std::env::var("SYNARA_LIVE_PASSWORD").expect("SYNARA_LIVE_PASSWORD is required");
    let proof_root = live_proof_root();
    let initiator = live_proof_client(&homeserver, proof_root.join("initiator")).await;
    let responder = live_proof_client(&homeserver, proof_root.join("responder")).await;

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
    wait_for_live_device(&initiator, &responder_device).await;
    let started = initiator_owner
        .start(Some(responder_device.clone()))
        .await
        .expect("verification request");
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
    wait_for_live_phase(&responder_owner, &flow_id, NativeVerificationPhase::Started).await;
    responder_owner
        .begin_sas(&flow_id)
        .await
        .expect("responder accepts SAS");

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

async fn live_proof_client(homeserver: &str, store_path: PathBuf) -> Client {
    Client::builder()
        .homeserver_url(homeserver)
        .sqlite_store(store_path, None)
        .build()
        .await
        .expect("build live proof client")
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
                    NativeVerificationPhase::Cancelled | NativeVerificationPhase::Mismatched
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
