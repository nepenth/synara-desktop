//! MatrixMockServer coverage for cross-signing setup, the sign-in bootstrap,
//! password UIA, refusal of unsafe prior state, recovery enrolment and the
//! iOS status reads that go through the device owner.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use matrix_sdk::{
    config::RequestConfig,
    ruma::{device_id, user_id, DeviceId},
    test_utils::mocks::MatrixMockServer,
    Client,
};
use serde_json::{json, Value};
use wiremock::{
    matchers::{method, path_regex},
    Match, Mock, Request, ResponseTemplate,
};

use super::{
    authenticate_password, bootstrap_after_login, setup, CrossSigningPasswordError,
    LoginCrossSigningOutcome, NativeCrossSigningReadiness, NativeCrossSigningSetupOutcome,
};
use crate::platform::{
    PlatformCrossSigningOwnIdentity, PlatformCrossSigningPrivateState,
    PlatformCryptoCrossSigningState,
};

const PASSWORD: &str = "test-account-password";
const UIA_SESSION: &str = "test-uia-session";
const SIGNING_UPLOAD: &str = r"^/_matrix/client/.*/keys/device_signing/upload$";

async fn alice(server: &MatrixMockServer, device: &DeviceId) -> Client {
    server
        .client_builder_for_crypto_end_to_end(user_id!("@alice:example.org"), device)
        .on_builder(|builder| builder.request_config(RequestConfig::new().disable_retry()))
        .build()
        .await
}

/// A homeserver that keeps keys (the SDK's stateful crypto preset).
async fn keys_server() -> MatrixMockServer {
    let server = MatrixMockServer::new().await;
    server.mock_crypto_endpoints_preset().await;
    server
}

/// Requests the password stage has not authorised: no `auth`, the wrong
/// password, or another UIA session.
struct WithoutPasswordStage;

impl Match for WithoutPasswordStage {
    fn matches(&self, request: &Request) -> bool {
        let body: Value = serde_json::from_slice(&request.body).unwrap_or_default();
        let auth = &body["auth"];
        auth["type"] != "m.login.password"
            || auth["password"] != PASSWORD
            || auth["session"] != UIA_SESSION
            || auth["identifier"]["user"] != "@alice:example.org"
    }
}

/// Synapse without MSC3967: every signing-key upload needs the password.
async fn require_password_stage(server: &MatrixMockServer) {
    Mock::given(method("POST"))
        .and(path_regex(SIGNING_UPLOAD))
        .and(WithoutPasswordStage)
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "flows": [{"stages": ["m.login.password"]}],
            "params": {},
            "session": UIA_SESSION,
        })))
        .with_priority(1)
        .mount(server.server())
        .await;
}

async fn signing_uploads(server: &MatrixMockServer) -> Vec<Value> {
    server
        .server()
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| request.url.path().ends_with("/keys/device_signing/upload"))
        .map(|request| serde_json::from_slice(&request.body).unwrap())
        .collect()
}

/// Every upload carried the same master key: retries re-send one identity.
fn assert_one_identity(uploads: &[Value]) {
    let first = &uploads[0]["master_key"];
    assert!(first.is_object());
    for upload in uploads {
        assert_eq!(
            &upload["master_key"], first,
            "a retry created a second identity"
        );
    }
}

#[tokio::test]
async fn sign_in_bootstrap_answers_the_password_stage_and_publishes_one_identity() {
    let server = keys_server().await;
    require_password_stage(&server).await;
    let client = alice(&server, device_id!("IOSDEVICE")).await;

    assert_eq!(
        bootstrap_after_login(&client, PASSWORD).await,
        LoginCrossSigningOutcome::Created
    );

    let uploads = signing_uploads(&server).await;
    assert_eq!(
        uploads.len(),
        2,
        "one refused attempt, one with the password"
    );
    assert!(uploads[0].get("auth").is_none());
    assert_eq!(uploads[1]["auth"]["type"], "m.login.password");
    assert_one_identity(&uploads);
    let private = client.encryption().cross_signing_status().await.unwrap();
    assert!(private.is_complete());
}

#[tokio::test]
async fn sign_in_bootstrap_needs_no_password_on_msc3967_servers() {
    let server = keys_server().await;
    let client = alice(&server, device_id!("IOSDEVICE")).await;

    assert_eq!(
        bootstrap_after_login(&client, PASSWORD).await,
        LoginCrossSigningOutcome::Created
    );
    let uploads = signing_uploads(&server).await;
    assert_eq!(uploads.len(), 1);
    assert!(uploads[0].get("auth").is_none());
}

#[tokio::test]
async fn sign_in_bootstrap_leaves_another_devices_identity_alone() {
    let server = keys_server().await;
    let first = alice(&server, device_id!("FIRSTDEVICE")).await;
    assert_eq!(
        bootstrap_after_login(&first, PASSWORD).await,
        LoginCrossSigningOutcome::Created
    );

    let second = alice(&server, device_id!("SECONDDEVICE")).await;
    assert_eq!(
        bootstrap_after_login(&second, PASSWORD).await,
        LoginCrossSigningOutcome::AlreadyConfigured
    );
    let (result, pending) = setup(&second, 4).await.unwrap();
    assert_eq!(
        result.outcome,
        NativeCrossSigningSetupOutcome::AlreadyConfigured
    );
    assert!(pending.is_none());
    assert_eq!(
        signing_uploads(&server).await.len(),
        1,
        "the second device must not enrol again"
    );
}

#[tokio::test]
async fn sign_in_bootstrap_with_an_unaccepted_password_uploads_nothing_more() {
    let server = keys_server().await;
    require_password_stage(&server).await;
    let client = alice(&server, device_id!("IOSDEVICE")).await;

    assert_eq!(
        bootstrap_after_login(&client, "not-the-password").await,
        LoginCrossSigningOutcome::Skipped("v-crypto.2-cross-signing-auth-failed")
    );
    // Security settings can still finish it: the server has no identity, so
    // setup re-sends the identity saved locally and asks for the password.
    let (result, pending) = setup(&client, 4).await.unwrap();
    assert_eq!(
        result.outcome,
        NativeCrossSigningSetupOutcome::AuthenticationRequired
    );
    assert_eq!(pending.as_deref(), Some(UIA_SESSION));
    authenticate_password(&client, 4, UIA_SESSION.to_owned(), PASSWORD)
        .await
        .unwrap();
    let uploads = signing_uploads(&server).await;
    assert_eq!(uploads.len(), 4);
    assert_one_identity(&uploads);
}

#[tokio::test]
async fn settings_setup_asks_for_the_password_and_rearms_after_a_wrong_one() {
    let server = keys_server().await;
    require_password_stage(&server).await;
    let client = alice(&server, device_id!("IOSDEVICE")).await;

    let (started, pending) = setup(&client, 7).await.unwrap();
    assert_eq!(
        started.outcome,
        NativeCrossSigningSetupOutcome::AuthenticationRequired
    );
    assert_eq!(pending.as_deref(), Some(UIA_SESSION));

    assert_eq!(
        authenticate_password(&client, 7, UIA_SESSION.to_owned(), "wrong").await,
        Err(CrossSigningPasswordError::Rejected {
            auth_session: Some(UIA_SESSION.to_owned())
        })
    );
    assert_eq!(
        authenticate_password(&client, 7, UIA_SESSION.to_owned(), "").await,
        Err(CrossSigningPasswordError::Failed(
            "v-crypto.2-cross-signing-password-empty"
        ))
    );
    let finished = authenticate_password(&client, 7, UIA_SESSION.to_owned(), PASSWORD)
        .await
        .unwrap();
    assert_eq!(finished.outcome, NativeCrossSigningSetupOutcome::Complete);
    assert_eq!(
        finished.status.readiness,
        NativeCrossSigningReadiness::Ready
    );

    let uploads = signing_uploads(&server).await;
    assert_eq!(uploads.len(), 3);
    assert_one_identity(&uploads);
    let (again, _) = setup(&client, 7).await.unwrap();
    assert_eq!(
        again.outcome,
        NativeCrossSigningSetupOutcome::AlreadyConfigured
    );
    assert_eq!(signing_uploads(&server).await.len(), 3);
}

#[tokio::test]
async fn partial_private_identity_without_a_published_one_is_refused_without_upload() {
    let server = keys_server().await;
    let first = alice(&server, device_id!("FIRSTDEVICE")).await;
    bootstrap_after_login(&first, PASSWORD).await;
    let export = first
        .olm_machine_for_testing()
        .await
        .as_ref()
        .unwrap()
        .export_cross_signing_keys()
        .await
        .unwrap()
        .unwrap();

    // The second device learns the public identity, then receives only the
    // master key (as an interrupted secret share would leave it).
    let second = alice(&server, device_id!("SECONDDEVICE")).await;
    second
        .encryption()
        .request_user_identity(user_id!("@alice:example.org"))
        .await
        .unwrap();
    let mut master_only = export;
    master_only.self_signing_key = None;
    master_only.user_signing_key = None;
    second
        .olm_machine_for_testing()
        .await
        .as_ref()
        .unwrap()
        .import_cross_signing_keys(master_only)
        .await
        .unwrap();
    let private = second.encryption().cross_signing_status().await.unwrap();
    assert!(private.has_master && !private.is_complete());

    // The homeserver now reports no identity for the account.
    Mock::given(method("POST"))
        .and(path_regex(r"^/_matrix/client/.*/keys/query$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_keys": {"@alice:example.org": {}},
            "master_keys": {},
            "self_signing_keys": {},
            "user_signing_keys": {},
        })))
        .with_priority(1)
        .mount(server.server())
        .await;
    let before = signing_uploads(&server).await.len();

    assert_eq!(
        setup(&second, 3).await.unwrap_err(),
        "v-crypto.2-cross-signing-partial-private-identity"
    );
    assert_eq!(
        bootstrap_after_login(&second, PASSWORD).await,
        LoginCrossSigningOutcome::Skipped("v-crypto.2-cross-signing-partial-private-identity")
    );
    assert_eq!(
        authenticate_password(&second, 3, UIA_SESSION.to_owned(), PASSWORD).await,
        Err(CrossSigningPasswordError::Failed(
            "v-crypto.2-cross-signing-partial-private-identity"
        ))
    );
    assert_eq!(signing_uploads(&server).await.len(), before);
}

/// Account data and key backup kept by the mock homeserver, so recovery
/// enrolment and its homeserver readback see one consistent account.
async fn mount_recovery_homeserver(server: &MatrixMockServer) {
    let account_data: Arc<Mutex<HashMap<String, Value>>> = Arc::default();
    let backup: Arc<Mutex<Option<Value>>> = Arc::default();
    let account_data_path = r"^/_matrix/client/.*/user/.*/account_data/[^/]+$";
    let event_type = |request: &Request| request.url.path().rsplit('/').next().unwrap().to_owned();

    let stored = Arc::clone(&account_data);
    Mock::given(method("PUT"))
        .and(path_regex(account_data_path))
        .respond_with(move |request: &Request| {
            stored
                .lock()
                .unwrap()
                .insert(event_type(request), request.body_json().unwrap());
            ResponseTemplate::new(200).set_body_json(json!({}))
        })
        .mount(server.server())
        .await;
    let stored = Arc::clone(&account_data);
    Mock::given(method("GET"))
        .and(path_regex(account_data_path))
        .respond_with(move |request: &Request| {
            match stored.lock().unwrap().get(&event_type(request)) {
                Some(content) => ResponseTemplate::new(200).set_body_json(content),
                None => ResponseTemplate::new(404)
                    .set_body_json(json!({"errcode": "M_NOT_FOUND", "error": "not found"})),
            }
        })
        .mount(server.server())
        .await;

    let created = Arc::clone(&backup);
    Mock::given(method("POST"))
        .and(path_regex(r"^/_matrix/client/.*/room_keys/version$"))
        .respond_with(move |request: &Request| {
            *created.lock().unwrap() = Some(request.body_json().unwrap());
            ResponseTemplate::new(200).set_body_json(json!({"version": "1"}))
        })
        .mount(server.server())
        .await;
    let created = Arc::clone(&backup);
    Mock::given(method("GET"))
        .and(path_regex(r"^/_matrix/client/.*/room_keys/version$"))
        .respond_with(move |_: &Request| match created.lock().unwrap().as_ref() {
            Some(info) => ResponseTemplate::new(200).set_body_json(json!({
                "algorithm": info["algorithm"],
                "auth_data": info["auth_data"],
                "count": 0,
                "etag": "0",
                "version": "1",
            })),
            None => ResponseTemplate::new(404)
                .set_body_json(json!({"errcode": "M_NOT_FOUND", "error": "No backup"})),
        })
        .mount(server.server())
        .await;
    Mock::given(method("PUT"))
        .and(path_regex(r"^/_matrix/client/.*/room_keys/keys$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"count": 0, "etag": "0"})))
        .mount(server.server())
        .await;
}

async fn backup_creations(server: &MatrixMockServer) -> usize {
    server
        .server()
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| {
            request.method.as_str() == "POST" && request.url.path().ends_with("/room_keys/version")
        })
        .count()
}

/// The iOS single-device path end to end: Security's setup asks for the
/// password, then recovery enrolment shows a key once; Core's crypto and
/// cross-signing reads (through the device owner, as on iOS) report it.
#[tokio::test]
async fn single_device_recovery_setup_through_the_device_owner_and_core_reads() {
    let server = keys_server().await;
    require_password_stage(&server).await;
    mount_recovery_homeserver(&server).await;
    let client = alice(&server, device_id!("IOSDEVICE")).await;
    let owner = Arc::new(
        crate::app::devices::NativeDeviceOwner::start(&client, Arc::new(|_| {}), 5)
            .await
            .unwrap(),
    );
    let core = crate::core::Core::new(Arc::new(crate::platform::IosFailClosedPlatform::new()));
    core.attach_devices(Arc::clone(&owner)).unwrap();

    // Before setup: real values, not the fail-closed platform's errors.
    let crypto = core.crypto_status().await.unwrap();
    assert!(crypto.encryption_enabled);
    assert_eq!(
        crypto.cross_signing_state,
        crate::MatrixCrossSigningState::NotSetUp
    );
    let projection = owner.cross_signing_status().await.unwrap();
    assert_eq!(
        projection.private_state(),
        PlatformCrossSigningPrivateState::Missing
    );
    assert_eq!(
        projection.own_identity(),
        PlatformCrossSigningOwnIdentity::Missing
    );
    assert_eq!(
        core.cross_signing_status().await.unwrap().readiness,
        crate::core_api::MatrixCrossSigningReadinessResponse::SetupRequired
    );
    let bootstrap_before = owner.secret_storage_bootstrap("recovery passphrase").await;
    assert_eq!(
        bootstrap_before.err(),
        Some("v-crypto.4-bootstrap-cross-signing-required")
    );

    let started = core.cross_signing_setup().await.unwrap();
    assert_eq!(
        started.outcome,
        NativeCrossSigningSetupOutcome::AuthenticationRequired
    );
    let rejected = core
        .cross_signing_setup_password("wrong")
        .await
        .unwrap_err();
    assert_eq!(
        rejected.diagnostic_id.as_deref(),
        Some("v-crypto.2-cross-signing-password-rejected")
    );
    let finished = core.cross_signing_setup_password(PASSWORD).await.unwrap();
    assert_eq!(finished.outcome, NativeCrossSigningSetupOutcome::Complete);
    assert!(owner.pending_cross_signing_auth().is_err());

    let enrolled = core
        .secret_storage_bootstrap("recovery passphrase")
        .await
        .unwrap();
    let key = enrolled.recovery_key.expect("the key is shown once");
    assert!(!key.trim().is_empty());
    assert!(enrolled.result.status.unlocked);
    assert!(enrolled.result.status.missing_secrets.is_empty());
    assert_eq!(backup_creations(&server).await, 1);

    // A second tap never enrols again and never shows a key again.
    let again = core
        .secret_storage_bootstrap("recovery passphrase")
        .await
        .unwrap();
    assert!(again.recovery_key.is_none());
    assert_eq!(backup_creations(&server).await, 1);

    let crypto = core.crypto_status().await.unwrap();
    assert_eq!(
        crypto.cross_signing_state,
        crate::MatrixCrossSigningState::Ready
    );
    assert_eq!(crypto.session_generation, 5);
    let projection = owner.cross_signing_status().await.unwrap();
    assert_eq!(
        projection.private_state(),
        PlatformCrossSigningPrivateState::Complete
    );
    assert_eq!(
        projection.own_identity(),
        PlatformCrossSigningOwnIdentity::Verified
    );
    assert_eq!(
        core.cross_signing_status().await.unwrap().readiness,
        crate::core_api::MatrixCrossSigningReadinessResponse::Ready
    );
    let crypto_projection = owner.crypto_status().await.unwrap();
    assert_eq!(
        crypto_projection.cross_signing_state(),
        PlatformCryptoCrossSigningState::Ready
    );
}
