//! Live MSC3814 dehydrated-device owner for one session.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use futures_util::StreamExt;
use matrix_sdk::{
    encryption::dehydrated_devices::DehydratedDeviceEvent, encryption::secret_storage::SecretStore,
    Client,
};
use tokio::task::JoinHandle;

use crate::app::devices::{DeviceListUpdateEmit, NativeDeviceUpdateSignal};

use super::{
    project_dehydrated_devices_status, NativeDehydratedDeviceEventKind,
    NativeDehydratedDevicesStatus, START_FAILED, START_SECRET_EMPTY,
    START_SECRET_STORE_UNAVAILABLE, START_UNSUPPORTED,
};

/// Outcome of an opportunistic MSC3814 start. Callers must ignore failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeDehydratedStartOutcome {
    Started,
    Unsupported,
    Skipped,
    Failed,
}

struct DehydratedStatusInner {
    supported: bool,
    active: bool,
    last_device_id: Option<String>,
    last_event_kind: Option<NativeDehydratedDeviceEventKind>,
}

pub struct NativeDehydratedDevicesOwner {
    client: Client,
    session_generation: u64,
    status: Arc<Mutex<DehydratedStatusInner>>,
    stopped: Arc<AtomicBool>,
    task: JoinHandle<()>,
}

impl NativeDehydratedDevicesOwner {
    /// Probe support and subscribe. Never fails attach: homeserver errors
    /// collapse to unsupported. Does not call `start()` — that needs a
    /// SecretStore still in host memory from bootstrap/unlock/restore.
    pub async fn start(
        client: &Client,
        emit: DeviceListUpdateEmit,
        session_generation: u64,
    ) -> Self {
        let dehydrated = client.encryption().dehydrated_devices();
        let supported = dehydrated.is_supported().await.unwrap_or_default();
        let status = Arc::new(Mutex::new(DehydratedStatusInner {
            supported,
            active: false,
            last_device_id: None,
            last_event_kind: None,
        }));
        let stopped = Arc::new(AtomicBool::new(false));
        let mut events = dehydrated.state_stream();
        let watch_status = Arc::clone(&status);
        let watch_stopped = Arc::clone(&stopped);
        let task = tokio::spawn(async move {
            while let Some(event) = events.next().await {
                if watch_stopped.load(Ordering::SeqCst) {
                    break;
                }
                let (kind, device_id) = match event {
                    Ok(event) => map_event(&event),
                    Err(_) => (NativeDehydratedDeviceEventKind::Lagged, None),
                };
                apply_event(&watch_status, kind, device_id);
                emit(NativeDeviceUpdateSignal { session_generation });
            }
        });
        Self {
            client: client.clone(),
            session_generation,
            status,
            stopped,
            task,
        }
    }

    pub fn status(&self) -> NativeDehydratedDevicesStatus {
        let inner = self
            .status
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        project_dehydrated_devices_status(
            self.session_generation,
            inner.supported,
            inner.active,
            inner.last_device_id.clone(),
            inner.last_event_kind,
        )
    }

    /// Start weekly rotation while `secret` is still in host memory.
    /// Failures stay silent so secret-storage / backup commands keep succeeding.
    pub async fn try_start_with_secret(&self, secret: &str) -> NativeDehydratedStartOutcome {
        let outcome = start_with_secret(&self.client, secret).await;
        if matches!(outcome, NativeDehydratedStartOutcome::Started) {
            if let Ok(mut inner) = self.status.lock() {
                inner.active = true;
                inner.supported = true;
            }
        }
        if matches!(outcome, NativeDehydratedStartOutcome::Unsupported) {
            if let Ok(mut inner) = self.status.lock() {
                inner.supported = false;
            }
        }
        outcome
    }

    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
        self.client.encryption().dehydrated_devices().stop();
        if let Ok(mut inner) = self.status.lock() {
            inner.active = false;
        }
        self.task.abort();
    }
}

impl Drop for NativeDehydratedDevicesOwner {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Open Secret Storage with the one-way recovery secret and start MSC3814.
/// Never panics; never returns SDK error text.
pub async fn start_with_secret(client: &Client, secret: &str) -> NativeDehydratedStartOutcome {
    if secret.is_empty() {
        let _ = START_SECRET_EMPTY;
        return NativeDehydratedStartOutcome::Skipped;
    }
    let dehydrated = client.encryption().dehydrated_devices();
    match dehydrated.is_supported().await {
        Ok(true) => {}
        Ok(false) => {
            let _ = START_UNSUPPORTED;
            return NativeDehydratedStartOutcome::Unsupported;
        }
        Err(_) => {
            let _ = START_UNSUPPORTED;
            return NativeDehydratedStartOutcome::Unsupported;
        }
    }
    let store = match client
        .encryption()
        .secret_storage()
        .open_secret_store(secret)
        .await
    {
        Ok(store) => store,
        Err(_) => {
            let _ = START_SECRET_STORE_UNAVAILABLE;
            return NativeDehydratedStartOutcome::Failed;
        }
    };
    start_with_store(client, &store).await
}

pub async fn start_with_store(
    client: &Client,
    store: &SecretStore,
) -> NativeDehydratedStartOutcome {
    match client.encryption().dehydrated_devices().start(store).await {
        Ok(()) => NativeDehydratedStartOutcome::Started,
        Err(_) => {
            let _ = START_FAILED;
            NativeDehydratedStartOutcome::Failed
        }
    }
}

fn apply_event(
    status: &Mutex<DehydratedStatusInner>,
    kind: NativeDehydratedDeviceEventKind,
    device_id: Option<String>,
) {
    if let Ok(mut inner) = status.lock() {
        inner.last_event_kind = Some(kind);
        if let Some(device_id) = device_id {
            inner.last_device_id = Some(device_id);
        }
        match kind {
            NativeDehydratedDeviceEventKind::Uploaded => inner.active = true,
            NativeDehydratedDeviceEventKind::Deleted => inner.active = false,
            _ => {}
        }
    }
}

fn map_event(event: &DehydratedDeviceEvent) -> (NativeDehydratedDeviceEventKind, Option<String>) {
    match event {
        DehydratedDeviceEvent::Created { device_id } => (
            NativeDehydratedDeviceEventKind::Created,
            Some(device_id.to_string()),
        ),
        DehydratedDeviceEvent::Uploaded { device_id } => (
            NativeDehydratedDeviceEventKind::Uploaded,
            Some(device_id.to_string()),
        ),
        DehydratedDeviceEvent::Deleted => (NativeDehydratedDeviceEventKind::Deleted, None),
        DehydratedDeviceEvent::KeyCached => (NativeDehydratedDeviceEventKind::KeyCached, None),
        DehydratedDeviceEvent::RehydrationStarted { device_id } => (
            NativeDehydratedDeviceEventKind::RehydrationStarted,
            Some(device_id.to_string()),
        ),
        DehydratedDeviceEvent::RehydrationProgress { .. } => {
            (NativeDehydratedDeviceEventKind::RehydrationProgress, None)
        }
        DehydratedDeviceEvent::RehydrationCompleted { device_id, .. } => (
            NativeDehydratedDeviceEventKind::RehydrationCompleted,
            Some(device_id.to_string()),
        ),
        DehydratedDeviceEvent::RehydrationError { .. } => {
            (NativeDehydratedDeviceEventKind::RehydrationError, None)
        }
        DehydratedDeviceEvent::RotationError { .. } => {
            (NativeDehydratedDeviceEventKind::RotationError, None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_sdk::test_utils::mocks::MatrixMockServer;
    use ruma::{owned_device_id, owned_user_id};
    use serde_json::json;
    use std::{
        collections::BTreeMap,
        sync::{Arc, Mutex},
        time::Duration,
    };
    use wiremock::{
        matchers::{method, path_regex},
        Mock, Request, ResponseTemplate,
    };

    async fn alice_client(server: &MatrixMockServer) -> Client {
        server.mock_crypto_endpoints_preset().await;
        server
            .client_builder_for_crypto_end_to_end(
                &owned_user_id!("@alice:example.org"),
                &owned_device_id!("4L1C3"),
            )
            .build()
            .await
    }

    async fn bootstrap_cross_signing(client: &Client) {
        client
            .encryption()
            .bootstrap_cross_signing(None)
            .await
            .unwrap();
    }

    type CapturedDevice = Arc<Mutex<Option<(ruma::OwnedDeviceId, serde_json::Value)>>>;

    async fn capture_uploaded_device(server: &MatrixMockServer) -> CapturedDevice {
        let captured: CapturedDevice = Arc::new(Mutex::new(None));
        let sink = captured.clone();
        server
            .mock_put_dehydrated_device()
            .respond_with(move |req: &Request| {
                #[derive(serde::Deserialize)]
                struct Body {
                    device_id: ruma::OwnedDeviceId,
                    device_data: serde_json::Value,
                }
                let body: Body = req.body_json().expect("valid PUT body");
                *sink.lock().unwrap() = Some((body.device_id.clone(), body.device_data));
                ResponseTemplate::new(200).set_body_json(json!({ "device_id": body.device_id }))
            })
            .mount()
            .await;
        captured
    }

    async fn mock_stateful_account_data(server: &MatrixMockServer) {
        let store: Arc<Mutex<BTreeMap<String, serde_json::Value>>> =
            Arc::new(Mutex::new(BTreeMap::new()));
        let sink = store.clone();
        Mock::given(method("PUT"))
            .and(path_regex(
                r"^/_matrix/client/.*/user/[^/]+/account_data/[^/]+$",
            ))
            .respond_with(move |req: &Request| {
                let body: serde_json::Value = req.body_json().expect("account-data PUT");
                let event_type = req
                    .url
                    .path_segments()
                    .and_then(Iterator::last)
                    .expect("event type")
                    .to_owned();
                sink.lock().unwrap().insert(event_type, body);
                ResponseTemplate::new(200).set_body_json(json!({}))
            })
            .mount(server.server())
            .await;
        Mock::given(method("GET"))
            .and(path_regex(
                r"^/_matrix/client/.*/user/[^/]+/account_data/[^/]+$",
            ))
            .respond_with(move |req: &Request| {
                let event_type = req
                    .url
                    .path_segments()
                    .and_then(Iterator::last)
                    .expect("event type")
                    .to_owned();
                match store.lock().unwrap().get(&event_type) {
                    Some(content) => ResponseTemplate::new(200).set_body_json(content),
                    None => ResponseTemplate::new(404).set_body_json(json!({
                        "errcode": "M_NOT_FOUND",
                        "error": "Account data not found",
                    })),
                }
            })
            .mount(server.server())
            .await;
    }

    fn noop_emit() -> DeviceListUpdateEmit {
        Arc::new(|_| {})
    }

    #[test]
    fn map_event_drops_sdk_error_strings() {
        let error = DehydratedDeviceEvent::RehydrationError {
            error: "pickle key ciphertext for @alice:example.org".into(),
        };
        let (kind, device_id) = map_event(&error);
        assert_eq!(kind, NativeDehydratedDeviceEventKind::RehydrationError);
        assert_eq!(device_id, None);
        let json = serde_json::to_string(&kind).unwrap();
        assert!(!json.contains("pickle"));
        assert!(!json.contains("@alice"));
    }

    #[test]
    fn drop_calls_stop_not_delete() {
        let source = include_str!("live.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production source");
        let drop_impl = production
            .split("impl Drop for NativeDehydratedDevicesOwner")
            .nth(1)
            .expect("drop impl");
        assert!(drop_impl.contains("self.stop()"));
        assert!(!drop_impl.contains(".delete()"));
        assert!(production.contains("dehydrated_devices().stop()"));
        assert!(!production.contains("dehydrated_devices().delete()"));
    }

    #[tokio::test]
    async fn unsupported_homeserver_is_silent_noop() {
        let server = MatrixMockServer::new().await;
        let client = alice_client(&server).await;
        server
            .mock_get_dehydrated_device()
            .error_unrecognized()
            .mount()
            .await;
        let owner = NativeDehydratedDevicesOwner::start(&client, noop_emit(), 3).await;
        let status = owner.status();
        assert!(!status.supported);
        assert!(!status.active);
        assert_eq!(status.session_generation, 3);
        let json = serde_json::to_string(&status).unwrap().to_ascii_lowercase();
        assert!(!json.contains("unrecognized"));
        assert!(!json.contains("errcode"));
        drop(owner);
    }

    #[tokio::test]
    async fn create_then_snapshot_projects_dehydrated_trust() {
        use crate::app::devices::{snapshot, NativeDeviceTrust};

        let server = MatrixMockServer::new().await;
        let client = alice_client(&server).await;
        bootstrap_cross_signing(&client).await;
        server.mock_put_dehydrated_device().ok_echo().mount().await;
        let pickle_key = matrix_sdk_crypto::store::types::DehydratedDeviceKey::new();
        let dehydrated_id = client
            .encryption()
            .dehydrated_devices()
            .create(None, &pickle_key)
            .await
            .unwrap();
        let current = client.device_id().expect("device").to_string();
        Mock::given(method("GET"))
            .and(path_regex(r"^/_matrix/client/.*/devices$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "devices": [
                    { "device_id": current, "display_name": "This session" },
                    {
                        "device_id": dehydrated_id.as_str(),
                        "display_name": "Dehydrated device"
                    }
                ]
            })))
            .mount(server.server())
            .await;

        let snapshot = snapshot(&client, 9).await.expect("device snapshot");
        let backup = snapshot
            .devices
            .iter()
            .find(|device| device.device_id == dehydrated_id.as_str())
            .expect("backup row");
        assert_eq!(backup.trust, NativeDeviceTrust::Dehydrated);
        assert!(!backup.is_current);
        assert!(!crate::app::devices::device_eligible_for_password_logout(
            backup
        ));
        let current_row = snapshot
            .devices
            .iter()
            .find(|device| device.is_current)
            .expect("current row");
        assert_ne!(current_row.device_id, dehydrated_id.as_str());
    }

    #[tokio::test]
    async fn start_rehydrate_then_creates_and_stop_on_drop() {
        let server = MatrixMockServer::new().await;
        let client = alice_client(&server).await;
        bootstrap_cross_signing(&client).await;
        mock_stateful_account_data(&server).await;
        let captured = capture_uploaded_device(&server).await;
        let secret_store = client
            .encryption()
            .secret_storage()
            .create_secret_store()
            .await
            .unwrap();
        let owner = NativeDehydratedDevicesOwner::start(&client, noop_emit(), 1).await;

        server
            .mock_get_dehydrated_device()
            .not_found()
            .mount()
            .await;
        assert_eq!(
            start_with_store(&client, &secret_store).await,
            NativeDehydratedStartOutcome::Started
        );
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if owner.status().last_event_kind == Some(NativeDehydratedDeviceEventKind::Uploaded)
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("first start uploads a dehydrated device");
        let first_id = captured
            .lock()
            .unwrap()
            .as_ref()
            .map(|(id, _)| id.clone())
            .expect("PUT recorded");

        let (uploaded_id, uploaded_data) = captured.lock().unwrap().clone().unwrap();
        assert_eq!(uploaded_id, first_id);
        server
            .mock_get_dehydrated_device()
            .ok(&uploaded_id, uploaded_data)
            .mount()
            .await;
        server
            .mock_dehydrated_device_events()
            .ok(vec![], None)
            .mount()
            .await;
        server
            .mock_delete_dehydrated_device()
            .ok(&uploaded_id)
            .mount()
            .await;

        assert_eq!(
            start_with_store(&client, &secret_store).await,
            NativeDehydratedStartOutcome::Started
        );
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if owner.status().last_device_id.as_deref() != Some(first_id.as_str())
                    && owner.status().last_event_kind
                        == Some(NativeDehydratedDeviceEventKind::Uploaded)
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("second start uploads a fresh dehydrated device");

        drop(owner);
        client.encryption().dehydrated_devices().stop();
    }

    #[tokio::test]
    async fn drain_truncated_does_not_delete() {
        let server = MatrixMockServer::new().await;
        let client = alice_client(&server).await;
        bootstrap_cross_signing(&client).await;
        let pickle_key = matrix_sdk_crypto::store::types::DehydratedDeviceKey::new();
        let captured = capture_uploaded_device(&server).await;
        client
            .encryption()
            .dehydrated_devices()
            .create(None, &pickle_key)
            .await
            .unwrap();
        let (uploaded_id, uploaded_data) = captured.lock().unwrap().clone().unwrap();

        server
            .mock_get_dehydrated_device()
            .ok(&uploaded_id, uploaded_data)
            .mount()
            .await;
        let batch = vec![json!({
            "type": "m.dummy",
            "sender": "@bob:example.org",
            "content": {},
        })];
        server
            .mock_dehydrated_device_events()
            .match_missing_next_batch()
            .ok(batch.clone(), Some("repeated-cursor"))
            .mount()
            .await;
        server
            .mock_dehydrated_device_events()
            .match_next_batch("repeated-cursor")
            .ok(batch, Some("repeated-cursor"))
            .mount()
            .await;
        server
            .mock_delete_dehydrated_device()
            .ok(&uploaded_id)
            .never()
            .mount()
            .await;

        let error = client
            .encryption()
            .dehydrated_devices()
            .rehydrate(&pickle_key)
            .await
            .expect_err("truncated drain");
        let message = error.to_string();
        assert!(
            message.contains("drain stopped") || message.contains("truncated"),
            "{message}"
        );
        assert!(!message.to_ascii_lowercase().contains("pickle key"));
    }

    #[tokio::test]
    async fn empty_secret_does_not_start() {
        let server = MatrixMockServer::new().await;
        let client = alice_client(&server).await;
        assert_eq!(
            start_with_secret(&client, "").await,
            NativeDehydratedStartOutcome::Skipped
        );
    }

    #[test]
    fn start_helper_never_echoes_secrets_in_source() {
        let source = include_str!("live.rs");
        let helper = source
            .split("pub async fn start_with_secret")
            .nth(1)
            .and_then(|rest| rest.split("pub async fn start_with_store").next())
            .expect("start_with_secret");
        assert!(helper.contains("open_secret_store(secret)"));
        assert!(!helper.contains("format!("));
        assert!(!helper.contains("to_string()"));
    }
}
