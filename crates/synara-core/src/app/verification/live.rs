//! Live Matrix SDK device-verification ownership for the native product path.
//!
//! The owner retains SDK request/SAS handles in Rust. IPC projections contain
//! only public identifiers, lifecycle state, and the short authentication
//! strings the user must compare. No keys, MACs, tokens, recovery material, or
//! raw SDK errors cross this boundary.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex as StdMutex},
    time::{SystemTime, UNIX_EPOCH},
};

use futures_util::StreamExt;
use matrix_sdk::{
    encryption::verification::{
        SasState, SasVerification, Verification, VerificationRequest, VerificationRequestState,
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
use tokio::{sync::Mutex, task::JoinHandle};

use super::{
    phase_rank, NativeVerificationDirection, NativeVerificationEmoji, NativeVerificationInbox,
    NativeVerificationPhase, NativeVerificationRequest, NativeVerificationSas,
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
    user_confirmed: bool,
    user_mismatched: bool,
}

struct VerificationRegistry {
    session_generation: u64,
    requests: HashMap<String, ManagedVerification>,
}

pub struct NativeVerificationOwner {
    client: Client,
    registry: Arc<Mutex<VerificationRegistry>>,
    watches: Arc<StdMutex<HashMap<String, JoinHandle<()>>>>,
    emit: VerificationUpdateEmit,
    session_generation: u64,
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
        let registry = Arc::new(Mutex::new(VerificationRegistry {
            session_generation,
            requests: HashMap::new(),
        }));
        let watches = Arc::new(StdMutex::new(HashMap::new()));
        let handler_registry = registry.clone();
        let handler_watches = Arc::clone(&watches);
        let handler_emit = Arc::clone(&emit);
        let request_handler = client.add_event_handler(
            move |event: ToDeviceKeyVerificationRequestEvent, client: Client| {
                let registry = handler_registry.clone();
                let watches = Arc::clone(&handler_watches);
                let emit = Arc::clone(&handler_emit);
                async move {
                    if let Some(request) =
                        register_incoming_request(&registry, &client, event).await
                    {
                        let flow_id = request.flow_id().to_owned();
                        arm_watch(
                            watches.as_ref(),
                            request,
                            flow_id,
                            registry,
                            Arc::clone(&emit),
                            session_generation,
                        );
                        emit(NativeVerificationUpdateSignal { session_generation });
                    }
                }
            },
        );
        let wake_emit = Arc::clone(&emit);
        let wake_handler = client.add_event_handler(move |event: AnyToDeviceEvent| {
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
            watches,
            emit,
            session_generation,
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
        requests.sort_by(|left, right| {
            phase_rank(left.phase)
                .cmp(&phase_rank(right.phase))
                .then_with(|| left.started_ts.cmp(&right.started_ts))
                .then_with(|| left.flow_id.cmp(&right.flow_id))
        });
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
                    .request_verification_with_methods(vec![VerificationMethod::SasV1])
                    .await
                    .map_err(|_| "v-crypto.1-device-request-failed")?;
                (request, Some(device_id))
            }
            None => {
                let identity = crate::app::cross_signing::query_own_identity(
                    &self.client.encryption(),
                    user_id,
                )
                .await
                .map_err(|_| "v-crypto.1-identity-query-failed")?
                .ok_or("v-crypto.1-own-identity-unavailable")?;
                let request = identity
                    .request_verification_with_methods(vec![VerificationMethod::SasV1])
                    .await
                    .map_err(|_| "v-crypto.1-own-request-failed")?;
                (request, None)
            }
        };

        let flow_id = request.flow_id().to_owned();
        let managed = ManagedVerification {
            request,
            other_user_id: user_id.to_owned(),
            other_device_id,
            direction: NativeVerificationDirection::Outgoing,
            started_ts: now_ms(),
            sas: None,
            user_confirmed: false,
            user_mismatched: false,
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
            flow_id,
            self.registry.clone(),
            Arc::clone(&self.emit),
            self.session_generation,
        );
        self.signal();
        Ok(projected)
    }

    pub async fn accept(&self, flow_id: &str) -> Result<NativeVerificationRequest, &'static str> {
        let request = self.request(flow_id).await?;
        request
            .accept_with_methods(vec![VerificationMethod::SasV1])
            .await
            .map_err(|_| "v-crypto.1-accept-failed")?;
        let snapshot = self.snapshot(flow_id).await?;
        self.signal();
        Ok(snapshot)
    }

    pub async fn begin_sas(
        &self,
        flow_id: &str,
    ) -> Result<NativeVerificationRequest, &'static str> {
        let request = self.request(flow_id).await?;
        let sas = match request.state() {
            VerificationRequestState::Ready { .. } => request
                .start_sas()
                .await
                .map_err(|_| "v-crypto.1-sas-start-failed")?
                .ok_or("v-crypto.1-sas-start-unavailable")?,
            VerificationRequestState::Transitioned {
                verification: Verification::SasV1(sas),
            } => {
                if !sas.we_started() {
                    sas.accept()
                        .await
                        .map_err(|_| "v-crypto.1-sas-accept-failed")?;
                }
                sas
            }
            _ => {
                return Err("v-crypto.1-sas-invalid-state");
            }
        };
        let mut registry = self.registry.lock().await;
        let managed = registry
            .requests
            .get_mut(flow_id)
            .ok_or("v-crypto.1-flow-not-found")?;
        managed.other_device_id = Some(sas.other_device().device_id().to_owned());
        let request_for_watch = managed.request.clone();
        managed.sas = Some(sas);
        let projected = project_request(managed);
        drop(registry);
        arm_watch(
            self.watches.as_ref(),
            request_for_watch,
            flow_id.to_owned(),
            self.registry.clone(),
            Arc::clone(&self.emit),
            self.session_generation,
        );
        self.signal();
        Ok(projected)
    }

    pub async fn confirm(&self, flow_id: &str) -> Result<NativeVerificationRequest, &'static str> {
        let sas = self.sas(flow_id).await?;
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
        let (request, sas) = {
            let registry = self.registry.lock().await;
            let managed = registry
                .requests
                .get(flow_id)
                .ok_or("v-crypto.1-flow-not-found")?;
            (managed.request.clone(), managed.sas.clone())
        };
        if let Some(sas) = sas {
            sas.cancel()
                .await
                .map_err(|_| "v-crypto.1-sas-cancel-failed")?;
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
}

impl Drop for NativeVerificationOwner {
    fn drop(&mut self) {
        if let Ok(mut watches) = self.watches.lock() {
            for (_, handle) in watches.drain() {
                handle.abort();
            }
        }
    }
}

async fn register_incoming_request(
    registry: &Arc<Mutex<VerificationRegistry>>,
    client: &Client,
    event: ToDeviceKeyVerificationRequestEvent,
) -> Option<VerificationRequest> {
    let flow_id = event.content.transaction_id.to_string();
    let request = client
        .encryption()
        .get_verification_request(&event.sender, &flow_id)
        .await?;
    if !request.is_self_verification() {
        return None;
    }
    let mut registry = registry.lock().await;
    if registry.requests.contains_key(&flow_id) {
        return None;
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
            user_confirmed: false,
            user_mismatched: false,
        },
    );
    Some(cloned)
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
) {
    let watch_id = flow_id.clone();
    let handle = tokio::spawn(async move {
        watch_request(request, registry, emit, session_generation, watch_id).await;
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
) {
    let mut request_changes = request.changes();
    let mut sas_stream = None;
    {
        let mut registry = registry.lock().await;
        if let Some(managed) = registry.requests.get_mut(&flow_id) {
            refresh_sas(managed);
            if let Some(sas) = managed.sas.as_ref() {
                sas_stream = Some(sas.changes());
            }
        }
    }
    loop {
        tokio::select! {
            maybe_state = request_changes.next() => {
                let Some(state) = maybe_state else {
                    break;
                };
                if let VerificationRequestState::Transitioned {
                    verification: Verification::SasV1(sas),
                } = &state
                {
                    let mut registry = registry.lock().await;
                    if let Some(managed) = registry.requests.get_mut(&flow_id) {
                        managed.other_device_id =
                            Some(sas.other_device().device_id().to_owned());
                        managed.sas = Some(sas.clone());
                    }
                    drop(registry);
                    sas_stream = Some(sas.changes());
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
                if maybe_sas.is_none() {
                    sas_stream = None;
                    continue;
                }
                emit(NativeVerificationUpdateSignal { session_generation });
            }
        }
    }
}

fn project_request(managed: &mut ManagedVerification) -> NativeVerificationRequest {
    refresh_sas(managed);
    let (phase, sas) = if let Some(sas) = managed.sas.as_ref() {
        if managed.user_mismatched {
            (NativeVerificationPhase::Mismatched, None)
        } else if sas.is_done() {
            (NativeVerificationPhase::Done, None)
        } else if sas.is_cancelled() {
            (NativeVerificationPhase::Cancelled, None)
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
            (
                if managed.user_confirmed {
                    NativeVerificationPhase::Confirmed
                } else {
                    NativeVerificationPhase::SasReady
                },
                Some(display),
            )
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
    }
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

fn now_ms() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}
