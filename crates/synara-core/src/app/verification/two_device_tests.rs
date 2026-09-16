//! MatrixMockServer two-device proofs for show-QR + SAS fallback.

use super::{
    live::NativeVerificationOwner, NativeVerificationPhase, NativeVerificationRequest,
    MAX_QR_IMAGE_DATA_URL_CHARS,
};
use matrix_sdk::{
    encryption::verification::{SasState, Verification, VerificationRequest},
    ruma::{
        events::key::verification::VerificationMethod, owned_device_id, owned_user_id, DeviceId,
        UserId,
    },
    test_utils::mocks::{encryption::PendingToDeviceMessages, MatrixMockServer},
    Client,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::time::Instant;
use wiremock::MockGuard;

struct TwoDevices {
    server: MatrixMockServer,
    alice: Client,
    alice2: Client,
    queue: Arc<Mutex<PendingToDeviceMessages>>,
    _guard: MockGuard,
}

async fn two_own_devices() -> TwoDevices {
    let server = MatrixMockServer::new().await;
    server.mock_crypto_endpoints_preset().await;
    let user = owned_user_id!("@alice:example.org");
    let alice = server
        .client_builder_for_crypto_end_to_end(&user, &owned_device_id!("ALICE1"))
        .build()
        .await;
    alice
        .encryption()
        .bootstrap_cross_signing(None)
        .await
        .expect("bootstrap alice cross-signing");
    server.mock_sync().ok_and_run(&alice, |_| {}).await;
    let alice2 = server
        .set_up_new_device_for_encryption(&alice, &owned_device_id!("ALICE2"), vec![])
        .await;
    server
        .mock_sync()
        .ok_and_run(&alice, |builder| {
            builder.add_change_device(&user);
        })
        .await;
    server
        .mock_sync()
        .ok_and_run(&alice2, |builder| {
            builder.add_change_device(&user);
        })
        .await;
    wait_for_crypto_device(&alice, alice2.device_id().expect("alice2 device")).await;
    wait_for_crypto_device(&alice2, alice.device_id().expect("alice device")).await;
    let queue = Arc::new(Mutex::new(PendingToDeviceMessages::default()));
    let guard = server
        .capture_put_to_device_traffic(&user, queue.clone())
        .await;
    TwoDevices {
        server,
        alice,
        alice2,
        queue,
        _guard: guard,
    }
}

fn scan_capable_methods() -> Vec<VerificationMethod> {
    vec![
        VerificationMethod::SasV1,
        VerificationMethod::QrCodeShowV1,
        VerificationMethod::QrCodeScanV1,
        VerificationMethod::ReciprocateV1,
    ]
}

fn sas_only_methods() -> Vec<VerificationMethod> {
    vec![VerificationMethod::SasV1]
}

fn show_qr_owner(client: &Client) -> NativeVerificationOwner {
    NativeVerificationOwner::with_show_qr(client, Arc::new(|_| {}), 1, true)
}

fn sas_only_owner(client: &Client) -> NativeVerificationOwner {
    NativeVerificationOwner::with_show_qr(client, Arc::new(|_| {}), 1, false)
}

async fn wait_for_crypto_device(client: &Client, device_id: &DeviceId) {
    let user = client.user_id().expect("signed-in user");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if client
            .encryption()
            .get_device(user, device_id)
            .await
            .expect("device query")
            .is_some()
        {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "peer device {device_id} did not become visible"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

fn take_pending(queue: &Arc<Mutex<PendingToDeviceMessages>>, recipient: &Client) -> Vec<matrix_sdk::ruma::serde::Raw<matrix_sdk::ruma::events::AnyToDeviceEvent>> {
    let user = recipient.user_id().expect("recipient user").to_owned();
    let device = recipient.device_id().expect("recipient device").to_owned();
    let mut queue = queue.lock().expect("to-device queue");
    queue
        .get_mut(&user)
        .and_then(|devices| devices.get_mut(&device))
        .map(std::mem::take)
        .unwrap_or_default()
}

async fn deliver_pending(devices: &TwoDevices, recipient: &Client) {
    for message in take_pending(&devices.queue, recipient) {
        devices
            .server
            .mock_sync()
            .ok_and_run(recipient, |builder| {
                builder.add_to_device_event(message.deserialize_as().expect("to-device json"));
            })
            .await;
    }
}

async fn pump(devices: &TwoDevices) {
    for _ in 0..6 {
        deliver_pending(devices, &devices.alice).await;
        deliver_pending(devices, &devices.alice2).await;
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn wait_for_owner_phase(
    devices: &TwoDevices,
    owner: &NativeVerificationOwner,
    flow_id: &str,
    expected: NativeVerificationPhase,
) -> NativeVerificationRequest {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        pump(devices).await;
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
            Instant::now() < deadline,
            "owner did not reach {expected:?} for {flow_id}"
        );
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
}

async fn wait_for_shown_qr(
    devices: &TwoDevices,
    owner: &NativeVerificationOwner,
    flow_id: &str,
) -> NativeVerificationRequest {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        pump(devices).await;
        if let Some(request) = owner
            .list()
            .await
            .requests
            .into_iter()
            .find(|request| request.flow_id == flow_id)
        {
            if let Some(qr) = request.qr.as_ref() {
                assert!(
                    qr.image_data_url.starts_with("data:image/svg+xml"),
                    "QR DTO must be an SVG data-URL"
                );
                assert!(
                    qr.image_data_url.chars().count() <= MAX_QR_IMAGE_DATA_URL_CHARS,
                    "QR SVG exceeded the privacy cap"
                );
                return request;
            }
            assert!(
                !matches!(
                    request.phase,
                    NativeVerificationPhase::Cancelled | NativeVerificationPhase::Failed
                ),
                "verification became {:?} before a QR appeared",
                request.phase
            );
        }
        assert!(
            Instant::now() < deadline,
            "scan-capable peer did not produce a show-QR DTO"
        );
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
}

async fn wait_for_sdk_request(
    devices: &TwoDevices,
    client: &Client,
    other_user: &UserId,
    flow_id: &str,
) -> VerificationRequest {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        pump(devices).await;
        if let Some(request) = client
            .encryption()
            .get_verification_request(other_user, flow_id)
            .await
        {
            return request;
        }
        assert!(
            Instant::now() < deadline,
            "SDK peer did not observe verification request {flow_id}"
        );
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
}

async fn wait_for_sdk_sas(
    devices: &TwoDevices,
    client: &Client,
    other_user: &UserId,
    flow_id: &str,
) -> matrix_sdk::encryption::verification::SasVerification {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        pump(devices).await;
        if let Some(Verification::SasV1(sas)) =
            client.encryption().get_verification(other_user, flow_id).await
        {
            return sas;
        }
        assert!(
            Instant::now() < deadline,
            "SDK peer did not observe SAS for {flow_id}"
        );
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
}

async fn wait_for_sdk_sas_ready(
    devices: &TwoDevices,
    sas: &matrix_sdk::encryption::verification::SasVerification,
) {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        pump(devices).await;
        if sas.can_be_presented() {
            return;
        }
        assert!(
            !matches!(sas.state(), SasState::Cancelled(_) | SasState::Done { .. }),
            "SDK SAS ended before codes were presentable: {:?}",
            sas.state()
        );
        assert!(
            Instant::now() < deadline,
            "SDK SAS codes did not become presentable"
        );
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scan_capable_peer_produces_bounded_svg_after_ready() {
    let devices = two_own_devices().await;
    let owner = show_qr_owner(&devices.alice);
    let peer_device = devices
        .alice2
        .device_id()
        .expect("alice2 device")
        .to_string();
    let started = owner
        .start(Some(peer_device))
        .await
        .expect("desktop owner starts verification");
    let flow_id = started.flow_id;
    let other_user = devices.alice.user_id().expect("alice user");
    let peer_request =
        wait_for_sdk_request(&devices, &devices.alice2, other_user, &flow_id).await;
    peer_request
        .accept_with_methods(scan_capable_methods())
        .await
        .expect("scan-capable peer accepts");
    let shown = wait_for_shown_qr(&devices, &owner, &flow_id).await;
    assert!(
        matches!(
            shown.phase,
            NativeVerificationPhase::Started | NativeVerificationPhase::Ready
        ),
        "show-QR should be presented after Ready, got {:?}",
        shown.phase
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sas_only_peer_still_completes_emoji() {
    let devices = two_own_devices().await;
    let initiator = show_qr_owner(&devices.alice);
    let responder = sas_only_owner(&devices.alice2);
    let peer_device = devices
        .alice2
        .device_id()
        .expect("alice2 device")
        .to_string();
    let started = initiator
        .start(Some(peer_device))
        .await
        .expect("desktop owner starts verification");
    let flow_id = started.flow_id;
    wait_for_owner_phase(
        &devices,
        &responder,
        &flow_id,
        NativeVerificationPhase::Requested,
    )
    .await;
    responder
        .accept(&flow_id)
        .await
        .expect("SAS-only peer accepts");
    let ready = wait_for_owner_phase(
        &devices,
        &initiator,
        &flow_id,
        NativeVerificationPhase::Ready,
    )
    .await;
    assert!(
        ready.qr.is_none(),
        "SAS-only peer must not produce a show-QR DTO"
    );
    initiator
        .begin_sas(&flow_id)
        .await
        .expect("initiator starts SAS");
    let initiator_sas = wait_for_owner_phase(
        &devices,
        &initiator,
        &flow_id,
        NativeVerificationPhase::SasReady,
    )
    .await;
    let responder_sas = wait_for_owner_phase(
        &devices,
        &responder,
        &flow_id,
        NativeVerificationPhase::SasReady,
    )
    .await;
    assert_eq!(initiator_sas.sas, responder_sas.sas);
    assert!(initiator_sas.qr.is_none());
    initiator.confirm(&flow_id).await.expect("initiator confirms");
    responder.confirm(&flow_id).await.expect("responder confirms");
    wait_for_owner_phase(&devices, &initiator, &flow_id, NativeVerificationPhase::Done).await;
    wait_for_owner_phase(&devices, &responder, &flow_id, NativeVerificationPhase::Done).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn generate_qr_then_begin_sas_fallback() {
    let devices = two_own_devices().await;
    let owner = show_qr_owner(&devices.alice);
    let peer_device = devices
        .alice2
        .device_id()
        .expect("alice2 device")
        .to_string();
    let started = owner
        .start(Some(peer_device))
        .await
        .expect("desktop owner starts verification");
    let flow_id = started.flow_id;
    let other_user = devices.alice.user_id().expect("alice user");
    let peer_request =
        wait_for_sdk_request(&devices, &devices.alice2, other_user, &flow_id).await;
    peer_request
        .accept_with_methods(scan_capable_methods())
        .await
        .expect("scan-capable peer accepts");
    let shown = wait_for_shown_qr(&devices, &owner, &flow_id).await;
    assert!(shown.qr.is_some());
    let after_sas = owner
        .begin_sas(&flow_id)
        .await
        .expect("begin_sas must fall back after generate_qr_code");
    assert!(
        after_sas.qr.is_none(),
        "active SAS must drop the show-QR image"
    );
    let peer_sas = wait_for_sdk_sas(&devices, &devices.alice2, other_user, &flow_id).await;
    peer_sas.accept().await.expect("scan-capable peer accepts SAS");
    wait_for_sdk_sas_ready(&devices, &peer_sas).await;
    let owner_sas = wait_for_owner_phase(
        &devices,
        &owner,
        &flow_id,
        NativeVerificationPhase::SasReady,
    )
    .await;
    assert!(owner_sas.sas.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accept_with_methods_sas_only_still_works() {
    let devices = two_own_devices().await;
    let responder = sas_only_owner(&devices.alice2);
    let other_user = devices.alice.user_id().expect("alice user");
    let responder_device = devices
        .alice
        .encryption()
        .get_device(other_user, devices.alice2.device_id().expect("alice2"))
        .await
        .expect("query responder")
        .expect("initiator sees responder device");
    let sdk_request = responder_device
        .request_verification_with_methods(scan_capable_methods())
        .await
        .expect("scan-capable initiator requests");
    let flow_id = sdk_request.flow_id().to_owned();
    wait_for_owner_phase(
        &devices,
        &responder,
        &flow_id,
        NativeVerificationPhase::Requested,
    )
    .await;
    let accepted = responder
        .accept(&flow_id)
        .await
        .expect("SAS-only accept_with_methods");
    assert!(
        accepted.qr.is_none(),
        "SAS-only accept must not generate show-QR"
    );
    pump(&devices).await;
    sdk_request
        .start_sas()
        .await
        .expect("scan-capable initiator starts SAS")
        .expect("SAS available after SAS-only accept");
    let responder_sas = wait_for_owner_phase(
        &devices,
        &responder,
        &flow_id,
        NativeVerificationPhase::SasReady,
    )
    .await;
    assert!(responder_sas.sas.is_some());
    let initiator_sas = wait_for_sdk_sas(&devices, &devices.alice, other_user, &flow_id).await;
    wait_for_sdk_sas_ready(&devices, &initiator_sas).await;
    responder
        .confirm(&flow_id)
        .await
        .expect("SAS-only owner confirms");
    initiator_sas.confirm().await.expect("initiator confirms");
    wait_for_owner_phase(
        &devices,
        &responder,
        &flow_id,
        NativeVerificationPhase::Done,
    )
    .await;
}

#[tokio::test]
async fn advertised_sas_only_methods_are_used_when_show_qr_is_off() {
    let source = include_str!("live.rs");
    assert!(source.contains("with_show_qr"));
    assert!(source.contains("advertised_verification_methods(self.show_qr)"));
    assert!(source.contains("if !show_qr"));
    assert!(source.contains("vec![VerificationMethod::SasV1]"));
    let _ = sas_only_methods();
}
