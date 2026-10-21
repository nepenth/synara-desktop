//! MatrixMockServer coverage for identity reset (password UIA, account-page
//! approval, cancel, a wrong password, cleanup on sign-out), recovery setup
//! right after it, and the post-sign-in "Verify this device" prompt.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use matrix_sdk::{
    ruma::{device_id, DeviceId},
    test_utils::mocks::MatrixMockServer,
    Client,
};
use serde_json::{json, Value};
use wiremock::{
    matchers::{method, path_regex},
    Match, Mock, Request, ResponseTemplate,
};

use super::setup_tests::{
    alice, backup_creations, keys_server, mount_recovery_homeserver, signing_uploads,
    WithoutPasswordStage, PASSWORD, SIGNING_UPLOAD, UIA_SESSION,
};
use super::{
    bootstrap_after_login, IdentityResetFlow, LoginCrossSigningOutcome, NativeIdentityResetStep,
    IDENTITY_RESET_APPROVAL_PENDING, IDENTITY_RESET_CANCELLED, IDENTITY_RESET_NOT_PENDING,
    IDENTITY_RESET_PASSWORD_REJECTED,
};
use crate::app::devices::NativeDeviceOwner;
use crate::core::Core;
use crate::core_api::MatrixSecretStorageActionResponse;
use crate::dto::{SessionLifecycle, SessionSnapshot};

const APPROVAL_URL: &str = "https://account.example.org/reset-cross-signing";

/// Synapse without MSC3967: every later signing-key upload needs the
/// password, and a wrong one is refused with `M_FORBIDDEN`.
async fn require_password_for_uploads(server: &MatrixMockServer) {
    Mock::given(method("POST"))
        .and(path_regex(SIGNING_UPLOAD))
        .and(WithoutPasswordStage)
        .respond_with(|request: &Request| {
            let body: Value = serde_json::from_slice(&request.body).unwrap_or_default();
            let mut challenge = json!({
                "flows": [{"stages": ["m.login.password"]}],
                "params": {},
                "session": UIA_SESSION,
            });
            if body["auth"]["type"] == "m.login.password" {
                challenge["errcode"] = json!("M_FORBIDDEN");
                challenge["error"] = json!("Invalid password");
            }
            ResponseTemplate::new(401).set_body_json(challenge)
        })
        .with_priority(1)
        .mount(server.server())
        .await;
}

/// A MAS server: uploads wait for approval on the account page.
struct Unapproved(Arc<AtomicBool>);

impl Match for Unapproved {
    fn matches(&self, _: &Request) -> bool {
        !self.0.load(Ordering::SeqCst)
    }
}

async fn require_account_page_approval(server: &MatrixMockServer) -> Arc<AtomicBool> {
    let approved = Arc::new(AtomicBool::new(false));
    Mock::given(method("POST"))
        .and(path_regex(SIGNING_UPLOAD))
        .and(Unapproved(Arc::clone(&approved)))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "flows": [{"stages": ["m.oauth"]}],
            "params": {"m.oauth": {"url": APPROVAL_URL}},
            "session": "oauth-session",
        })))
        .with_priority(1)
        .mount(server.server())
        .await;
    approved
}

/// An account whose first device published the identity and set up
/// recovery, then was lost along with the recovery key.
async fn account_with_lost_identity(server: &MatrixMockServer) -> Value {
    mount_recovery_homeserver(server).await;
    let lost = alice(server, device_id!("LOSTDEVICE")).await;
    assert_eq!(
        bootstrap_after_login(&lost, PASSWORD).await,
        LoginCrossSigningOutcome::Created
    );
    crate::app::secret_storage::bootstrap(&lost, 1, "old recovery passphrase")
        .await
        .expect("the lost device had recovery");
    signing_uploads(server).await[0]["master_key"].clone()
}

/// A signed-in Core for `device`, as the shells wire it after sign-in.
async fn signed_in_core(client: &Client, device: &DeviceId, generation: u64) -> Core {
    let owner = Arc::new(
        NativeDeviceOwner::start(client, Arc::new(|_| {}), generation)
            .await
            .unwrap(),
    );
    let core = Core::new(Arc::new(crate::platform::IosFailClosedPlatform::new()));
    core.open(SessionSnapshot {
        session_generation: generation,
        user_id: "@alice:example.org".to_owned(),
        device_id: device.to_string(),
        homeserver_url: "https://example.org".to_owned(),
        display_name: None,
        avatar_url: None,
        lifecycle: SessionLifecycle::Ready,
        crypto_ready: true,
    })
    .await
    .unwrap();
    core.attach_devices(owner).unwrap();
    core
}

#[tokio::test]
async fn password_reset_after_a_wrong_password_publishes_a_new_identity_then_recovery_gives_a_new_key(
) {
    let server = keys_server().await;
    require_password_for_uploads(&server).await;
    let old_master = account_with_lost_identity(&server).await;
    let creations_before = backup_creations(&server).await;

    let client = alice(&server, device_id!("NEWDEVICE")).await;
    let outcome = bootstrap_after_login(&client, PASSWORD).await;
    assert_eq!(outcome, LoginCrossSigningOutcome::AlreadyConfigured);
    let core = signed_in_core(&client, device_id!("NEWDEVICE"), 6).await;
    core.arm_device_verification_prompt(outcome).unwrap();
    assert!(core.device_verification_prompt().await.unwrap().show);

    let started = core.identity_reset_start().await.unwrap();
    assert_eq!(started.step, NativeIdentityResetStep::PasswordRequired);
    assert_eq!(started.session_generation, 6);
    assert!(started.approval_url.is_none());

    let rejected = core.identity_reset_password("wrong").await.unwrap_err();
    assert_eq!(
        rejected.diagnostic_id.as_deref(),
        Some(IDENTITY_RESET_PASSWORD_REJECTED)
    );
    let done = core.identity_reset_password(PASSWORD).await.unwrap();
    assert_eq!(done.step, NativeIdentityResetStep::Complete);

    let uploads = signing_uploads(&server).await;
    let last = uploads.last().unwrap();
    assert_eq!(last["auth"]["type"], "m.login.password");
    assert!(last["master_key"].is_object());
    assert_ne!(
        last["master_key"], old_master,
        "a new identity was published"
    );
    let private = client.encryption().cross_signing_status().await.unwrap();
    assert!(private.is_complete());
    assert!(
        !core.device_verification_prompt().await.unwrap().show,
        "the new identity has verified this device"
    );

    // Straight into recovery setup: the old secret storage reads as gone and
    // enrolment makes a new backup and shows a new key once.
    let enrolled = core
        .secret_storage_bootstrap("new recovery passphrase")
        .await
        .expect("recovery setup after the reset");
    assert!(enrolled
        .recovery_key
        .is_some_and(|key| !key.trim().is_empty()));
    assert!(enrolled.result.status.unlocked);
    assert_eq!(backup_creations(&server).await, creations_before + 1);
}

#[tokio::test]
async fn approval_reset_returns_the_account_page_and_waits_for_approval() {
    let server = keys_server().await;
    account_with_lost_identity(&server).await;
    let approved = require_account_page_approval(&server).await;
    let client = alice(&server, device_id!("NEWDEVICE")).await;
    let flow = IdentityResetFlow::with_wait(3, Duration::from_millis(1200));

    let started = flow.start(&client).await.unwrap();
    assert_eq!(started.step, NativeIdentityResetStep::ApprovalRequired);
    assert_eq!(started.approval_url.as_deref(), Some(APPROVAL_URL));
    assert_eq!(
        flow.authenticate_password(&client, PASSWORD).await,
        Err(IDENTITY_RESET_NOT_PENDING),
        "an approval reset has no password step"
    );

    // Continue before approving: not completed, and still pending.
    assert_eq!(
        flow.continue_after_approval(&client).await,
        Err(IDENTITY_RESET_APPROVAL_PENDING)
    );
    assert!(flow.is_pending());

    approved.store(true, Ordering::SeqCst);
    let done = flow.continue_after_approval(&client).await.unwrap();
    assert_eq!(done.step, NativeIdentityResetStep::Complete);
    assert!(!flow.is_pending());
    assert!(client
        .encryption()
        .cross_signing_status()
        .await
        .unwrap()
        .is_complete());
}

#[tokio::test]
async fn cancel_drops_the_reset_and_stops_a_continue_that_is_polling() {
    let server = keys_server().await;
    account_with_lost_identity(&server).await;
    require_account_page_approval(&server).await;
    let client = alice(&server, device_id!("NEWDEVICE")).await;
    let flow = Arc::new(IdentityResetFlow::with_wait(3, Duration::from_secs(20)));

    flow.start(&client).await.unwrap();
    let polling = tokio::spawn({
        let flow = Arc::clone(&flow);
        let client = client.clone();
        async move { flow.continue_after_approval(&client).await }
    });
    tokio::time::sleep(Duration::from_millis(300)).await;
    flow.cancel(&client).await;
    let stopped = tokio::time::timeout(Duration::from_secs(5), polling)
        .await
        .expect("cancel stops the polling Continue")
        .unwrap();
    assert_eq!(stopped, Err(IDENTITY_RESET_CANCELLED));
    assert!(!flow.is_pending());
    assert_eq!(
        flow.continue_after_approval(&client).await,
        Err(IDENTITY_RESET_NOT_PENDING)
    );
}

#[tokio::test]
async fn sign_out_cancels_and_drops_the_pending_reset() {
    let server = keys_server().await;
    require_password_for_uploads(&server).await;
    account_with_lost_identity(&server).await;
    let client = alice(&server, device_id!("NEWDEVICE")).await;
    let owner = Arc::new(
        NativeDeviceOwner::start(&client, Arc::new(|_| {}), 2)
            .await
            .unwrap(),
    );
    let core = Core::new(Arc::new(crate::platform::IosFailClosedPlatform::new()));
    core.attach_devices(Arc::clone(&owner)).unwrap();

    core.identity_reset_start().await.unwrap();
    assert!(owner.identity_reset_pending());
    let handle = owner.identity_reset_handle().expect("handle held");
    assert!(handle.upgrade().is_some());

    core.close().await.unwrap();
    assert!(
        !owner.identity_reset_pending(),
        "sign-out cancels the reset even while the shell still holds the owner"
    );
    assert!(handle.upgrade().is_none(), "the SDK handle is dropped");
    let no_session = core.identity_reset_password(PASSWORD).await.unwrap_err();
    assert_eq!(
        no_session.diagnostic_id.as_deref(),
        Some("p2-identity-reset-no-session")
    );
    let before = signing_uploads(&server).await.len();
    drop(owner);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(signing_uploads(&server).await.len(), before);
}

#[tokio::test]
async fn verify_prompt_shows_once_for_a_new_device_of_an_existing_identity() {
    let server = keys_server().await;
    let first = alice(&server, device_id!("FIRSTDEVICE")).await;
    let created = bootstrap_after_login(&first, PASSWORD).await;
    assert_eq!(created, LoginCrossSigningOutcome::Created);

    // Brand-new account: this device just created the identity.
    let first_core = signed_in_core(&first, device_id!("FIRSTDEVICE"), 1).await;
    first_core.arm_device_verification_prompt(created).unwrap();
    assert!(!first_core.device_verification_prompt().await.unwrap().show);

    // A new device for the existing identity.
    let second = alice(&server, device_id!("SECONDDEVICE")).await;
    let found = bootstrap_after_login(&second, PASSWORD).await;
    assert_eq!(found, LoginCrossSigningOutcome::AlreadyConfigured);
    let second_core = signed_in_core(&second, device_id!("SECONDDEVICE"), 2).await;
    assert!(
        !second_core.device_verification_prompt().await.unwrap().show,
        "a restored session is never armed"
    );
    second_core.arm_device_verification_prompt(found).unwrap();
    let prompt = second_core.device_verification_prompt().await.unwrap();
    assert!(prompt.show);
    assert_eq!(prompt.session_generation, 2);
    // Reading it again does not consume it; only "Later" does.
    assert!(second_core.device_verification_prompt().await.unwrap().show);

    let dismissed = second_core.dismiss_device_verification_prompt().unwrap();
    assert!(!dismissed.show);
    assert!(!second_core.device_verification_prompt().await.unwrap().show);

    // Sign-out clears it: the next session starts unarmed.
    second_core.close().await.unwrap();
    assert!(second_core.device_verification_prompt().await.is_err());
}

/// Settings' reset on a device that holds the account's identity, cancelled
/// at the password step. The published identity is unchanged, but the SDK
/// already removed key backup and recovery and replaced this device's private
/// keys, so recovery setup is offered yet must wait for verification; it
/// never stores the unpublished keys.
#[tokio::test]
async fn cancelling_after_start_points_back_to_recovery_setup_without_unpublished_keys() {
    let server = keys_server().await;
    require_password_for_uploads(&server).await;
    mount_recovery_homeserver(&server).await;
    let client = alice(&server, device_id!("SETTINGSDEVICE")).await;
    assert_eq!(
        bootstrap_after_login(&client, PASSWORD).await,
        LoginCrossSigningOutcome::Created
    );
    crate::app::secret_storage::bootstrap(&client, 1, "old recovery passphrase")
        .await
        .expect("recovery set up");
    let published_master = signing_uploads(&server).await[0]["master_key"].clone();
    let core = signed_in_core(&client, device_id!("SETTINGSDEVICE"), 5).await;
    assert_eq!(
        core.secret_storage_status().await.unwrap().action,
        MatrixSecretStorageActionResponse::None
    );

    let started = core.identity_reset_start().await.unwrap();
    assert_eq!(started.step, NativeIdentityResetStep::PasswordRequired);
    core.identity_reset_cancel().await.unwrap();

    let after = core.secret_storage_status().await.unwrap();
    assert_eq!(
        after.action,
        MatrixSecretStorageActionResponse::BootstrapRequired,
        "recovery is off after the cancelled reset and Set Up Recovery is offered"
    );
    assert!(
        !after.bootstrap_ready,
        "the unpublished keys were dropped, so setup waits for verification"
    );
    assert_eq!(
        core.secret_storage_bootstrap("replacement passphrase")
            .await
            .err()
            .and_then(|error| error.diagnostic_id),
        Some("v-crypto.4-bootstrap-cross-signing-required".to_owned())
    );
    // The homeserver still has the old identity.
    let user = matrix_sdk::ruma::user_id!("@alice:example.org");
    let identity = client
        .encryption()
        .request_user_identity(user)
        .await
        .unwrap()
        .expect("published identity");
    let published_key = published_master["keys"]
        .as_object()
        .and_then(|keys| keys.values().next())
        .and_then(Value::as_str)
        .unwrap()
        .to_owned();
    assert_eq!(
        identity.master_key().get_first_key().unwrap().to_base64(),
        published_key
    );
}
