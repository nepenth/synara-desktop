//! Live Matrix SDK device-verification ownership for the native product path.
//!
//! The owner retains SDK request/SAS handles in Rust. IPC projections contain
//! only public identifiers, lifecycle state, and the short authentication
//! strings the user must compare. No keys, MACs, tokens, recovery material, or
//! raw SDK errors cross this boundary.

use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use matrix_sdk::{
    encryption::verification::{
        SasVerification, Verification, VerificationRequest, VerificationRequestState,
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
use tokio::sync::Mutex;

use super::{
    phase_rank, NativeVerificationDirection, NativeVerificationEmoji, NativeVerificationInbox,
    NativeVerificationPhase, NativeVerificationRequest, NativeVerificationSas,
};

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
    _request_handler: EventHandlerHandle,
}

impl NativeVerificationOwner {
    pub fn new(client: &Client, session_generation: u64) -> Self {
        let registry = Arc::new(Mutex::new(VerificationRegistry {
            session_generation,
            requests: HashMap::new(),
        }));
        let handler_registry = registry.clone();
        let request_handler = client.add_event_handler(
            move |event: ToDeviceKeyVerificationRequestEvent, client: Client| {
                let registry = handler_registry.clone();
                async move {
                    register_incoming_request(&registry, &client, event).await;
                }
            },
        );
        Self {
            client: client.clone(),
            registry,
            _request_handler: request_handler,
        }
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
                let identity = self
                    .client
                    .encryption()
                    .request_user_identity(user_id)
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
        let mut registry = self.registry.lock().await;
        registry.requests.insert(flow_id.clone(), managed);
        Ok(project_request(
            registry
                .requests
                .get_mut(&flow_id)
                .expect("verification was inserted"),
        ))
    }

    pub async fn accept(&self, flow_id: &str) -> Result<NativeVerificationRequest, &'static str> {
        let request = self.request(flow_id).await?;
        request
            .accept_with_methods(vec![VerificationMethod::SasV1])
            .await
            .map_err(|_| "v-crypto.1-accept-failed")?;
        self.snapshot(flow_id).await
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
        managed.sas = Some(sas);
        Ok(project_request(managed))
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
        Ok(project_request(managed))
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
        Ok(project_request(managed))
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
        self.snapshot(flow_id).await
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

async fn register_incoming_request(
    registry: &Arc<Mutex<VerificationRegistry>>,
    client: &Client,
    event: ToDeviceKeyVerificationRequestEvent,
) {
    let flow_id = event.content.transaction_id.to_string();
    let Some(request) = client
        .encryption()
        .get_verification_request(&event.sender, &flow_id)
        .await
    else {
        return;
    };
    if !request.is_self_verification() {
        return;
    }
    let mut registry = registry.lock().await;
    registry
        .requests
        .entry(flow_id)
        .or_insert(ManagedVerification {
            request,
            other_user_id: event.sender,
            other_device_id: Some(event.content.from_device),
            direction: NativeVerificationDirection::Incoming,
            started_ts: Some(event.content.timestamp.get().into()),
            sas: None,
            user_confirmed: false,
            user_mismatched: false,
        });
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
