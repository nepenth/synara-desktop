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
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use matrix_sdk::Client;
use matrix_sdk_ui::room_list_service::State as RoomListState;
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

/// How long the SDK may report `Running` without a successful sliding-sync
/// response before the connection counts as not live. Longer than the SDK's
/// 30 s long-poll plus its 30 s network timeout, so a quiet healthy session
/// never trips it; a long-poll left dead by sleep or a network change does.
pub const SYNC_HEARTBEAT_STALE_AFTER: Duration = Duration::from_secs(90);
/// Grace after a start before the first successful response is overdue.
pub const SYNC_HEARTBEAT_STARTUP_GRACE: Duration = Duration::from_secs(30);
/// How often the recovery task checks the heartbeat while the SDK is quiet.
const SYNC_HEARTBEAT_CHECK_INTERVAL: Duration = Duration::from_secs(10);
/// Delay before restarting after an ordinary transient sync error.
const TRANSIENT_ERROR_RESTART_DELAY: Duration = Duration::from_secs(5);
/// Bounds for restarting a sync service that terminated on its own.
const TERMINATED_RESTART_INITIAL: Duration = Duration::from_secs(5);
const TERMINATED_RESTART_MAX: Duration = Duration::from_secs(60);
/// Minimum spacing between classifier probes after `UnknownToken` broadcasts.
const SESSION_CHANGE_PROBE_INTERVAL: Duration = Duration::from_secs(30);

/// Heartbeat thresholds; production uses the constants above.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HeartbeatPolicy {
    pub startup_grace: Duration,
    pub stale_after: Duration,
    pub check_interval: Duration,
}

impl Default for HeartbeatPolicy {
    fn default() -> Self {
        Self {
            startup_grace: SYNC_HEARTBEAT_STARTUP_GRACE,
            stale_after: SYNC_HEARTBEAT_STALE_AFTER,
            check_interval: SYNC_HEARTBEAT_CHECK_INTERVAL,
        }
    }
}

/// Last successful sliding-sync response, measured against the last start.
///
/// The SDK sets `SyncService` state to `Running` as soon as `start()` spawns
/// its loops, before any request succeeds, and keeps it there while a
/// long-poll hangs. This records when a response actually arrived.
#[derive(Debug, Default)]
pub(crate) struct SyncHeartbeat {
    times: std::sync::Mutex<HeartbeatTimes>,
}

#[derive(Debug, Default, Clone, Copy)]
struct HeartbeatTimes {
    started_at: Option<Instant>,
    last_success: Option<Instant>,
}

impl SyncHeartbeat {
    fn lock(&self) -> std::sync::MutexGuard<'_, HeartbeatTimes> {
        self.times
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(crate) fn record_start(&self, now: Instant) {
        self.lock().started_at = Some(now);
    }

    pub(crate) fn record_success(&self, now: Instant) {
        self.lock().last_success = Some(now);
    }

    /// `true` when no response arrived within the startup grace after the
    /// last start, or within `stale_after` of the last response. A service
    /// that was never started is not stale.
    pub(crate) fn is_stale(&self, now: Instant, policy: HeartbeatPolicy) -> bool {
        let times = *self.lock();
        match (times.started_at, times.last_success) {
            (None, _) => false,
            (Some(started), Some(success)) if success >= started => {
                now.saturating_duration_since(success) > policy.stale_after
            }
            (Some(started), _) => now.saturating_duration_since(started) > policy.startup_grace,
        }
    }
}

/// Owned SyncService handle for one supervisor session generation.
pub struct SyncServiceOwner {
    service: Arc<SyncService>,
    session_generation: u64,
    offline_mode_enabled: bool,
    /// Best-effort preflight verdict for server sliding-sync support.
    sliding_sync_capable: Option<bool>,
    room_subscriptions: Arc<Mutex<RoomSubscriptions>>,
    recovery_task: Option<tokio::task::JoinHandle<()>>,
    /// Re-enables send queues after recoverable send errors; aborted with
    /// this owner so it never outlives the session generation.
    send_queue_recovery: tokio::task::JoinHandle<()>,
    authentication_rejected: Arc<AtomicBool>,
    sync_requested: Arc<AtomicBool>,
    lifecycle_gate: Arc<Mutex<()>>,
    heartbeat: Arc<SyncHeartbeat>,
    heartbeat_policy: HeartbeatPolicy,
    /// Records successful room-list responses; aborted with this owner.
    heartbeat_task: tokio::task::JoinHandle<()>,
    /// Re-checks credentials after an SDK `UnknownToken` broadcast.
    session_change_task: tokio::task::JoinHandle<()>,
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
                .with_sliding_sync_capability(self.sliding_sync_capable);
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
        // `Running` only means the loops were spawned. Without a recent
        // successful response the connection is still being (re)established.
        if snapshot.readiness == SyncReadiness::Running
            && self.sync_requested.load(Ordering::Acquire)
            && self
                .heartbeat
                .is_stale(Instant::now(), self.heartbeat_policy)
        {
            snapshot.readiness = SyncReadiness::Offline;
        }
        snapshot
    }

    /// Whether the SDK reports `Running` but no response arrived in time.
    pub fn heartbeat_is_stale(&self) -> bool {
        matches!(self.service.state().get(), SdkSyncState::Running)
            && self
                .heartbeat
                .is_stale(Instant::now(), self.heartbeat_policy)
    }

    /// Start (or restart) underlying sliding syncs.
    pub async fn start(&self) -> Result<SyncReadinessSnapshot, SyncError> {
        let _gate = self.lifecycle_gate.lock().await;
        let snapshot = self.observe();
        if self.authentication_rejected.load(Ordering::Acquire) {
            return Ok(snapshot);
        }
        self.sync_requested.store(true, Ordering::Release);
        let sdk_running = matches!(self.service.state().get(), SdkSyncState::Running);
        if sdk_running
            && self
                .heartbeat
                .is_stale(Instant::now(), self.heartbeat_policy)
        {
            // `SyncService::start` is a no-op while Running, so a dead
            // long-poll would never be replaced. Stop it first.
            self.service.stop().await;
            self.heartbeat.record_start(Instant::now());
        } else if !sdk_running {
            self.heartbeat.record_start(Instant::now());
        }
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
    build_sync_service_with_heartbeat(
        client,
        session_generation,
        config,
        HeartbeatPolicy::default(),
    )
    .await
}

pub(crate) async fn build_sync_service_with_heartbeat(
    client: &Client,
    session_generation: u64,
    config: SyncServiceConfig,
    heartbeat_policy: HeartbeatPolicy,
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
    let heartbeat = Arc::new(SyncHeartbeat::default());
    let heartbeat_task = spawn_sync_heartbeat(&service, heartbeat.clone());
    let session_change_task =
        spawn_session_change_classifier(client.clone(), authentication_rejected.clone());
    let recovery_task = config.offline_mode.then(|| {
        spawn_network_recovery(
            RecoveryContext {
                service: service.clone(),
                authentication_rejected: authentication_rejected.clone(),
                sync_requested: sync_requested.clone(),
                lifecycle_gate: lifecycle_gate.clone(),
                room_subscriptions: room_subscriptions.clone(),
                heartbeat: heartbeat.clone(),
            },
            heartbeat_policy,
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
    // the sync path. On probe failure `None` is stored and sync proceeds.
    let sliding_sync_capable = probe_sliding_sync(client).await;

    Ok(SyncServiceOwner {
        service,
        session_generation,
        offline_mode_enabled: config.offline_mode,
        sliding_sync_capable,
        room_subscriptions,
        recovery_task,
        send_queue_recovery,
        authentication_rejected,
        sync_requested,
        lifecycle_gate,
        heartbeat,
        heartbeat_policy,
        heartbeat_task,
        session_change_task,
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
        self.heartbeat_task.abort();
        self.session_change_task.abort();
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

/// True for the typed sliding-sync `M_UNKNOWN_POS` error. The SDK has already
/// expired the session (and cleared room subscriptions) before reporting it,
/// so a restart can begin immediately instead of waiting out a network delay.
pub(crate) fn is_expired_sync_session_error(error: &(dyn std::error::Error + 'static)) -> bool {
    use matrix_sdk::ruma::api::error::ErrorKind;
    if let Some(error) = error.downcast_ref::<matrix_sdk_ui::sync_service::Error>() {
        return match error {
            matrix_sdk_ui::sync_service::Error::RoomList(inner) => {
                is_expired_sync_session_error(inner)
            }
            matrix_sdk_ui::sync_service::Error::EncryptionSync(inner) => {
                is_expired_sync_session_error(inner)
            }
            _ => false,
        };
    }
    if let Some(error) = error.downcast_ref::<matrix_sdk_ui::room_list_service::Error>() {
        return match error {
            matrix_sdk_ui::room_list_service::Error::SlidingSync(inner) => {
                is_expired_sync_session_error(inner)
            }
            _ => false,
        };
    }
    if let Some(error) = error.downcast_ref::<matrix_sdk_ui::encryption_sync_service::Error>() {
        return match error {
            matrix_sdk_ui::encryption_sync_service::Error::SlidingSync(inner) => {
                is_expired_sync_session_error(inner)
            }
            _ => false,
        };
    }
    if let Some(error) = error.downcast_ref::<matrix_sdk::Error>() {
        return error.client_api_error_kind() == Some(&ErrorKind::UnknownPos);
    }
    error.source().is_some_and(is_expired_sync_session_error)
}

/// Delay before restarting after a non-terminal sync error.
fn restart_delay_for_error(error: &(dyn std::error::Error + 'static)) -> Duration {
    if is_expired_sync_session_error(error) {
        Duration::ZERO
    } else {
        TRANSIENT_ERROR_RESTART_DELAY
    }
}

/// Doubling delay for restarting a sync service that terminated on its own,
/// reset once the service is running again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TerminatedBackoff(Duration);

impl Default for TerminatedBackoff {
    fn default() -> Self {
        Self(TERMINATED_RESTART_INITIAL)
    }
}

impl TerminatedBackoff {
    /// Delay for this attempt; the next one doubles up to the maximum.
    fn next_delay(&mut self) -> Duration {
        let delay = self.0;
        self.0 = (self.0 * 2).min(TERMINATED_RESTART_MAX);
        delay
    }

    fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Record every successful room-list response. The room-list state machine
/// sets its state after each successful sync (even `Running` → `Running`), and
/// sets an `Error`/`Terminated` state instead when a sync fails.
fn spawn_sync_heartbeat(
    service: &SyncService,
    heartbeat: Arc<SyncHeartbeat>,
) -> tokio::task::JoinHandle<()> {
    let mut states = service.room_list_service().state();
    tokio::spawn(async move {
        while let Some(state) = states.next().await {
            if is_successful_room_list_state(&state) {
                heartbeat.record_success(Instant::now());
            }
        }
    })
}

fn is_successful_room_list_state(state: &RoomListState) -> bool {
    matches!(
        state,
        RoomListState::SettingUp | RoomListState::Recovering | RoomListState::Running
    )
}

/// Latch a rejected session that sync has not seen yet, for example while sync
/// is stopped in the background and a user command hits a dead token.
///
/// For password sessions the SDK broadcasts `UnknownToken` after *any* failed
/// refresh, including a network error, so the broadcast is only a trigger. One
/// explicit refresh is classified with [`is_terminal_auth_error`]: only a
/// typed rejection latches, and a successful refresh heals the session.
fn spawn_session_change_classifier(
    client: Client,
    authentication_rejected: Arc<AtomicBool>,
) -> tokio::task::JoinHandle<()> {
    let mut changes = client.subscribe_to_session_changes();
    tokio::spawn(async move {
        let mut last_probe: Option<Instant> = None;
        loop {
            let change = match changes.recv().await {
                Ok(change) => change,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            };
            if !matches!(change, matrix_sdk::SessionChange::UnknownToken(_))
                || authentication_rejected.load(Ordering::Acquire)
            {
                continue;
            }
            let now = Instant::now();
            if last_probe.is_some_and(|probe| {
                now.saturating_duration_since(probe) < SESSION_CHANGE_PROBE_INTERVAL
            }) {
                continue;
            }
            last_probe = Some(now);
            if refresh_confirms_rejection(&client).await {
                authentication_rejected.store(true, Ordering::Release);
            }
        }
    })
}

/// One explicit refresh, classified by type, never by text.
async fn refresh_confirms_rejection(client: &Client) -> bool {
    match client.matrix_auth().refresh_access_token().await {
        Ok(_) => false,
        Err(error) => is_terminal_auth_error(&matrix_sdk::HttpError::RefreshToken(error)),
    }
}

struct RecoveryContext {
    service: Arc<SyncService>,
    authentication_rejected: Arc<AtomicBool>,
    sync_requested: Arc<AtomicBool>,
    lifecycle_gate: Arc<Mutex<()>>,
    room_subscriptions: Arc<Mutex<RoomSubscriptions>>,
    heartbeat: Arc<SyncHeartbeat>,
}

impl RecoveryContext {
    fn may_restart(&self) -> bool {
        self.sync_requested.load(Ordering::Acquire)
            && !self.authentication_rejected.load(Ordering::Acquire)
    }

    /// Restart under the lifecycle gate if `still_needed` holds once the gate
    /// is held. `stop_first` replaces loops the SDK still reports as Running.
    async fn restart_if(&self, stop_first: bool, still_needed: impl FnOnce(&SdkSyncState) -> bool) {
        let _gate = self.lifecycle_gate.lock().await;
        if !self.may_restart() || !still_needed(&self.service.state().get()) {
            return;
        }
        if stop_first {
            self.service.stop().await;
        }
        self.heartbeat.record_start(Instant::now());
        self.service.start().await;
        reapply_room_subscriptions(&self.service, &self.room_subscriptions).await;
    }
}

fn spawn_network_recovery(
    context: RecoveryContext,
    policy: HeartbeatPolicy,
) -> tokio::task::JoinHandle<()> {
    // Subscribe before spawning so a fast first sync failure cannot be missed.
    let mut states = context.service.state();
    tokio::spawn(async move {
        let mut pending = Some(states.get());
        let mut terminated_backoff = TerminatedBackoff::default();
        loop {
            let state = match pending.take() {
                Some(state) => state,
                None => {
                    tokio::select! {
                        next = states.next() => match next {
                            Some(state) => state,
                            None => break,
                        },
                        _ = tokio::time::sleep(policy.check_interval) => {
                            // A long-poll left dead by sleep keeps the SDK in
                            // Running with no error; replace it.
                            if context.may_restart()
                                && context.heartbeat.is_stale(Instant::now(), policy)
                            {
                                context
                                    .restart_if(true, |state| matches!(state, SdkSyncState::Running))
                                    .await;
                            }
                            continue;
                        }
                    }
                }
            };
            match state {
                SdkSyncState::Error(error) if is_terminal_auth_error(error.as_ref()) => {
                    context
                        .authentication_rejected
                        .store(true, Ordering::Release);
                }
                SdkSyncState::Error(error) if context.may_restart() => {
                    let delay = restart_delay_for_error(error.as_ref());
                    tokio::select! {
                        next = states.next() => { pending = next; if pending.is_none() { break; } }
                        _ = tokio::time::sleep(delay) => {
                            context
                                .restart_if(false, |state| {
                                    matches!(state, SdkSyncState::Error(error) if !is_terminal_auth_error(error.as_ref()))
                                })
                                .await;
                        }
                    }
                }
                SdkSyncState::Terminated if context.may_restart() => {
                    // The SDK supervisor ended without our stop() while a
                    // session is installed; bring it back with growing delays.
                    let delay = terminated_backoff.next_delay();
                    tokio::select! {
                        next = states.next() => { pending = next; if pending.is_none() { break; } }
                        _ = tokio::time::sleep(delay) => {
                            context
                                .restart_if(false, |state| matches!(state, SdkSyncState::Terminated))
                                .await;
                        }
                    }
                }
                SdkSyncState::Running => terminated_backoff.reset(),
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
        // `M_UNKNOWN_POS` restarts immediately instead of after the 5 s
        // transient-error delay, so this resolves well inside that delay.
        tokio::time::timeout(Duration::from_secs(3), async {
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

#[cfg(test)]
mod heartbeat_tests {
    use super::*;

    fn policy() -> HeartbeatPolicy {
        HeartbeatPolicy {
            startup_grace: Duration::from_secs(30),
            stale_after: Duration::from_secs(90),
            check_interval: Duration::from_secs(10),
        }
    }

    #[test]
    fn never_started_service_is_not_stale() {
        let heartbeat = SyncHeartbeat::default();
        assert!(!heartbeat.is_stale(Instant::now() + Duration::from_secs(3600), policy()));
    }

    #[test]
    fn first_response_is_overdue_only_after_the_startup_grace() {
        let heartbeat = SyncHeartbeat::default();
        let start = Instant::now();
        heartbeat.record_start(start);
        assert!(!heartbeat.is_stale(start + Duration::from_secs(30), policy()));
        assert!(heartbeat.is_stale(start + Duration::from_secs(31), policy()));
    }

    #[test]
    fn a_response_keeps_the_session_live_until_it_is_old() {
        let heartbeat = SyncHeartbeat::default();
        let start = Instant::now();
        heartbeat.record_start(start);
        heartbeat.record_success(start + Duration::from_secs(1));
        assert!(!heartbeat.is_stale(start + Duration::from_secs(91), policy()));
        assert!(heartbeat.is_stale(start + Duration::from_secs(92), policy()));
    }

    #[test]
    fn a_response_from_before_a_restart_does_not_count() {
        let heartbeat = SyncHeartbeat::default();
        let start = Instant::now();
        heartbeat.record_success(start);
        heartbeat.record_start(start + Duration::from_secs(5));
        assert!(!heartbeat.is_stale(start + Duration::from_secs(35), policy()));
        assert!(
            heartbeat.is_stale(start + Duration::from_secs(36), policy()),
            "after a restart only a new response proves the connection"
        );
    }

    #[test]
    fn only_successful_room_list_states_are_heartbeats() {
        assert!(is_successful_room_list_state(&RoomListState::Running));
        assert!(is_successful_room_list_state(&RoomListState::SettingUp));
        assert!(is_successful_room_list_state(&RoomListState::Recovering));
        assert!(!is_successful_room_list_state(&RoomListState::Init));
        assert!(!is_successful_room_list_state(&RoomListState::Error {
            from: Box::new(RoomListState::Running)
        }));
        assert!(!is_successful_room_list_state(&RoomListState::Terminated {
            from: Box::new(RoomListState::Running)
        }));
    }

    #[test]
    fn self_terminated_restarts_back_off_to_a_minute_and_reset() {
        let mut backoff = TerminatedBackoff::default();
        let delays: Vec<u64> = (0..6).map(|_| backoff.next_delay().as_secs()).collect();
        assert_eq!(delays, vec![5, 10, 20, 40, 60, 60]);
        backoff.reset();
        assert_eq!(backoff.next_delay(), Duration::from_secs(5));
    }
}

#[cfg(test)]
mod liveness_tests {
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

    const SLIDING_SYNC: &str = "/_matrix/client/unstable/org.matrix.simplified_msc3575/sync";

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

    async fn mount_versions(server: &MatrixMockServer) {
        Mock::given(method("GET")).and(path("/_matrix/client/versions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"versions":["v1.12"],"unstable_features":{"org.matrix.simplified_msc3575":true}})))
            .mount(server.server()).await;
    }

    fn sliding_sync_requests(requests: &[wiremock::Request]) -> usize {
        requests
            .iter()
            .filter(|request| request.url.path() == SLIDING_SYNC)
            .count()
    }

    fn fast_policy() -> HeartbeatPolicy {
        HeartbeatPolicy {
            startup_grace: Duration::from_millis(400),
            stale_after: Duration::from_millis(400),
            check_interval: Duration::from_millis(200),
        }
    }

    async fn wait_until(what: &str, limit: Duration, mut done: impl FnMut() -> bool) {
        tokio::time::timeout(limit, async {
            while !done() {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("timed out waiting for {what}"));
    }

    #[tokio::test]
    async fn running_without_any_response_reports_offline_and_is_replaced() {
        let server = MatrixMockServer::new().await;
        let client = refreshing_client(&server).await;
        mount_versions(&server).await;
        server.mock_upload_keys().ok().mount().await;
        // A long-poll that never answers: the SDK keeps reporting Running.
        Mock::given(method("POST"))
            .and(path(SLIDING_SYNC))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(60)))
            .mount(server.server())
            .await;
        // The check interval is longer than the grace, so the stale window is
        // observable before the recovery task replaces the long-poll.
        let policy = HeartbeatPolicy {
            check_interval: Duration::from_millis(1_500),
            ..fast_policy()
        };
        let owner =
            build_sync_service_with_heartbeat(&client, 4, SyncServiceConfig::default(), policy)
                .await
                .unwrap();
        owner.start().await.unwrap();
        assert_eq!(
            owner.observe().readiness,
            SyncReadiness::Running,
            "inside the startup grace the session is still connecting quietly"
        );
        wait_until("a stale Running session", Duration::from_secs(3), || {
            owner.observe().readiness == SyncReadiness::Offline
        })
        .await;
        assert!(owner.heartbeat_is_stale());

        let before = sliding_sync_requests(&server.server().received_requests().await.unwrap());
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let now =
                    sliding_sync_requests(&server.server().received_requests().await.unwrap());
                if now > before {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("the recovery task must replace a dead long-poll");
        owner.stop().await.unwrap();
        assert_eq!(owner.observe().readiness, SyncReadiness::Idle);
    }

    #[tokio::test]
    async fn successful_responses_keep_a_running_session_live() {
        let server = MatrixMockServer::new().await;
        let client = refreshing_client(&server).await;
        mount_versions(&server).await;
        server.mock_upload_keys().ok().mount().await;
        Mock::given(method("POST"))
            .and(path(SLIDING_SYNC))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(
                        serde_json::json!({"pos":"1","lists":{},"rooms":{},"extensions":{}}),
                    )
                    .set_delay(Duration::from_millis(50)),
            )
            .mount(server.server())
            .await;
        let owner = build_sync_service_with_heartbeat(
            &client,
            5,
            SyncServiceConfig::default(),
            fast_policy(),
        )
        .await
        .unwrap();
        owner.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(900)).await;
        assert_eq!(owner.observe().readiness, SyncReadiness::Running);
        assert!(!owner.heartbeat_is_stale());
        owner.stop().await.unwrap();
    }

    async fn mount_rejecting_whoami(server: &MatrixMockServer) {
        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/account/whoami"))
            .respond_with(
                ResponseTemplate::new(401).set_body_json(
                    serde_json::json!({"errcode":"M_UNKNOWN_TOKEN","error":"expired"}),
                ),
            )
            .mount(server.server())
            .await;
    }

    #[tokio::test]
    async fn unknown_token_broadcast_latches_only_a_confirmed_rejection() {
        let server = MatrixMockServer::new().await;
        let client = refreshing_client(&server).await;
        mount_versions(&server).await;
        mount_rejecting_whoami(&server).await;
        Mock::given(method("POST")).and(path("/_matrix/client/v3/refresh"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({"errcode":"M_UNKNOWN_TOKEN","error":"refresh token does not exist"})))
            .mount(server.server()).await;
        // Sync never starts: only a user command sees the dead token.
        let owner = build_sync_service(&client, 6, SyncServiceConfig::default())
            .await
            .unwrap();
        assert!(client.whoami().await.is_err());
        wait_until("the rejection latch", Duration::from_secs(3), || {
            owner.observe().failure_diagnostic_id
                == Some(super::super::SYNC_AUTHENTICATION_FAILURE_DIAGNOSTIC_ID)
        })
        .await;
        assert_eq!(owner.observe().readiness, SyncReadiness::Failed);
        let refreshes = server
            .server()
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|request| request.url.path().ends_with("/refresh"))
            .count();
        assert!(
            refreshes <= 2,
            "one SDK refresh plus at most one classifier probe, got {refreshes}"
        );
    }

    #[tokio::test]
    async fn unknown_token_after_a_transient_refresh_failure_keeps_the_session() {
        let server = MatrixMockServer::new().await;
        let client = refreshing_client(&server).await;
        mount_versions(&server).await;
        mount_rejecting_whoami(&server).await;
        Mock::given(method("POST"))
            .and(path("/_matrix/client/v3/refresh"))
            .respond_with(
                ResponseTemplate::new(500)
                    .set_body_json(serde_json::json!({"errcode":"M_UNKNOWN","error":"temporary"})),
            )
            .mount(server.server())
            .await;
        let owner = build_sync_service(&client, 7, SyncServiceConfig::default())
            .await
            .unwrap();
        assert!(client.whoami().await.is_err());
        // Give the classifier time to probe and classify.
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let refreshes = server
                    .server()
                    .received_requests()
                    .await
                    .unwrap()
                    .iter()
                    .filter(|request| request.url.path().ends_with("/refresh"))
                    .count();
                if refreshes >= 2 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("the broadcast must trigger one classifier probe");
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_ne!(
            owner.observe().failure_diagnostic_id,
            Some(super::super::SYNC_AUTHENTICATION_FAILURE_DIAGNOSTIC_ID),
            "a server error during refresh is not a rejected session"
        );
    }
}
