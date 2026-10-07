//! SyncService ownership + start/stop (P4.1 harness foundation).
//!
//! Single owner for `matrix_sdk_ui::sync_service::SyncService` per session
//! generation. Builds with optional offline mode; never dual-backend.
//!
//! **No** production Tauri commands. **No** room-list projection (P4.2).

use std::collections::HashSet;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use futures_util::StreamExt;
use matrix_sdk::Client;
use matrix_sdk_ui::sync_service::{State as SdkSyncState, SyncService};
use ruma::{OwnedRoomId, RoomId};
use tokio::sync::Mutex;

use super::capability::probe_sliding_sync;
use super::error::SyncError;
use super::readiness::{snapshot_from_sdk_state, SyncReadiness, SyncReadinessSnapshot};
use super::reconnect::{decide_reconnect, ReconnectAction, SyncIntent};

/// Configuration for building the product SyncService.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncServiceConfig {
    /// Enable native recovery for transient sync errors (bounded sync retries).
    pub offline_mode: bool,
}

impl Default for SyncServiceConfig {
    fn default() -> Self {
        Self {
            // The partial path prefers automatic recovery from transient outages.
            offline_mode: true,
        }
    }
}

/// Owned SyncService handle for one supervisor session generation.
pub struct SyncServiceOwner {
    service: Arc<SyncService>,
    session_generation: u64,
    offline_mode_enabled: bool,
    /// Best-effort server sliding-sync verdict, filled in by a background
    /// probe after install. `None` until the probe answers.
    sliding_sync_capable: Arc<std::sync::OnceLock<bool>>,
    sliding_sync_probe: tokio::task::JoinHandle<()>,
    room_subscriptions: Arc<Mutex<RoomSubscriptions>>,
    recovery_task: Option<tokio::task::JoinHandle<()>>,
    /// Re-enables send queues after recoverable send errors; aborted with
    /// this owner so it never outlives the session generation.
    send_queue_recovery: tokio::task::JoinHandle<()>,
    authentication_rejected: Arc<AtomicBool>,
    sync_requested: Arc<AtomicBool>,
    lifecycle_gate: Arc<Mutex<()>>,
}

#[derive(Default)]
struct RoomSubscriptions {
    viewport: Vec<OwnedRoomId>,
    active: Option<OwnedRoomId>,
}

impl SyncServiceOwner {
    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn offline_mode_enabled(&self) -> bool {
        self.offline_mode_enabled
    }

    pub fn service(&self) -> &Arc<SyncService> {
        &self.service
    }

    /// Observe current SDK state and project a privacy-safe readiness snapshot.
    pub fn observe(&self) -> SyncReadinessSnapshot {
        // `state()` returns a Subscriber; read the current value without async.
        let subscriber = self.service.state();
        let current: SdkSyncState = subscriber.get();
        if matches!(&current, SdkSyncState::Error(error) if is_terminal_auth_error(error.as_ref()))
        {
            self.authentication_rejected.store(true, Ordering::Release);
        }
        let mut snapshot =
            snapshot_from_sdk_state(&current, self.session_generation, self.offline_mode_enabled)
                .with_sliding_sync_capability(self.sliding_sync_capable.get().copied());
        if self.authentication_rejected.load(Ordering::Acquire) {
            snapshot.readiness = SyncReadiness::Failed;
            snapshot.failure_diagnostic_id =
                Some(super::readiness::SYNC_AUTHENTICATION_FAILURE_DIAGNOSTIC_ID);
        } else if self.offline_mode_enabled
            && snapshot.failure_diagnostic_id
                == Some(super::readiness::SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID)
        {
            snapshot.readiness = if self.sync_requested.load(Ordering::Acquire) {
                SyncReadiness::Offline
            } else {
                SyncReadiness::Idle
            };
            snapshot.failure_diagnostic_id = None;
        }
        snapshot
    }

    /// Start (or restart) underlying sliding syncs.
    pub async fn start(&self) -> Result<SyncReadinessSnapshot, SyncError> {
        let _gate = self.lifecycle_gate.lock().await;
        let snapshot = self.observe();
        if self.authentication_rejected.load(Ordering::Acquire) {
            return Ok(snapshot);
        }
        self.sync_requested.store(true, Ordering::Release);
        self.service.start().await;
        // An expired sliding-sync session (`M_UNKNOWN_POS`) clears every SDK
        // room subscription. Re-apply ours after each start so the open room
        // and visible rows do not fall back to the timeline-limit-1 feed.
        reapply_room_subscriptions(&self.service, &self.room_subscriptions).await;
        Ok(self.observe())
    }

    /// Stop underlying sliding syncs (background / logout path).
    pub async fn stop(&self) -> Result<SyncReadinessSnapshot, SyncError> {
        let _gate = self.lifecycle_gate.lock().await;
        self.sync_requested.store(false, Ordering::Release);
        let _ = self.observe();
        self.service.stop().await;
        Ok(self.observe())
    }

    /// Apply a reconnect decision for the given intent.
    pub async fn apply_intent(
        &self,
        intent: SyncIntent,
    ) -> Result<SyncReadinessSnapshot, SyncError> {
        let snap = self.observe();
        match decide_reconnect(snap.readiness, intent) {
            ReconnectAction::None => Ok(snap),
            ReconnectAction::Start => self.start().await,
            ReconnectAction::Stop => self.stop().await,
            ReconnectAction::Restart => {
                let _ = self.stop().await?;
                self.start().await
            }
        }
    }

    /// Room list service accessor for later P4.2 — not projected here.
    pub fn room_list_service(&self) -> Arc<matrix_sdk_ui::RoomListService> {
        self.service.room_list_service()
    }

    /// Keep the SDK's latest-event calculation active for the current room-list
    /// viewport. `RoomListService::set_room_subscriptions` makes the subscription
    /// set exactly `room_ids` without marking remaining rooms' members unsynced.
    /// This is coordinated with the active-room subscription below.
    pub async fn subscribe_to_room_list(&self, room_ids: &[OwnedRoomId]) {
        let mut subscriptions = self.room_subscriptions.lock().await;
        if viewport_ids_equivalent(&subscriptions.viewport, room_ids) {
            return;
        }
        subscriptions.viewport = room_ids.to_vec();
        self.apply_room_subscriptions(&subscriptions).await;
    }

    /// Promote the active room from the room-list preview to the SDK's full
    /// room subscription before its Timeline is opened without dropping the
    /// visible room-list subscriptions.
    pub async fn subscribe_to_room(&self, room_id: &RoomId) {
        let mut subscriptions = self.room_subscriptions.lock().await;
        if subscriptions.active.as_deref() == Some(room_id) {
            return;
        }
        subscriptions.active = Some(room_id.to_owned());
        self.apply_room_subscriptions(&subscriptions).await;
    }

    async fn apply_room_subscriptions(&self, subscriptions: &RoomSubscriptions) {
        let room_ids = coordinated_room_subscriptions(subscriptions);
        let room_id_refs = room_ids.iter().map(OwnedRoomId::as_ref).collect::<Vec<_>>();
        self.service
            .room_list_service()
            .set_room_subscriptions(&room_id_refs)
            .await;
    }
}

/// Re-send the cached subscription set to the SDK. Our cache skips unchanged
/// viewports, so without this a restarted session would never resubscribe.
async fn reapply_room_subscriptions(
    service: &SyncService,
    subscriptions: &Mutex<RoomSubscriptions>,
) {
    let subscriptions = subscriptions.lock().await;
    let room_ids = coordinated_room_subscriptions(&subscriptions);
    if room_ids.is_empty() {
        return;
    }
    let room_id_refs = room_ids.iter().map(OwnedRoomId::as_ref).collect::<Vec<_>>();
    service
        .room_list_service()
        .set_room_subscriptions(&room_id_refs)
        .await;
}

fn viewport_ids_equivalent(current: &[OwnedRoomId], next: &[OwnedRoomId]) -> bool {
    if current.len() != next.len() {
        return false;
    }
    if current == next {
        return true;
    }
    let current_set: HashSet<&OwnedRoomId> = current.iter().collect();
    next.iter().all(|room_id| current_set.contains(room_id))
}

fn coordinated_room_subscriptions(subscriptions: &RoomSubscriptions) -> Vec<OwnedRoomId> {
    let mut room_ids = Vec::with_capacity(subscriptions.viewport.len() + 1);
    if let Some(active) = &subscriptions.active {
        room_ids.push(active.clone());
    }
    room_ids.extend(
        subscriptions
            .viewport
            .iter()
            .filter(|room_id| Some(room_id.as_ref()) != subscriptions.active.as_deref())
            .cloned(),
    );
    room_ids
}

/// Build a SyncService for an **authenticated** client.
///
/// Refuses unauthenticated clients so we never start dual-empty sync loops.
pub async fn build_sync_service(
    client: &Client,
    session_generation: u64,
    config: SyncServiceConfig,
) -> Result<SyncServiceOwner, SyncError> {
    if client.session().is_none() {
        return Err(SyncError::NotAuthenticated {
            diagnostic_id: "p4.1-sync-requires-session",
        });
    }

    // SDK 0.19 offline mode hides the typed error and probes authenticated
    // /versions every 100 ms after a rejected refresh. Keep the typed error;
    // the native owner recovers only non-terminal failures with bounded delay.
    let service = Arc::new(
        SyncService::builder(client.clone())
            .build()
            .await
            .map_err(map_build_error)?,
    );
    let authentication_rejected = Arc::new(AtomicBool::new(false));
    let sync_requested = Arc::new(AtomicBool::new(false));
    let lifecycle_gate = Arc::new(Mutex::new(()));
    let room_subscriptions = Arc::new(Mutex::new(RoomSubscriptions::default()));
    let recovery_task = config.offline_mode.then(|| {
        spawn_network_recovery(
            service.clone(),
            authentication_rejected.clone(),
            sync_requested.clone(),
            lifecycle_gate.clone(),
            room_subscriptions.clone(),
        )
    });
    let send_queue_recovery = crate::app::send::spawn_send_queue_recovery(
        client.clone(),
        Box::pin(
            service
                .state()
                .map(|state| matches!(state, SdkSyncState::Running)),
        ),
    );
    // Best-effort server capability probe: purely informational, never gates
    // the sync path. It runs after install so restore does not wait up to
    // the probe's 3 s timeout; readiness reports `None` until it answers.
    let sliding_sync_capable = Arc::new(std::sync::OnceLock::new());
    let sliding_sync_probe = {
        let client = client.clone();
        let verdict = Arc::clone(&sliding_sync_capable);
        tokio::spawn(async move {
            if let Some(capable) = probe_sliding_sync(&client).await {
                let _ = verdict.set(capable);
            }
        })
    };

    Ok(SyncServiceOwner {
        service,
        session_generation,
        offline_mode_enabled: config.offline_mode,
        sliding_sync_capable,
        sliding_sync_probe,
        room_subscriptions,
        recovery_task,
        send_queue_recovery,
        authentication_rejected,
        sync_requested,
        lifecycle_gate,
    })
}

/// Ensure `owner` matches the live supervisor generation before use.
pub fn assert_generation(owner: &SyncServiceOwner, live_generation: u64) -> Result<(), SyncError> {
    if owner.session_generation != live_generation {
        return Err(SyncError::StaleGeneration {
            diagnostic_id: "p4.1-stale-sync-generation",
            expected: live_generation,
            observed: owner.session_generation,
        });
    }
    Ok(())
}

/// Convenience: unconfigured snapshot when no owner exists.
pub fn unconfigured_snapshot(session_generation: u64) -> SyncReadinessSnapshot {
    SyncReadinessSnapshot::unconfigured(session_generation)
}

/// Pure helper for tests / supervisor bridge: readiness label from owner or none.
pub fn readiness_of(owner: Option<&SyncServiceOwner>) -> SyncReadiness {
    match owner {
        None => SyncReadiness::Unconfigured,
        Some(o) => o.observe().readiness,
    }
}

fn map_build_error(err: matrix_sdk_ui::sync_service::Error) -> SyncError {
    // Classify from Display internally; never export raw text (may embed URLs).
    let raw = format!("{err}");
    let lower = raw.to_ascii_lowercase();
    let diagnostic_id = if lower.contains("sliding") || lower.contains("unrecognized") {
        "p4.1-sync-build-sliding-sync-unsupported"
    } else if lower.contains("encrypt") || lower.contains("crypto") {
        "p4.1-sync-build-encryption-failed"
    } else if lower.contains("room list") || lower.contains("room_list") {
        "p4.1-sync-build-room-list-failed"
    } else {
        "p4.1-sync-build-failed"
    };
    SyncError::Sdk {
        diagnostic_id,
        category: MatrixIpcErrorCategory::SdkInvariant,
    }
}

use crate::transport::MatrixIpcErrorCategory;

impl Drop for SyncServiceOwner {
    fn drop(&mut self) {
        if let Some(task) = self.recovery_task.take() {
            task.abort();
        }
        self.send_queue_recovery.abort();
        self.sliding_sync_probe.abort();
    }
}

/// Inspect typed SDK errors, never their display text. An ordinary room
/// permission denial is not a rejected session. In particular, a transient
/// refresh/network failure must preserve the session rather than force login.
pub(crate) fn is_terminal_auth_error(error: &(dyn std::error::Error + 'static)) -> bool {
    use matrix_sdk::ruma::api::error::ErrorKind;
    use matrix_sdk::{HttpError, RefreshTokenError};
    if let Some(error) = error.downcast_ref::<matrix_sdk_ui::sync_service::Error>() {
        return match error {
            matrix_sdk_ui::sync_service::Error::RoomList(inner) => is_terminal_auth_error(inner),
            matrix_sdk_ui::sync_service::Error::EncryptionSync(inner) => {
                is_terminal_auth_error(inner)
            }
            _ => false,
        };
    }
    if let Some(error) = error.downcast_ref::<matrix_sdk_ui::room_list_service::Error>() {
        return match error {
            matrix_sdk_ui::room_list_service::Error::SlidingSync(inner) => {
                is_terminal_auth_error(inner)
            }
            _ => false,
        };
    }
    if let Some(error) = error.downcast_ref::<matrix_sdk_ui::encryption_sync_service::Error>() {
        return match error {
            matrix_sdk_ui::encryption_sync_service::Error::SlidingSync(inner)
            | matrix_sdk_ui::encryption_sync_service::Error::LockError(inner)
            | matrix_sdk_ui::encryption_sync_service::Error::ClientError(inner) => {
                is_terminal_auth_error(inner)
            }
        };
    }
    if let Some(error) = error.downcast_ref::<matrix_sdk::Error>() {
        return match error {
            matrix_sdk::Error::Http(inner) => is_terminal_auth_error(inner.as_ref()),
            _ => false,
        };
    }
    if let Some(http) = error.downcast_ref::<HttpError>() {
        return match http {
            HttpError::RefreshToken(RefreshTokenError::MatrixAuth(inner)) => {
                matches!(
                    inner.client_api_error_kind(),
                    Some(
                        ErrorKind::UnknownToken(_) | ErrorKind::Forbidden | ErrorKind::MissingToken
                    )
                )
            }
            HttpError::RefreshToken(RefreshTokenError::RefreshTokenRequired) => true,
            HttpError::Cached(inner) => is_terminal_auth_error(inner.as_ref()),
            _ => matches!(
                http.client_api_error_kind(),
                Some(ErrorKind::UnknownToken(_) | ErrorKind::MissingToken)
            ),
        };
    }
    error.source().is_some_and(is_terminal_auth_error)
}

fn spawn_network_recovery(
    service: Arc<SyncService>,
    authentication_rejected: Arc<AtomicBool>,
    sync_requested: Arc<AtomicBool>,
    lifecycle_gate: Arc<Mutex<()>>,
    room_subscriptions: Arc<Mutex<RoomSubscriptions>>,
) -> tokio::task::JoinHandle<()> {
    // Subscribe before spawning so a fast first sync failure cannot be missed.
    let mut states = service.state();
    tokio::spawn(async move {
        let mut pending = Some(states.get());
        loop {
            let state = match pending.take() {
                Some(state) => state,
                None => match states.next().await {
                    Some(state) => state,
                    None => break,
                },
            };
            match state {
                SdkSyncState::Error(error) if is_terminal_auth_error(error.as_ref()) => {
                    authentication_rejected.store(true, Ordering::Release);
                }
                SdkSyncState::Error(error)
                    if sync_requested.load(Ordering::Acquire)
                        && !is_terminal_auth_error(error.as_ref()) =>
                {
                    tokio::select! {
                        next = states.next() => { pending = next; if pending.is_none() { break; } }
                        _ = tokio::time::sleep(Duration::from_secs(5)) => {
                            let _gate = lifecycle_gate.lock().await;
                            if sync_requested.load(Ordering::Acquire)
                                && !authentication_rejected.load(Ordering::Acquire)
                                && matches!(service.state().get(), SdkSyncState::Error(ref error) if !is_terminal_auth_error(error.as_ref())) {
                                service.start().await;
                                reapply_room_subscriptions(&service, &room_subscriptions).await;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    })
}

#[cfg(test)]
mod subscription_tests {
    use super::*;

    #[test]
    fn active_room_is_prioritized_without_dropping_or_duplicating_viewport_rooms() {
        let active: OwnedRoomId = "!active:example.org".try_into().unwrap();
        let other: OwnedRoomId = "!other:example.org".try_into().unwrap();
        let subscriptions = RoomSubscriptions {
            viewport: vec![other.clone(), active.clone()],
            active: Some(active.clone()),
        };

        assert_eq!(
            coordinated_room_subscriptions(&subscriptions),
            vec![active, other]
        );
    }

    #[test]
    fn room_list_viewport_equality_ignores_recency_reorder() {
        let first: OwnedRoomId = "!a:example.org".try_into().unwrap();
        let second: OwnedRoomId = "!b:example.org".try_into().unwrap();
        assert!(viewport_ids_equivalent(
            &[first.clone(), second.clone()],
            &[second.clone(), first.clone()]
        ));
        let third: OwnedRoomId = "!c:example.org".try_into().unwrap();
        assert!(!viewport_ids_equivalent(
            &[first.clone(), second],
            &[first, third]
        ));
    }
}

#[cfg(test)]
mod auth_recovery_tests {
    use super::*;
    use matrix_sdk::ruma::{device_id, user_id};
    use matrix_sdk::test_utils::mocks::MatrixMockServer;
    use matrix_sdk::{
        authentication::matrix::MatrixSession, store::RoomLoadSettings, SessionMeta, SessionTokens,
    };
    use wiremock::{
        matchers::{method, path},
        Mock, ResponseTemplate,
    };

    async fn refreshing_client(server: &MatrixMockServer) -> Client {
        let client = server
            .client_builder()
            .unlogged()
            .on_builder(|builder| {
                builder
                    .handle_refresh_tokens()
                    .request_config(matrix_sdk::config::RequestConfig::new().retry_limit(0))
            })
            .build()
            .await;
        client
            .matrix_auth()
            .restore_session(
                MatrixSession {
                    meta: SessionMeta {
                        user_id: user_id!("@alice:example.org").to_owned(),
                        device_id: device_id!("DEVICE").to_owned(),
                    },
                    tokens: SessionTokens {
                        access_token: "test-access-old".into(),
                        refresh_token: Some("test-refresh-old".into()),
                    },
                },
                RoomLoadSettings::default(),
            )
            .await
            .unwrap();
        client
    }

    async fn rejected_refresh(status: u16) -> matrix_sdk::HttpError {
        let server = MatrixMockServer::new().await;
        let client = refreshing_client(&server).await;
        Mock::given(method("GET"))
            .and(path("/_matrix/client/versions"))
            .respond_with(
                ResponseTemplate::new(401).set_body_json(
                    serde_json::json!({"errcode":"M_UNKNOWN_TOKEN","error":"expired"}),
                ),
            )
            .expect(1)
            .mount(server.server())
            .await;
        Mock::given(method("POST")).and(path("/_matrix/client/v3/refresh"))
            .respond_with(ResponseTemplate::new(status).set_body_json(serde_json::json!({"errcode":if status == 401 {"M_UNKNOWN_TOKEN"} else {"M_FORBIDDEN"},"error":"rejected"})))
            .expect(1).mount(server.server()).await;
        client.fetch_server_versions(None).await.unwrap_err()
    }

    #[tokio::test]
    async fn rejected_refresh_is_terminal_through_both_sdk_sync_error_wrappers() {
        for status in [401, 403] {
            let error = rejected_refresh(status).await;
            let error = matrix_sdk_ui::sync_service::Error::RoomList(
                matrix_sdk_ui::room_list_service::Error::SlidingSync(matrix_sdk::Error::Http(
                    Box::new(error),
                )),
            );
            assert!(is_terminal_auth_error(&error));
            let snapshot = snapshot_from_sdk_state(&SdkSyncState::Error(Arc::new(error)), 7, true);
            assert_eq!(
                snapshot.failure_diagnostic_id,
                Some(super::super::SYNC_AUTHENTICATION_FAILURE_DIAGNOSTIC_ID)
            );
        }
        let error = rejected_refresh(401).await;
        let error = matrix_sdk_ui::sync_service::Error::EncryptionSync(
            matrix_sdk_ui::encryption_sync_service::Error::SlidingSync(matrix_sdk::Error::Http(
                Box::new(error),
            )),
        );
        assert!(is_terminal_auth_error(&error));
    }

    #[tokio::test]
    async fn refresh_network_failure_is_not_terminal_and_probe_sends_no_authorization() {
        let server = MatrixMockServer::new().await;
        let client = refreshing_client(&server).await;
        Mock::given(method("GET")).and(path("/_matrix/client/versions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"versions":["v1.12"],"unstable_features":{"org.matrix.simplified_msc3575":true}})))
            .expect(1).mount(server.server()).await;
        assert_eq!(probe_sliding_sync(&client).await, Some(true));
        let requests = server.server().received_requests().await.unwrap();
        assert!(requests
            .iter()
            .filter(|request| request.url.path() == "/_matrix/client/versions")
            .all(|request| !request.headers.contains_key("authorization")));
        server.server().reset().await;
        Mock::given(method("POST"))
            .and(path("/_matrix/client/v3/refresh"))
            .respond_with(
                ResponseTemplate::new(500)
                    .set_body_json(serde_json::json!({"errcode":"M_UNKNOWN","error":"temporary"})),
            )
            .mount(server.server())
            .await;
        let error = client
            .matrix_auth()
            .refresh_access_token()
            .await
            .unwrap_err();
        assert!(!is_terminal_auth_error(
            &matrix_sdk::HttpError::RefreshToken(error)
        ));
    }

    #[tokio::test]
    async fn successful_refresh_persists_new_tokens_and_restore_uses_them() {
        use crate::app::lifecycle::{
            load_session_material, matrix_session_from_host_secrets, persist_session_after_login,
            InMemorySessionMaterialVault,
        };
        use crate::app::store::AccountIdentity;
        let server = MatrixMockServer::new().await;
        let client = refreshing_client(&server).await;
        let vault = Arc::new(InMemorySessionMaterialVault::new());
        let identity = AccountIdentity::new("@alice:example.org", &server.uri()).unwrap();
        persist_session_after_login(&client, &identity, vault.as_ref()).unwrap();
        let writer = vault.clone();
        let account = identity.clone();
        client
            .set_session_callbacks(
                Box::new(|_| {
                    Ok(SessionTokens {
                        access_token: "test-access-old".into(),
                        refresh_token: Some("test-refresh-old".into()),
                    })
                }),
                Box::new(move |client| {
                    persist_session_after_login(&client, &account, writer.as_ref())?;
                    Ok(())
                }),
            )
            .unwrap();
        Mock::given(method("POST")).and(path("/_matrix/client/v3/refresh"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"access_token":"test-access-new","refresh_token":"test-refresh-new","expires_in_ms":300000})))
            .expect(1).mount(server.server()).await;
        client.matrix_auth().refresh_access_token().await.unwrap();
        let saved = load_session_material(vault.as_ref(), &identity)
            .unwrap()
            .unwrap();
        let restored =
            matrix_session_from_host_secrets(&identity, &saved.decode_host_secrets().unwrap())
                .unwrap();
        assert_eq!(restored.tokens.access_token, "test-access-new");
        assert_eq!(
            restored.tokens.refresh_token.as_deref(),
            Some("test-refresh-new")
        );
    }

    #[tokio::test]
    async fn rejected_sync_stays_terminal_after_delay_stop_and_wake() {
        let server = MatrixMockServer::new().await;
        let client = refreshing_client(&server).await;
        Mock::given(method("GET")).and(path("/_matrix/client/versions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"versions":["v1.12"],"unstable_features":{"org.matrix.simplified_msc3575":true}})))
            .expect(1).mount(server.server()).await;
        server
            .mock_sliding_sync()
            .error_unknown_token(false)
            .mount()
            .await;
        server
            .mock_upload_keys()
            .error_unknown_token(false)
            .mount()
            .await;
        Mock::given(method("POST")).and(path("/_matrix/client/v3/refresh"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({"errcode":"M_UNKNOWN_TOKEN","error":"refresh token does not exist"})))
            .mount(server.server()).await;
        let owner = build_sync_service(&client, 9, SyncServiceConfig::default())
            .await
            .unwrap();
        owner.start().await.unwrap();
        tokio::time::timeout(Duration::from_secs(3), async {
            while owner.observe().failure_diagnostic_id
                != Some(super::super::SYNC_AUTHENTICATION_FAILURE_DIAGNOSTIC_ID)
            {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("typed rejected refresh must reach the owner");
        let count = server.server().received_requests().await.unwrap().len();
        tokio::time::sleep(Duration::from_millis(5200)).await;
        owner.apply_intent(SyncIntent::Resume).await.unwrap();
        owner.stop().await.unwrap();
        owner.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(
            server.server().received_requests().await.unwrap().len(),
            count,
            "no offline probes or automatic/manual restart of rejected credentials"
        );
        assert_eq!(
            owner.observe().failure_diagnostic_id,
            Some(super::super::SYNC_AUTHENTICATION_FAILURE_DIAGNOSTIC_ID)
        );
        let requests = server.server().received_requests().await.unwrap();
        assert!(
            requests
                .iter()
                .filter(|r| r.url.path().ends_with("/refresh"))
                .count()
                <= 2
        );
    }

    #[tokio::test]
    async fn sdk_reports_success_even_when_rotation_save_fails() {
        use crate::app::lifecycle::{
            load_session_material, persist_session_after_login, InMemorySessionMaterialVault,
        };
        use crate::app::store::AccountIdentity;
        let server = MatrixMockServer::new().await;
        let client = refreshing_client(&server).await;
        let vault = InMemorySessionMaterialVault::new();
        let identity = AccountIdentity::new("@alice:example.org", &server.uri()).unwrap();
        persist_session_after_login(&client, &identity, &vault).unwrap();
        let called = Arc::new(AtomicBool::new(false));
        let callback_called = called.clone();
        client
            .set_session_callbacks(
                Box::new(|_| Err(std::io::Error::other("unused Matrix reload callback").into())),
                Box::new(move |_| {
                    callback_called.store(true, Ordering::Release);
                    Err(std::io::Error::other("test vault unavailable").into())
                }),
            )
            .unwrap();
        Mock::given(method("POST")).and(path("/_matrix/client/v3/refresh"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"access_token":"test-access-new","refresh_token":"test-refresh-new","expires_in_ms":300000})))
            .expect(1).mount(server.server()).await;
        client.matrix_auth().refresh_access_token().await.unwrap();
        assert!(called.load(Ordering::Acquire));
        assert_eq!(
            client
                .matrix_auth()
                .session()
                .unwrap()
                .tokens
                .refresh_token
                .as_deref(),
            Some("test-refresh-new")
        );
        let saved = load_session_material(&vault, &identity)
            .unwrap()
            .unwrap()
            .decode_host_secrets()
            .unwrap();
        assert_eq!(saved.refresh_token.as_deref(), Some("test-refresh-old"));
    }

    #[tokio::test]
    async fn transient_sync_recovery_is_delayed_and_explicit_stop_cancels_it() {
        let server = MatrixMockServer::new().await;
        let client = refreshing_client(&server).await;
        Mock::given(method("GET")).and(path("/_matrix/client/versions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"versions":["v1.12"],"unstable_features":{"org.matrix.simplified_msc3575":true}})))
            .expect(1).mount(server.server()).await;
        server.mock_sliding_sync().error500().mount().await;
        server.mock_upload_keys().error500().mount().await;
        let owner = build_sync_service(&client, 10, SyncServiceConfig::default())
            .await
            .unwrap();
        owner.start().await.unwrap();
        async fn wait_offline(owner: &SyncServiceOwner) {
            tokio::time::timeout(Duration::from_secs(3), async {
                while owner.observe().readiness != SyncReadiness::Offline {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
        }
        wait_offline(&owner).await;
        let first = server.server().received_requests().await.unwrap().len();
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(
            server.server().received_requests().await.unwrap().len(),
            first
        );
        tokio::time::timeout(Duration::from_secs(6), async {
            while server.server().received_requests().await.unwrap().len() == first {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("a transient outage remains recoverable");
        wait_offline(&owner).await;
        owner.stop().await.unwrap();
        let stopped = server.server().received_requests().await.unwrap().len();
        tokio::time::sleep(Duration::from_millis(5200)).await;
        assert_eq!(
            server.server().received_requests().await.unwrap().len(),
            stopped
        );
        assert_eq!(owner.observe().readiness, SyncReadiness::Idle);
    }

    #[tokio::test]
    async fn expired_sliding_sync_session_resubscribes_the_open_room() {
        use wiremock::matchers::body_partial_json;
        let server = MatrixMockServer::new().await;
        let client = refreshing_client(&server).await;
        Mock::given(method("GET")).and(path("/_matrix/client/versions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"versions":["v1.12"],"unstable_features":{"org.matrix.simplified_msc3575":true}})))
            .mount(server.server()).await;
        server.mock_upload_keys().ok().mount().await;
        let sliding_sync = "/_matrix/client/unstable/org.matrix.simplified_msc3575/sync";
        // The first room-list request expires the session (M_UNKNOWN_POS), which
        // makes the SDK clear every room subscription before the loop errors.
        Mock::given(method("POST"))
            .and(path(sliding_sync))
            .and(body_partial_json(
                serde_json::json!({"conn_id": "room-list"}),
            ))
            .respond_with(
                ResponseTemplate::new(400).set_body_json(
                    serde_json::json!({"errcode":"M_UNKNOWN_POS","error":"expired"}),
                ),
            )
            .up_to_n_times(1)
            .with_priority(1)
            .mount(server.server())
            .await;
        Mock::given(method("POST"))
            .and(path(sliding_sync))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(
                        serde_json::json!({"pos":"1","lists":{},"rooms":{},"extensions":{}}),
                    )
                    .set_delay(Duration::from_millis(50)),
            )
            .mount(server.server())
            .await;
        let owner = build_sync_service(&client, 3, SyncServiceConfig::default())
            .await
            .unwrap();
        let room: OwnedRoomId = "!open:example.org".try_into().unwrap();
        owner.subscribe_to_room(&room).await;
        owner.start().await.unwrap();

        let resubscribed = |requests: &[wiremock::Request]| {
            let expired_at = requests.iter().position(|request| {
                request.url.path() == sliding_sync
                    && request
                        .body_json::<serde_json::Value>()
                        .ok()
                        .and_then(|body| {
                            body.get("conn_id")
                                .and_then(|id| id.as_str())
                                .map(|id| id == "room-list")
                        })
                        == Some(true)
            });
            expired_at.is_some_and(|index| {
                requests[index + 1..].iter().any(|request| {
                    request.url.path() == sliding_sync
                        && request
                            .body_json::<serde_json::Value>()
                            .ok()
                            .and_then(|body| body.get("room_subscriptions").cloned())
                            .is_some_and(|subscriptions| subscriptions.get(room.as_str()).is_some())
                })
            })
        };
        // The owner's recovery task restarts a non-terminal error after 5 s.
        tokio::time::timeout(Duration::from_secs(12), async {
            loop {
                let requests = server.server().received_requests().await.unwrap();
                if resubscribed(&requests) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("the restarted session must carry the open room subscription again");
        owner.stop().await.unwrap();
    }
}
