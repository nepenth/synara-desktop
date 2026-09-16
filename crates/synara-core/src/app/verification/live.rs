//! Live Matrix SDK device-verification ownership for the native product path.
//!
//! The owner retains SDK request/SAS handles in Rust. IPC projections contain
//! only public identifiers, lifecycle state, and the short authentication
//! strings the user must compare. No keys, MACs, tokens, recovery material, or
//! raw SDK errors cross this boundary.

use std::{
    collections::HashMap,
    fmt::Write as _,
    sync::{Arc, Mutex as StdMutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use futures_util::StreamExt;
use matrix_sdk::{
    encryption::verification::{
        QrVerification, QrVerificationState, SasState, SasVerification, Verification,
        VerificationRequest, VerificationRequestState,
    },
    event_handler::EventHandlerHandle,
    ruma::{
        events::{
            key::verification::{request::ToDeviceKeyVerificationRequestEvent, VerificationMethod},
            AnyToDeviceEvent,
        },
        OwnedDeviceId, OwnedUserId,
    },
    Client,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::{sync::Mutex, task::JoinHandle};

use super::{
    capped_qr_image_data_url, compare_for_inbox, NativeVerificationDirection,
    NativeVerificationEmoji, NativeVerificationInbox, NativeVerificationPhase,
    NativeVerificationQr, NativeVerificationRequest, NativeVerificationSas,
};

/// Privacy-safe verification wake-up. No user ids, tokens, or SAS secrets.
/// Desktop maps this to `VERIFICATION_UPDATED_EVENT`; iOS re-fetches via list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeVerificationUpdateSignal {
    pub session_generation: u64,
}

pub type VerificationUpdateEmit = Arc<dyn Fn(NativeVerificationUpdateSignal) + Send + Sync>;

#[derive(Clone)]
struct ManagedVerification {
    request: VerificationRequest,
    other_user_id: OwnedUserId,
    other_device_id: Option<OwnedDeviceId>,
    direction: NativeVerificationDirection,
    started_ts: Option<u64>,
    sas: Option<SasVerification>,
    qr: Option<QrVerification>,
    qr_image_data_url: Option<String>,
    user_confirmed: bool,
    user_mismatched: bool,
    owner_failed: bool,
}

struct VerificationRegistry {
    session_generation: u64,
    requests: HashMap<String, ManagedVerification>,
}

struct VerificationRegistrationTasks {
    active: bool,
    handles: Vec<JoinHandle<()>>,
}

pub struct NativeVerificationOwner {
    client: Client,
    registry: Arc<Mutex<VerificationRegistry>>,
    registrations: Arc<StdMutex<VerificationRegistrationTasks>>,
    watches: Arc<StdMutex<HashMap<String, JoinHandle<()>>>>,
    emit: VerificationUpdateEmit,
    session_generation: u64,
    /// Desktop can render the show-QR SVG. SharedCore / iOS cannot, so that
    /// host advertises SAS only and never calls `generate_qr_code()`.
    show_qr: bool,
    _request_handler: EventHandlerHandle,
    _wake_handler: EventHandlerHandle,
}

impl NativeVerificationOwner {
    pub fn new(client: &Client, session_generation: u64) -> Self {
        Self::with_emit(client, Arc::new(|_| {}), session_generation)
    }

    pub fn with_emit(
        client: &Client,
        emit: VerificationUpdateEmit,
        session_generation: u64,
    ) -> Self {
        Self::with_show_qr(client, emit, session_generation, true)
    }

    /// `show_qr` must be true only when the host can render `VerificationQrDto`.
    /// iOS SharedCore passes false and stays SAS-only.
    pub fn with_show_qr(
        client: &Client,
        emit: VerificationUpdateEmit,
        session_generation: u64,
        show_qr: bool,
    ) -> Self {
        let registry = Arc::new(Mutex::new(VerificationRegistry {
            session_generation,
            requests: HashMap::new(),
        }));
        let registrations = Arc::new(StdMutex::new(VerificationRegistrationTasks {
            active: true,
            handles: Vec::new(),
        }));
        let watches = Arc::new(StdMutex::new(HashMap::new()));
        let handler_registry = registry.clone();
        let handler_registrations = Arc::clone(&registrations);
        let handler_watches = Arc::clone(&watches);
        let handler_emit = Arc::clone(&emit);
        let handler_show_qr = show_qr;
        let request_handler = client.add_event_handler(
            move |event: ToDeviceKeyVerificationRequestEvent, client: Client| {
                let registry = handler_registry.clone();
                let registrations = Arc::clone(&handler_registrations);
                let watches = Arc::clone(&handler_watches);
                let emit = Arc::clone(&handler_emit);
                let show_qr = handler_show_qr;
                async move {
                    let flow_id = event.content.transaction_id.to_string();
                    verification_trace(&flow_id, "incoming_event", None, None);

                    // Event handlers run inside the client's sync dispatch. Do not
                    // poll the SDK verification cache from this callback: on some
                    // sync paths the cache publishes the request only after event
                    // dispatch yields, making every in-handler lookup miss. Keep
                    // the SDK as the sole request owner, but observe it from a
                    // detached task that can keep polling after this handler
                    // yields. The lifecycle gate makes that task owner-scoped:
                    // it cannot arm a watcher after the owner begins teardown.
                    let task_registrations = Arc::clone(&registrations);
                    let handle = tokio::spawn(async move {
                        match register_incoming_request(&registry, &client, event).await {
                            IncomingRegistration::Registered(request) => {
                                let flow_id = request.flow_id().to_owned();
                                let Ok(lifecycle) = task_registrations.lock() else {
                                    return;
                                };
                                if !lifecycle.active {
                                    return;
                                }
                                arm_watch(
                                    watches.as_ref(),
                                    request,
                                    flow_id.clone(),
                                    registry,
                                    Arc::clone(&emit),
                                    session_generation,
                                    show_qr,
                                );
                                verification_trace(&flow_id, "incoming_registered", None, None);
                                emit(NativeVerificationUpdateSignal { session_generation });
                            }
                            IncomingRegistration::AlreadyRegistered => verification_trace(
                                &flow_id,
                                "incoming_already_registered",
                                None,
                                None,
                            ),
                            IncomingRegistration::SdkRequestUnavailable => verification_trace(
                                &flow_id,
                                "incoming_sdk_request_unavailable",
                                None,
                                None,
                            ),
                            IncomingRegistration::Rejected => verification_trace(
                                &flow_id,
                                "incoming_request_rejected",
                                None,
                                None,
                            ),
                        }
                    });
                    if let Ok(mut lifecycle) = registrations.lock() {
                        lifecycle.handles.retain(|handle| !handle.is_finished());
                        if lifecycle.active {
                            lifecycle.handles.push(handle);
                        } else {
                            handle.abort();
                        }
                    } else {
                        handle.abort();
                    }
                }
            },
        );
        let wake_emit = Arc::clone(&emit);
        let wake_handler =
            client.add_event_handler(move |event: AnyToDeviceEvent, _client: Client| {
                let emit = Arc::clone(&wake_emit);
                async move {
                    if is_verification_to_device(&event) {
                        emit(NativeVerificationUpdateSignal { session_generation });
                    }
                }
            });
        Self {
            client: client.clone(),
            registry,
            registrations,
            watches,
            emit,
            session_generation,
            show_qr,
            _request_handler: request_handler,
            _wake_handler: wake_handler,
        }
    }

    fn signal(&self) {
        (self.emit)(NativeVerificationUpdateSignal {
            session_generation: self.session_generation,
        });
    }

    pub async fn list(&self) -> NativeVerificationInbox {
        let mut registry = self.registry.lock().await;
        let session_generation = registry.session_generation;
        let mut requests: Vec<_> = registry
            .requests
            .values_mut()
            .map(project_request)
            .collect();
        requests.sort_by(compare_for_inbox);
        NativeVerificationInbox {
            session_generation,
            requests,
        }
    }

    pub async fn start(
        &self,
        device_id: Option<String>,
    ) -> Result<NativeVerificationRequest, &'static str> {
        let user_id = self
            .client
            .user_id()
            .ok_or("v-crypto.1-start-requires-session")?;
        // Remote peers reject a request from a session whose public device
        // keys are missing, even though `/devices` lists that session. Restore
        // retained-store publication before sending any verification event.
        super::publication::ensure_own_device_keys_published(&self.client).await?;
        let (request, other_device_id) = match device_id {
            Some(device_id) => {
                let device_id = OwnedDeviceId::from(device_id);
                let device = self
                    .client
                    .encryption()
                    .get_device(user_id, &device_id)
                    .await
                    .map_err(|_| "v-crypto.1-device-query-failed")?
                    .ok_or("v-crypto.1-device-not-found")?;
                let request = device
                    .request_verification_with_methods(self.advertised_methods())
                    .await
                    .map_err(|_| "v-crypto.1-device-request-failed")?;
                (request, Some(device_id))
            }
            None => start_self_verification(&self.client, user_id, self.advertised_methods()).await?,
        };

        let flow_id = request.flow_id().to_owned();
        verification_trace(
            &flow_id,
            "request_sent",
            Some(request_state_label(&request.state())),
            None,
        );
        let managed = ManagedVerification {
            request,
            other_user_id: user_id.to_owned(),
            other_device_id,
            direction: NativeVerificationDirection::Outgoing,
            started_ts: now_ms(),
            sas: None,
            qr: None,
            qr_image_data_url: None,
            user_confirmed: false,
            user_mismatched: false,
            owner_failed: false,
        };
        let request_for_watch = managed.request.clone();
        let mut registry = self.registry.lock().await;
        registry.requests.insert(flow_id.clone(), managed);
        let projected = project_request(
            registry
                .requests
                .get_mut(&flow_id)
                .expect("verification was inserted"),
        );
        drop(registry);
        arm_watch(
            self.watches.as_ref(),
            request_for_watch,
            flow_id.clone(),
            self.registry.clone(),
            Arc::clone(&self.emit),
            self.session_generation,
            self.show_qr,
        );
        self.signal();
        Ok(projected)
    }

    pub async fn accept(&self, flow_id: &str) -> Result<NativeVerificationRequest, &'static str> {
        let request = self.request(flow_id).await?;
        verification_trace(
            flow_id,
            "request_accepting",
            Some(request_state_label(&request.state())),
            None,
        );
        request
            .accept_with_methods(self.advertised_methods())
            .await
            .map_err(|_| "v-crypto.1-accept-failed")?;
        try_generate_show_qr(&self.registry, flow_id, self.show_qr).await;
        let snapshot = self.snapshot(flow_id).await?;
        self.signal();
        Ok(snapshot)
    }

    pub async fn begin_sas(
        &self,
        flow_id: &str,
    ) -> Result<NativeVerificationRequest, &'static str> {
        let request = self.request(flow_id).await?;
        verification_trace(
            flow_id,
            "sas_beginning",
            Some(request_state_label(&request.state())),
            None,
        );
        let sas = match request.state() {
            VerificationRequestState::Ready { .. } => request
                .start_sas()
                .await
                .map_err(|_| "v-crypto.1-sas-start-failed")?
                .ok_or("v-crypto.1-sas-start-unavailable")?,
            VerificationRequestState::Transitioned {
                verification: Verification::SasV1(sas),
            } => sas,
            VerificationRequestState::Transitioned { .. } => request
                .start_sas()
                .await
                .map_err(|_| "v-crypto.1-sas-start-failed")?
                .ok_or("v-crypto.1-sas-start-unavailable")?,
            _ => {
                return Err("v-crypto.1-sas-invalid-state");
            }
        };
        verification_trace(
            flow_id,
            "sas_started",
            Some(request_state_label(&request.state())),
            Some(sas_state_label(&sas.state())),
        );
        let mut registry = self.registry.lock().await;
        let managed = registry
            .requests
            .get_mut(flow_id)
            .ok_or("v-crypto.1-flow-not-found")?;
        managed.other_device_id = Some(sas.other_device().device_id().to_owned());
        let request_for_watch = managed.request.clone();
        managed.sas = Some(sas);
        // SAS is now the active comparison. Drop the show-QR image so the host
        // cannot display a now-invalid code next to emoji.
        managed.qr_image_data_url = None;
        let projected = project_request(managed);
        drop(registry);
        arm_watch(
            self.watches.as_ref(),
            request_for_watch,
            flow_id.to_owned(),
            self.registry.clone(),
            Arc::clone(&self.emit),
            self.session_generation,
            self.show_qr,
        );
        self.signal();
        Ok(projected)
    }

    pub async fn confirm(&self, flow_id: &str) -> Result<NativeVerificationRequest, &'static str> {
        let sas = self.sas(flow_id).await?;
        verification_trace(
            flow_id,
            "sas_confirming",
            None,
            Some(sas_state_label(&sas.state())),
        );
        if !sas.can_be_presented() {
            return Err("v-crypto.1-confirm-before-sas");
        }
        sas.confirm()
            .await
            .map_err(|_| "v-crypto.1-confirm-failed")?;
        let mut registry = self.registry.lock().await;
        let managed = registry
            .requests
            .get_mut(flow_id)
            .ok_or("v-crypto.1-flow-not-found")?;
        managed.user_confirmed = true;
        let projected = project_request(managed);
        drop(registry);
        self.signal();
        Ok(projected)
    }

    pub async fn mismatch(&self, flow_id: &str) -> Result<NativeVerificationRequest, &'static str> {
        let sas = self.sas(flow_id).await?;
        sas.mismatch()
            .await
            .map_err(|_| "v-crypto.1-mismatch-failed")?;
        let mut registry = self.registry.lock().await;
        let managed = registry
            .requests
            .get_mut(flow_id)
            .ok_or("v-crypto.1-flow-not-found")?;
        managed.user_mismatched = true;
        let projected = project_request(managed);
        drop(registry);
        self.signal();
        Ok(projected)
    }

    pub async fn cancel(&self, flow_id: &str) -> Result<NativeVerificationRequest, &'static str> {
        let (request, sas, qr) = {
            let registry = self.registry.lock().await;
            let managed = registry
                .requests
                .get(flow_id)
                .ok_or("v-crypto.1-flow-not-found")?;
            (
                managed.request.clone(),
                managed.sas.clone(),
                managed.qr.clone(),
            )
        };
        if let Some(sas) = sas {
            sas.cancel()
                .await
                .map_err(|_| "v-crypto.1-sas-cancel-failed")?;
        } else if let Some(qr) = qr {
            qr.cancel()
                .await
                .map_err(|_| "v-crypto.1-qr-cancel-failed")?;
        } else {
            request
                .cancel()
                .await
                .map_err(|_| "v-crypto.1-request-cancel-failed")?;
        }
        let snapshot = self.snapshot(flow_id).await?;
        self.signal();
        Ok(snapshot)
    }

    pub async fn dismiss(&self, flow_id: &str) -> Result<(), &'static str> {
        let mut registry = self.registry.lock().await;
        let managed = registry
            .requests
            .get_mut(flow_id)
            .ok_or("v-crypto.1-flow-not-found")?;
        let projection = project_request(managed);
        if !matches!(
            projection.phase,
            NativeVerificationPhase::Done
                | NativeVerificationPhase::Mismatched
                | NativeVerificationPhase::Cancelled
                | NativeVerificationPhase::Failed
        ) {
            return Err("v-crypto.1-dismiss-active-flow");
        }
        registry.requests.remove(flow_id);
        drop(registry);
        abort_watch(self.watches.as_ref(), flow_id);
        self.signal();
        Ok(())
    }

    async fn request(&self, flow_id: &str) -> Result<VerificationRequest, &'static str> {
        let registry = self.registry.lock().await;
        registry
            .requests
            .get(flow_id)
            .map(|managed| managed.request.clone())
            .ok_or("v-crypto.1-flow-not-found")
    }

    async fn sas(&self, flow_id: &str) -> Result<SasVerification, &'static str> {
        let mut registry = self.registry.lock().await;
        let managed = registry
            .requests
            .get_mut(flow_id)
            .ok_or("v-crypto.1-flow-not-found")?;
        refresh_sas(managed);
        managed.sas.clone().ok_or("v-crypto.1-sas-unavailable")
    }

    async fn snapshot(&self, flow_id: &str) -> Result<NativeVerificationRequest, &'static str> {
        let mut registry = self.registry.lock().await;
        registry
            .requests
            .get_mut(flow_id)
            .map(project_request)
            .ok_or("v-crypto.1-flow-not-found")
    }

    fn advertised_methods(&self) -> Vec<VerificationMethod> {
        advertised_verification_methods(self.show_qr)
    }
}

impl Drop for NativeVerificationOwner {
    fn drop(&mut self) {
        // Hold the lifecycle gate through cancellation so an event callback
        // cannot register a late observer or arm a watcher after this drain.
        retire_registration_tasks(self.registrations.as_ref());
        if let Ok(mut watches) = self.watches.lock() {
            for (_, handle) in watches.drain() {
                handle.abort();
            }
        }
    }
}

fn retire_registration_tasks(registrations: &StdMutex<VerificationRegistrationTasks>) {
    if let Ok(mut lifecycle) = registrations.lock() {
        lifecycle.active = false;
        for handle in lifecycle.handles.drain(..) {
            handle.abort();
        }
    }
}

async fn start_self_verification(
    client: &Client,
    user_id: &matrix_sdk::ruma::UserId,
    methods: Vec<VerificationMethod>,
) -> Result<(VerificationRequest, Option<OwnedDeviceId>), &'static str> {
    let encryption = client.encryption();
    // "Verify this device" is an own-identity operation. The SDK broadcasts
    // this request to the user's E2EE-capable devices and, after successful
    // SAS, updates the authoritative `Encryption::verification_state()` for
    // this device. A direct `Device::request_verification` only establishes
    // local peer trust and therefore cannot implement this route.
    // Refresh through the SDK so the identity and eligible recipient devices
    // come from the current server key set, including the publication above.
    let identity = encryption
        .request_user_identity(user_id)
        .await
        .map_err(|_| "v-crypto.1-own-identity-query-failed")?
        .ok_or("v-crypto.1-own-identity-not-found")?;
    let request = identity
        .request_verification_with_methods(methods)
        .await
        .map_err(|_| "v-crypto.1-own-request-failed")?;
    Ok((request, None))
}

enum IncomingRegistration {
    Registered(VerificationRequest),
    AlreadyRegistered,
    SdkRequestUnavailable,
    Rejected,
}

async fn register_incoming_request(
    registry: &Arc<Mutex<VerificationRegistry>>,
    client: &Client,
    event: ToDeviceKeyVerificationRequestEvent,
) -> IncomingRegistration {
    let flow_id = event.content.transaction_id.to_string();
    let mut request = None;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        request = client
            .encryption()
            .get_verification_request(&event.sender, &flow_id)
            .await;
        if request.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let Some(request) = request else {
        return IncomingRegistration::SdkRequestUnavailable;
    };
    let Some(own_user) = client.user_id() else {
        return IncomingRegistration::Rejected;
    };
    if !request.is_self_verification() && event.sender != *own_user {
        return IncomingRegistration::Rejected;
    }
    let mut registry = registry.lock().await;
    if registry.requests.contains_key(&flow_id) {
        return IncomingRegistration::AlreadyRegistered;
    }
    let cloned = request.clone();
    registry.requests.insert(
        flow_id,
        ManagedVerification {
            request,
            other_user_id: event.sender,
            other_device_id: Some(event.content.from_device),
            direction: NativeVerificationDirection::Incoming,
            started_ts: Some(event.content.timestamp.get().into()),
            sas: None,
            qr: None,
            qr_image_data_url: None,
            user_confirmed: false,
            user_mismatched: false,
            owner_failed: false,
        },
    );
    IncomingRegistration::Registered(cloned)
}

fn is_verification_to_device(event: &AnyToDeviceEvent) -> bool {
    event
        .event_type()
        .to_string()
        .starts_with("m.key.verification.")
}

fn abort_watch(watches: &StdMutex<HashMap<String, JoinHandle<()>>>, flow_id: &str) {
    if let Ok(mut watches) = watches.lock() {
        if let Some(handle) = watches.remove(flow_id) {
            handle.abort();
        }
    }
}

fn arm_watch(
    watches: &StdMutex<HashMap<String, JoinHandle<()>>>,
    request: VerificationRequest,
    flow_id: String,
    registry: Arc<Mutex<VerificationRegistry>>,
    emit: VerificationUpdateEmit,
    session_generation: u64,
    show_qr: bool,
) {
    let watch_id = flow_id.clone();
    let handle = tokio::spawn(async move {
        watch_request(request, registry, emit, session_generation, watch_id, show_qr).await;
    });
    if let Ok(mut watches) = watches.lock() {
        if let Some(previous) = watches.insert(flow_id, handle) {
            previous.abort();
        }
    }
}

async fn watch_request(
    request: VerificationRequest,
    registry: Arc<Mutex<VerificationRegistry>>,
    emit: VerificationUpdateEmit,
    session_generation: u64,
    flow_id: String,
    show_qr: bool,
) {
    let mut request_changes = request.changes();
    let mut sas_stream = None;
    let mut qr_stream = None;
    let mut initial_sas = None;
    {
        let mut registry = registry.lock().await;
        if let Some(managed) = registry.requests.get_mut(&flow_id) {
            refresh_sas(managed);
            refresh_qr(managed);
            if let Some(sas) = managed.sas.as_ref() {
                initial_sas = Some(sas.clone());
                sas_stream = Some(sas.changes());
            }
            if let Some(qr) = managed.qr.as_ref() {
                qr_stream = Some(qr.changes());
            }
        }
    }
    if let Some(sas) = initial_sas {
        if accept_transitioned_sas(&flow_id, &sas).await.is_err() {
            mark_owner_failed(&registry, &flow_id).await;
            emit(NativeVerificationUpdateSignal { session_generation });
        }
    }
    loop {
        tokio::select! {
            maybe_state = request_changes.next() => {
                let Some(state) = maybe_state else {
                    break;
                };
                verification_trace(
                    &flow_id,
                    "request_state",
                    Some(request_state_label(&state)),
                    None,
                );
                match &state {
                    VerificationRequestState::Ready { .. } => {
                        try_generate_show_qr(&registry, &flow_id, show_qr).await;
                    }
                    VerificationRequestState::Transitioned {
                        verification: Verification::SasV1(sas),
                    } => {
                        // Protocol acceptance belongs to the owner, not a UI phase.
                        // Either side may start SAS after Ready, so accept every
                        // transitioned handle direction-independently. The SDK sends
                        // m.key.verification.accept only when this handle is in the
                        // actionable Started state and is otherwise idempotent.
                        let accept_failed = accept_transitioned_sas(&flow_id, sas).await.is_err();
                        let mut registry = registry.lock().await;
                        if let Some(managed) = registry.requests.get_mut(&flow_id) {
                            managed.owner_failed |= accept_failed;
                            managed.other_device_id =
                                Some(sas.other_device().device_id().to_owned());
                            managed.sas = Some(sas.clone());
                        }
                        drop(registry);
                        sas_stream = Some(sas.changes());
                    }
                    VerificationRequestState::Transitioned {
                        verification: Verification::QrV1(qr),
                    } => {
                        retain_show_qr(&registry, &flow_id, qr).await;
                        qr_stream = Some(qr.changes());
                        if confirm_scanned_show_qr(&flow_id, qr).await.is_err() {
                            mark_owner_failed(&registry, &flow_id).await;
                        }
                    }
                    _ => {}
                }
                emit(NativeVerificationUpdateSignal { session_generation });
                if matches!(
                    state,
                    VerificationRequestState::Done | VerificationRequestState::Cancelled(_)
                ) {
                    break;
                }
            }
            maybe_sas = async {
                if let Some(stream) = sas_stream.as_mut() {
                    stream.next().await
                } else {
                    std::future::pending::<Option<SasState>>().await
                }
            } => {
                let Some(state) = maybe_sas else {
                    sas_stream = None;
                    continue;
                };
                verification_trace(
                    &flow_id,
                    "sas_state",
                    None,
                    Some(sas_state_label(&state)),
                );
                emit(NativeVerificationUpdateSignal { session_generation });
            }
            maybe_qr = async {
                if let Some(stream) = qr_stream.as_mut() {
                    stream.next().await
                } else {
                    std::future::pending::<Option<QrVerificationState>>().await
                }
            } => {
                let Some(state) = maybe_qr else {
                    qr_stream = None;
                    continue;
                };
                verification_trace(
                    &flow_id,
                    "qr_state",
                    None,
                    Some(qr_state_label(&state)),
                );
                if matches!(state, QrVerificationState::Scanned) {
                    let qr = {
                        let registry = registry.lock().await;
                        registry
                            .requests
                            .get(&flow_id)
                            .and_then(|managed| managed.qr.clone())
                    };
                    if let Some(qr) = qr {
                        if confirm_scanned_show_qr(&flow_id, &qr).await.is_err() {
                            mark_owner_failed(&registry, &flow_id).await;
                        }
                    }
                }
                emit(NativeVerificationUpdateSignal { session_generation });
            }
        }
    }
}

async fn accept_transitioned_sas(flow_id: &str, sas: &SasVerification) -> Result<(), &'static str> {
    match sas.accept().await {
        Ok(()) => {
            verification_trace(
                flow_id,
                "sas_owner_accept",
                None,
                Some(sas_state_label(&sas.state())),
            );
            Ok(())
        }
        Err(_) => {
            verification_trace(
                flow_id,
                "sas_owner_accept_failed",
                None,
                Some(sas_state_label(&sas.state())),
            );
            Err("v-crypto.1-sas-owner-accept-failed")
        }
    }
}

async fn mark_owner_failed(registry: &Arc<Mutex<VerificationRegistry>>, flow_id: &str) {
    let mut registry = registry.lock().await;
    if let Some(managed) = registry.requests.get_mut(flow_id) {
        managed.owner_failed = true;
    }
}

fn project_request(managed: &mut ManagedVerification) -> NativeVerificationRequest {
    refresh_sas(managed);
    refresh_qr(managed);
    let qr = if managed.sas.is_some() {
        None
    } else {
        project_qr(managed)
    };
    let (phase, sas) = if managed.owner_failed {
        (NativeVerificationPhase::Failed, None)
    } else if let Some(sas) = managed.sas.as_ref() {
        if managed.user_mismatched {
            (NativeVerificationPhase::Mismatched, None)
        } else if sas.is_done() {
            (NativeVerificationPhase::Done, None)
        } else if sas.is_cancelled() {
            (NativeVerificationPhase::Cancelled, None)
        } else if managed.user_confirmed || matches!(sas.state(), SasState::Confirmed) {
            (NativeVerificationPhase::Confirmed, None)
        } else if sas.can_be_presented() {
            let display = NativeVerificationSas {
                emoji: sas.emoji().map(|emoji| {
                    emoji
                        .iter()
                        .map(|item| NativeVerificationEmoji {
                            symbol: item.symbol.to_owned(),
                            description: item.description.to_owned(),
                        })
                        .collect()
                }),
                decimals: sas
                    .decimals()
                    .map(|(first, second, third)| [first, second, third]),
            };
            (NativeVerificationPhase::SasReady, Some(display))
        } else if matches!(sas.state(), SasState::Accepted { .. }) {
            (NativeVerificationPhase::KeysExchanging, None)
        } else {
            (NativeVerificationPhase::Started, None)
        }
    } else if let Some(qr) = managed.qr.as_ref() {
        if qr.is_done() {
            (NativeVerificationPhase::Done, None)
        } else if qr.is_cancelled() {
            (NativeVerificationPhase::Cancelled, None)
        } else if matches!(
            qr.state(),
            QrVerificationState::Confirmed | QrVerificationState::Scanned
        ) {
            (NativeVerificationPhase::Confirmed, None)
        } else {
            (NativeVerificationPhase::Started, None)
        }
    } else {
        match managed.request.state() {
            VerificationRequestState::Created { .. }
            | VerificationRequestState::Requested { .. } => {
                (NativeVerificationPhase::Requested, None)
            }
            VerificationRequestState::Ready { .. } => (NativeVerificationPhase::Ready, None),
            VerificationRequestState::Transitioned { .. } => {
                (NativeVerificationPhase::Started, None)
            }
            VerificationRequestState::Done => (NativeVerificationPhase::Done, None),
            VerificationRequestState::Cancelled(_) => (NativeVerificationPhase::Cancelled, None),
        }
    };
    NativeVerificationRequest {
        flow_id: managed.request.flow_id().to_owned(),
        other_user_id: managed.other_user_id.to_string(),
        other_device_id: managed.other_device_id.as_ref().map(ToString::to_string),
        direction: managed.direction,
        phase,
        started_ts: managed.started_ts,
        sas,
        qr,
    }
}

fn project_qr(managed: &ManagedVerification) -> Option<NativeVerificationQr> {
    let image_data_url = managed.qr_image_data_url.clone()?;
    Some(NativeVerificationQr {
        image_data_url,
        scanned: managed.qr.as_ref().is_some_and(|qr| qr.has_been_scanned()),
    })
}

fn refresh_sas(managed: &mut ManagedVerification) {
    if managed.sas.is_some() {
        return;
    }
    if let VerificationRequestState::Transitioned {
        verification: Verification::SasV1(sas),
    } = managed.request.state()
    {
        managed.other_device_id = Some(sas.other_device().device_id().to_owned());
        managed.sas = Some(sas);
    }
}

fn refresh_qr(managed: &mut ManagedVerification) {
    if managed.qr.is_some() {
        return;
    }
    if let VerificationRequestState::Transitioned {
        verification: Verification::QrV1(qr),
    } = managed.request.state()
    {
        managed.other_device_id = Some(qr.other_device().device_id().to_owned());
        if managed.qr_image_data_url.is_none() {
            managed.qr_image_data_url = qr_svg_data_url(&qr);
        }
        managed.qr = Some(qr);
    }
}

fn advertised_verification_methods(show_qr: bool) -> Vec<VerificationMethod> {
    if show_qr {
        vec![
            VerificationMethod::SasV1,
            VerificationMethod::QrCodeShowV1,
            VerificationMethod::ReciprocateV1,
        ]
    } else {
        vec![VerificationMethod::SasV1]
    }
}

async fn try_generate_show_qr(
    registry: &Arc<Mutex<VerificationRegistry>>,
    flow_id: &str,
    show_qr: bool,
) {
    if !show_qr {
        return;
    }
    let request = {
        let registry = registry.lock().await;
        registry
            .requests
            .get(flow_id)
            .map(|managed| managed.request.clone())
    };
    let Some(request) = request else {
        return;
    };
    let Ok(Some(qr)) = request.generate_qr_code().await else {
        return;
    };
    let image_data_url = qr_svg_data_url(&qr);
    if image_data_url.is_none() {
        // Generating transitions the request out of Ready. If the host cannot
        // render the image, start SAS immediately instead of stalling.
        verification_trace(flow_id, "qr_image_dropped_sas_fallback", None, None);
        if request.start_sas().await.is_err() {
            mark_owner_failed(registry, flow_id).await;
        }
        return;
    }
    retain_show_qr(registry, flow_id, &qr).await;
}

async fn retain_show_qr(
    registry: &Arc<Mutex<VerificationRegistry>>,
    flow_id: &str,
    qr: &QrVerification,
) {
    let image_data_url = qr_svg_data_url(qr);
    let mut registry = registry.lock().await;
    if let Some(managed) = registry.requests.get_mut(flow_id) {
        managed.other_device_id = Some(qr.other_device().device_id().to_owned());
        managed.qr = Some(qr.clone());
        if image_data_url.is_some() {
            managed.qr_image_data_url = image_data_url;
        }
    }
}

async fn confirm_scanned_show_qr(flow_id: &str, qr: &QrVerification) -> Result<(), &'static str> {
    if !qr.has_been_scanned() {
        return Ok(());
    }
    match qr.confirm().await {
        Ok(()) => {
            verification_trace(flow_id, "qr_owner_confirm", None, Some("scanned"));
            Ok(())
        }
        Err(_) => {
            verification_trace(flow_id, "qr_owner_confirm_failed", None, Some("scanned"));
            Err("v-crypto.1-qr-owner-confirm-failed")
        }
    }
}

fn qr_svg_data_url(qr: &QrVerification) -> Option<String> {
    let code = qr.to_qr_code().ok()?;
    let width = code.width();
    if width == 0 {
        return None;
    }
    let quiet = 4usize;
    let dim = width + quiet * 2;
    let mut path = String::new();
    let mut run_start = None;
    let emit_run = |x: usize, y: usize, run: usize, path: &mut String| {
        let x = x + quiet;
        let y = y + quiet;
        let _ = write!(path, "M{x},{y}h{run}v1H{x}");
    };
    for (index, module) in code.to_colors().into_iter().enumerate() {
        let x = index % width;
        let y = index / width;
        let dark = module.select(true, false);
        match (dark, run_start) {
            (true, None) => run_start = Some((x, y, 1)),
            (true, Some((start_x, start_y, run))) if y == start_y && x == start_x + run => {
                run_start = Some((start_x, start_y, run + 1));
            }
            (true, Some((start_x, start_y, run))) => {
                emit_run(start_x, start_y, run, &mut path);
                run_start = Some((x, y, 1));
            }
            (false, Some((start_x, start_y, run))) => {
                emit_run(start_x, start_y, run, &mut path);
                run_start = None;
            }
            (false, None) => {}
        }
    }
    if let Some((start_x, start_y, run)) = run_start {
        emit_run(start_x, start_y, run, &mut path);
    }
    let svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 {dim} {dim}' shape-rendering='crispEdges'><rect width='{dim}' height='{dim}' fill='white'/><path fill='black' d='{path}'/></svg>"
    );
    capped_qr_image_data_url(&format!("data:image/svg+xml;charset=utf-8,{svg}"))
}

fn now_ms() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}

fn verification_trace(
    flow_id: &str,
    event: &str,
    request_state: Option<&str>,
    sas_state: Option<&str>,
) {
    if std::env::var("SYNARA_VERIFICATION_DIAGNOSTICS").as_deref() != Ok("1") {
        return;
    }
    eprintln!(
        "synara_verification event={event} flow={} request={} sas={}",
        verification_flow_tag(flow_id),
        request_state.unwrap_or("none"),
        sas_state.unwrap_or("none"),
    );
}

pub(super) fn verification_flow_tag(flow_id: &str) -> String {
    let digest = Sha256::digest(flow_id.as_bytes());
    let mut tag = String::with_capacity(12);
    for byte in digest.iter().take(6) {
        let _ = write!(&mut tag, "{byte:02x}");
    }
    tag
}

fn request_state_label(state: &VerificationRequestState) -> &'static str {
    match state {
        VerificationRequestState::Created { .. } => "created",
        VerificationRequestState::Requested { .. } => "requested",
        VerificationRequestState::Ready { .. } => "ready",
        VerificationRequestState::Transitioned { .. } => "transitioned",
        VerificationRequestState::Done => "done",
        VerificationRequestState::Cancelled(_) => "cancelled",
    }
}

fn qr_state_label(state: &QrVerificationState) -> &'static str {
    match state {
        QrVerificationState::Started => "started",
        QrVerificationState::Scanned => "scanned",
        QrVerificationState::Confirmed => "confirmed",
        QrVerificationState::Reciprocated => "reciprocated",
        QrVerificationState::Done { .. } => "done",
        QrVerificationState::Cancelled(_) => "cancelled",
    }
}

fn sas_state_label(state: &SasState) -> &'static str {
    match state {
        SasState::Created { .. } => "created",
        SasState::Started { .. } => "started",
        SasState::Accepted { .. } => "accepted",
        SasState::KeysExchanged { .. } => "keys_exchanged",
        SasState::Confirmed => "confirmed",
        SasState::Done { .. } => "done",
        SasState::Cancelled(_) => "cancelled",
    }
}

#[cfg(test)]
mod registration_lifecycle_tests {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    use super::*;

    struct MarksDrop(Arc<AtomicBool>);

    impl Drop for MarksDrop {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn advertised_methods_are_show_qr_not_scan() {
        assert_eq!(
            advertised_verification_methods(true),
            vec![
                VerificationMethod::SasV1,
                VerificationMethod::QrCodeShowV1,
                VerificationMethod::ReciprocateV1,
            ]
        );
        assert_eq!(
            advertised_verification_methods(false),
            vec![VerificationMethod::SasV1]
        );
    }

    #[tokio::test]
    async fn retiring_registration_tasks_closes_gate_and_aborts_pending_observers() {
        let dropped = Arc::new(AtomicBool::new(false));
        let mark = MarksDrop(Arc::clone(&dropped));
        let handle = tokio::spawn(async move {
            let _mark = mark;
            std::future::pending::<()>().await;
        });
        tokio::task::yield_now().await;

        let registrations = StdMutex::new(VerificationRegistrationTasks {
            active: true,
            handles: vec![handle],
        });
        retire_registration_tasks(&registrations);
        tokio::task::yield_now().await;

        let lifecycle = registrations.lock().expect("registration lifecycle lock");
        assert!(!lifecycle.active);
        assert!(lifecycle.handles.is_empty());
        assert!(dropped.load(Ordering::SeqCst));
    }
}
