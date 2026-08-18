//! SyncService ownership + start/stop (P4.1 harness foundation).
//!
//! Single owner for `matrix_sdk_ui::sync_service::SyncService` per session
//! generation. Builds with optional offline mode; never dual-backend.
//!
//! **No** production Tauri commands. **No** room-list projection (P4.2).

use std::sync::Arc;

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
    /// Enable SDK offline mode (periodic `/versions` probe when sync fails).
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
    /// Best-effort preflight verdict for server sliding-sync support.
    sliding_sync_capable: Option<bool>,
    room_subscriptions: Mutex<RoomSubscriptions>,
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
        snapshot_from_sdk_state(&current, self.session_generation, self.offline_mode_enabled)
            .with_sliding_sync_capability(self.sliding_sync_capable)
    }

    /// Start (or restart) underlying sliding syncs.
    pub async fn start(&self) -> Result<SyncReadinessSnapshot, SyncError> {
        self.service.start().await;
        Ok(self.observe())
    }

    /// Stop underlying sliding syncs (background / logout path).
    pub async fn stop(&self) -> Result<SyncReadinessSnapshot, SyncError> {
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
    /// viewport. The SDK replaces the complete subscription set on each call,
    /// so this is coordinated with the active-room subscription below.
    pub async fn subscribe_to_room_list(&self, room_ids: &[OwnedRoomId]) {
        let mut subscriptions = self.room_subscriptions.lock().await;
        if subscriptions.viewport == room_ids {
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
            .subscribe_to_rooms(&room_id_refs)
            .await;
    }
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

    let mut builder = SyncService::builder(client.clone());
    if config.offline_mode {
        builder = builder.with_offline_mode();
    }

    let service = builder.build().await.map_err(map_build_error)?;
    // Best-effort server capability probe: purely informational, never gates
    // the sync path. On probe failure `None` is stored and sync proceeds.
    let sliding_sync_capable = probe_sliding_sync(client).await;

    Ok(SyncServiceOwner {
        service: Arc::new(service),
        session_generation,
        offline_mode_enabled: config.offline_mode,
        sliding_sync_capable,
        room_subscriptions: Mutex::new(RoomSubscriptions::default()),
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
}
