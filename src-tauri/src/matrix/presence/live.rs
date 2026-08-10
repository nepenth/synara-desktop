//! V-PRESENCE.USER — live native user-presence projection and subscriptions.

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::{SystemTime, UNIX_EPOCH};

use matrix_sdk::{
    event_handler::EventHandlerDropGuard,
    ruma::{events::presence::PresenceEvent, UserId},
    Client, StateStore,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use super::{PresenceIndex, PresenceSnapshot, PresenceState, MAX_STATUS_MSG_CHARS};

pub const PRESENCE_UPDATED_EVENT: &str = "matrix-presence-updated";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativePresenceState {
    Unknown,
    Offline,
    Online,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativePresenceSnapshot {
    pub user_id: String,
    pub state: NativePresenceState,
    pub currently_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_active_ts: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_msg: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum NativePresenceSnapshotResult {
    Ready {
        #[serde(rename = "sessionGeneration")]
        session_generation: u64,
        #[serde(rename = "userId")]
        user_id: String,
        snapshot: NativePresenceSnapshot,
    },
    Unknown {
        #[serde(rename = "sessionGeneration")]
        session_generation: u64,
        #[serde(rename = "userId")]
        user_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativePresenceSubscription {
    pub subscription_id: String,
    pub user_id: String,
    pub session_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum NativePresenceUpdateOutcome {
    Ready {
        snapshot: NativePresenceSnapshot,
    },
    Unknown,
    Unavailable {
        #[serde(rename = "diagnosticId")]
        diagnostic_id: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativePresenceUpdate {
    pub subscription_id: String,
    pub user_id: String,
    pub session_generation: u64,
    pub outcome: NativePresenceUpdateOutcome,
}

#[derive(Debug)]
struct PresenceSubscriptionState {
    user_id: String,
}

#[derive(Debug)]
struct PresenceSubscriptionRegistry {
    session_generation: u64,
    subscriptions: HashMap<String, PresenceSubscriptionState>,
    retired: bool,
}

impl PresenceSubscriptionRegistry {
    fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            subscriptions: HashMap::new(),
            retired: false,
        }
    }

    fn register(&mut self, subscription_id: String, user_id: String) -> Result<(), &'static str> {
        if self.retired {
            return Err("v-presence-session-not-live");
        }
        self.subscriptions
            .insert(subscription_id, PresenceSubscriptionState { user_id });
        Ok(())
    }

    fn recipients(&self, session_generation: u64, user_id: &str) -> Vec<String> {
        if self.retired || session_generation != self.session_generation {
            return Vec::new();
        }
        self.subscriptions
            .iter()
            .filter(|(_, subscription)| subscription.user_id == user_id)
            .map(|(subscription_id, _)| subscription_id.clone())
            .collect()
    }

    fn unsubscribe(&mut self, subscription_id: &str) -> Result<(), &'static str> {
        if self.retired {
            return Err("v-presence-session-not-live");
        }
        match subscription_id_generation(subscription_id) {
            Some(generation) if generation == self.session_generation => {}
            Some(_) => return Err("v-presence-stale-session-generation"),
            None => return Err("v-presence-invalid-subscription-id"),
        }
        // Release is deliberately idempotent for the live generation. This
        // also makes React Strict Mode/profile-close cleanup safe.
        self.subscriptions.remove(subscription_id);
        Ok(())
    }

    fn retire(&mut self) {
        self.retired = true;
        self.subscriptions.clear();
    }
}

fn subscription_id_generation(subscription_id: &str) -> Option<u64> {
    let suffix = subscription_id.strip_prefix("presence-")?;
    let (generation, counter) = suffix.split_once('-')?;
    if counter.is_empty() || !counter.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    generation.parse().ok()
}

/// Owns the global Matrix presence event stream for one authenticated session.
pub struct NativePresenceOwner {
    client: Client,
    session_generation: u64,
    index: Arc<Mutex<PresenceIndex>>,
    subscriptions: Arc<Mutex<PresenceSubscriptionRegistry>>,
    retired: Arc<AtomicBool>,
    next_subscription_id: AtomicU64,
    _handler: EventHandlerDropGuard,
}

impl NativePresenceOwner {
    pub fn start(
        client: &Client,
        app: AppHandle,
        session_generation: u64,
    ) -> Result<Self, &'static str> {
        client.user_id().ok_or("v-presence-user-owner-missing")?;

        let index = Arc::new(Mutex::new(PresenceIndex::new(session_generation)));
        let subscriptions = Arc::new(Mutex::new(PresenceSubscriptionRegistry::new(
            session_generation,
        )));
        let retired = Arc::new(AtomicBool::new(false));
        let index_for_handler = index.clone();
        let subscriptions_for_handler = subscriptions.clone();
        let retired_for_handler = retired.clone();
        let app_for_handler = app;
        let handler = client.add_event_handler(move |event: PresenceEvent| {
            let index = index_for_handler.clone();
            let subscriptions = subscriptions_for_handler.clone();
            let retired = retired_for_handler.clone();
            let app = app_for_handler.clone();
            async move {
                if retired.load(Ordering::Acquire) {
                    return;
                }
                let user_id = event.sender.to_string();
                let projected = match project_presence_event(&event) {
                    Ok(snapshot) => {
                        let mut index = index.lock().await;
                        index
                            .set(
                                snapshot.user_id.clone(),
                                snapshot.state,
                                snapshot.currently_active,
                                snapshot.last_active_ts,
                                snapshot.status_msg.clone(),
                            )
                            .map_err(|error| error.diagnostic_id())
                    }
                    Err(error) => Err(error),
                };
                let diagnostic_id = projected.as_ref().err().copied();

                let recipients = subscriptions
                    .lock()
                    .await
                    .recipients(session_generation, &user_id);

                for subscription_id in recipients {
                    if retired.load(Ordering::Acquire) {
                        return;
                    }
                    let outcome = match projected.as_ref() {
                        Ok(snapshot) if snapshot.state == PresenceState::Unknown => {
                            NativePresenceUpdateOutcome::Unknown
                        }
                        Ok(snapshot) => NativePresenceUpdateOutcome::Ready {
                            snapshot: NativePresenceSnapshot::from(snapshot.clone()),
                        },
                        Err(_) => NativePresenceUpdateOutcome::Unavailable {
                            diagnostic_id: diagnostic_id.unwrap_or("v-presence-event-invalid"),
                        },
                    };
                    let _ = app.emit(
                        PRESENCE_UPDATED_EVENT,
                        NativePresenceUpdate {
                            subscription_id: subscription_id.clone(),
                            user_id: user_id.clone(),
                            session_generation,
                            outcome,
                        },
                    );
                }
            }
        });

        Ok(Self {
            client: client.clone(),
            session_generation,
            index,
            subscriptions,
            retired,
            next_subscription_id: AtomicU64::new(0),
            _handler: client.event_handler_drop_guard(handler),
        })
    }

    fn ensure_live(&self) -> Result<(), &'static str> {
        if self.retired.load(Ordering::Acquire) {
            Err("v-presence-session-not-live")
        } else {
            Ok(())
        }
    }

    /// Retire the owner before the managed client is torn down. The session
    /// boundary currently drops this owner on logout/account switch; keeping
    /// this explicit method makes teardown semantics deterministic for callers
    /// that already own the lifecycle operation.
    pub async fn retire(&self) {
        self.retired.store(true, Ordering::Release);
        self.subscriptions.lock().await.retire();
        self.index.lock().await.clear();
    }

    pub async fn snapshot(
        &self,
        user_id: &str,
    ) -> Result<NativePresenceSnapshotResult, &'static str> {
        self.ensure_live()?;
        let user_id = UserId::parse(user_id).map_err(|_| "v-presence-invalid-user-id")?;
        let user_id_string = user_id.to_string();
        let raw = self
            .client
            .state_store()
            .get_presence_event(&user_id)
            .await
            .map_err(|_| "v-presence-store-read-failed")?;

        let Some(raw) = raw else {
            return Ok(NativePresenceSnapshotResult::Unknown {
                session_generation: self.session_generation,
                user_id: user_id_string,
            });
        };
        let event: PresenceEvent = raw
            .deserialize()
            .map_err(|_| "v-presence-event-deserialize-failed")?;
        if event.sender != user_id {
            return Err("v-presence-user-mismatch");
        }
        let projected = project_presence_event(&event)?;
        let mut index = self.index.lock().await;
        let stored = index
            .set(
                projected.user_id.clone(),
                projected.state,
                projected.currently_active,
                projected.last_active_ts,
                projected.status_msg,
            )
            .map_err(|error| error.diagnostic_id())?;
        Ok(NativePresenceSnapshotResult::Ready {
            session_generation: self.session_generation,
            user_id: user_id_string,
            snapshot: stored.into(),
        })
    }

    pub async fn subscribe(
        &self,
        user_id: &str,
    ) -> Result<NativePresenceSubscription, &'static str> {
        self.ensure_live()?;
        let user_id = UserId::parse(user_id).map_err(|_| "v-presence-invalid-user-id")?;
        let user_id = user_id.to_string();
        let subscription_id = format!(
            "presence-{}-{}",
            self.session_generation,
            self.next_subscription_id.fetch_add(1, Ordering::Relaxed)
        );
        self.subscriptions
            .lock()
            .await
            .register(subscription_id.clone(), user_id.clone())?;
        Ok(NativePresenceSubscription {
            subscription_id,
            user_id,
            session_generation: self.session_generation,
        })
    }

    pub async fn unsubscribe(&self, subscription_id: &str) -> Result<(), &'static str> {
        self.ensure_live()?;
        if subscription_id.trim().is_empty() {
            return Err("v-presence-invalid-subscription-id");
        }
        self.subscriptions.lock().await.unsubscribe(subscription_id)
    }
}

impl Drop for NativePresenceOwner {
    fn drop(&mut self) {
        self.retired.store(true, Ordering::Release);
        if let Ok(mut subscriptions) = self.subscriptions.try_lock() {
            subscriptions.retire();
        }
    }
}

fn project_presence_event(event: &PresenceEvent) -> Result<PresenceSnapshot, &'static str> {
    let state = match event.content.presence.as_str() {
        "online" => PresenceState::Online,
        "unavailable" => PresenceState::Unavailable,
        "offline" => PresenceState::Offline,
        _ => return Err("v-presence-state-unsupported"),
    };
    if event
        .content
        .status_msg
        .as_ref()
        .is_some_and(|status| status.chars().count() > MAX_STATUS_MSG_CHARS)
    {
        return Err("p4.7-status-msg-cap");
    }
    let last_active_ts = event
        .content
        .last_active_ago
        .map(|age| unix_now_ms().saturating_sub(i128::from(age) as u64));
    Ok(PresenceSnapshot {
        user_id: event.sender.to_string(),
        state,
        currently_active: event.content.currently_active.unwrap_or(false),
        last_active_ts,
        status_msg: event.content.status_msg.clone(),
    })
}

fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

impl From<PresenceSnapshot> for NativePresenceSnapshot {
    fn from(snapshot: PresenceSnapshot) -> Self {
        Self {
            user_id: snapshot.user_id,
            state: match snapshot.state {
                PresenceState::Unknown => NativePresenceState::Unknown,
                PresenceState::Offline => NativePresenceState::Offline,
                PresenceState::Online => NativePresenceState::Online,
                PresenceState::Unavailable => NativePresenceState::Unavailable,
            },
            currently_active: snapshot.currently_active,
            last_active_ts: snapshot.last_active_ts,
            status_msg: snapshot.status_msg,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn event(
        state: &str,
        currently_active: Option<bool>,
        status_msg: Option<String>,
    ) -> PresenceEvent {
        event_with_age(state, currently_active, None, status_msg)
    }

    fn event_with_age(
        state: &str,
        currently_active: Option<bool>,
        last_active_ago: Option<u64>,
        status_msg: Option<String>,
    ) -> PresenceEvent {
        serde_json::from_value(json!({
            "type": "m.presence",
            "sender": "@alice:example.org",
            "content": {
                "presence": state,
                "currently_active": currently_active,
                "last_active_ago": last_active_ago,
                "status_msg": status_msg,
            }
        }))
        .unwrap()
    }

    #[test]
    fn projection_preserves_supported_state_and_optional_fields() {
        let projected = project_presence_event(&event_with_age(
            "online",
            Some(true),
            Some(5_000),
            Some("coffee".to_owned()),
        ))
        .unwrap();
        assert_eq!(projected.user_id, "@alice:example.org");
        assert_eq!(projected.state, PresenceState::Online);
        assert!(projected.currently_active);
        assert!(projected.last_active_ts.is_some());
        assert_eq!(projected.status_msg.as_deref(), Some("coffee"));
    }

    #[test]
    fn projection_preserves_all_supported_states() {
        for (state, expected) in [
            ("online", PresenceState::Online),
            ("unavailable", PresenceState::Unavailable),
            ("offline", PresenceState::Offline),
        ] {
            let projected = project_presence_event(&event(state, Some(false), None)).unwrap();
            assert_eq!(projected.state, expected);
            assert!(!projected.currently_active);
        }
    }

    #[test]
    fn missing_presence_record_is_an_explicit_unknown_result() {
        let result = NativePresenceSnapshotResult::Unknown {
            session_generation: 4,
            user_id: "@alice:example.org".to_owned(),
        };
        assert_eq!(
            result,
            NativePresenceSnapshotResult::Unknown {
                session_generation: 4,
                user_id: "@alice:example.org".to_owned(),
            }
        );
        let raw = serde_json::to_value(result).unwrap();
        assert_eq!(raw["status"], "unknown");
        assert!(raw.get("snapshot").is_none());
    }

    #[test]
    fn projection_rejects_unsupported_presence_state() {
        assert_eq!(
            project_presence_event(&event("custom", None, None)).unwrap_err(),
            "v-presence-state-unsupported"
        );
    }

    #[test]
    fn projection_rejects_oversized_status_message() {
        assert_eq!(
            project_presence_event(&event(
                "online",
                None,
                Some("x".repeat(MAX_STATUS_MSG_CHARS + 1)),
            ))
            .unwrap_err(),
            "p4.7-status-msg-cap"
        );
    }

    #[test]
    fn subscription_registry_filters_user_and_generation() {
        let mut registry = PresenceSubscriptionRegistry::new(7);
        registry
            .register("presence-7-0".to_owned(), "@alice:example.org".to_owned())
            .unwrap();
        registry
            .register("presence-7-1".to_owned(), "@bob:example.org".to_owned())
            .unwrap();
        assert_eq!(
            registry.recipients(7, "@alice:example.org"),
            vec!["presence-7-0".to_owned()]
        );
        assert!(registry.recipients(8, "@alice:example.org").is_empty());
        assert!(registry.recipients(7, "@carol:example.org").is_empty());
    }

    #[test]
    fn subscription_registry_is_idempotent_but_rejects_stale_generation() {
        let mut registry = PresenceSubscriptionRegistry::new(7);
        registry
            .register("presence-7-0".to_owned(), "@alice:example.org".to_owned())
            .unwrap();
        registry.unsubscribe("presence-7-0").unwrap();
        registry.unsubscribe("presence-7-0").unwrap();
        assert_eq!(
            registry.unsubscribe("presence-6-0").unwrap_err(),
            "v-presence-stale-session-generation"
        );
        assert_eq!(
            registry
                .unsubscribe("not-a-presence-subscription")
                .unwrap_err(),
            "v-presence-invalid-subscription-id"
        );
    }

    #[test]
    fn subscription_registry_retirement_rejects_new_and_late_work() {
        let mut registry = PresenceSubscriptionRegistry::new(7);
        registry
            .register("presence-7-0".to_owned(), "@alice:example.org".to_owned())
            .unwrap();
        registry.retire();
        assert!(registry.recipients(7, "@alice:example.org").is_empty());
        assert_eq!(
            registry
                .register("presence-7-1".to_owned(), "@alice:example.org".to_owned())
                .unwrap_err(),
            "v-presence-session-not-live"
        );
        assert_eq!(
            registry.unsubscribe("presence-7-0").unwrap_err(),
            "v-presence-session-not-live"
        );
    }

    #[test]
    fn native_snapshot_serializes_privacy_safe_camel_case() {
        let snapshot = NativePresenceSnapshot {
            user_id: "@alice:example.org".into(),
            state: NativePresenceState::Online,
            currently_active: true,
            last_active_ts: Some(1_700_000_000_000),
            status_msg: Some("coffee".into()),
        };
        let raw = serde_json::to_string(&snapshot).unwrap();
        assert!(raw.contains("currentlyActive"));
        assert!(raw.contains("lastActiveTs"));
        assert!(raw.contains("statusMsg"));
        for forbidden in ["accessToken", "refreshToken", "password", "ciphertext"] {
            assert!(!raw.contains(forbidden));
        }

        let update = NativePresenceUpdate {
            subscription_id: "presence-4-0".into(),
            user_id: "@alice:example.org".into(),
            session_generation: 4,
            outcome: NativePresenceUpdateOutcome::Unavailable {
                diagnostic_id: "v-presence-store-read-failed",
            },
        };
        let raw_update = serde_json::to_string(&update).unwrap();
        assert!(raw_update.contains("subscriptionId"));
        assert!(raw_update.contains("sessionGeneration"));
        assert!(!raw_update.contains("coffee"));

        let subscription = NativePresenceSubscription {
            subscription_id: "presence-4-0".into(),
            user_id: "@alice:example.org".into(),
            session_generation: 4,
        };
        let raw_subscription = serde_json::to_value(subscription).unwrap();
        assert_eq!(raw_subscription["subscriptionId"], "presence-4-0");
        assert_eq!(raw_subscription["userId"], "@alice:example.org");
        assert_eq!(raw_subscription["sessionGeneration"], 4);

        let ready_update = NativePresenceUpdate {
            subscription_id: "presence-4-0".into(),
            user_id: "@alice:example.org".into(),
            session_generation: 4,
            outcome: NativePresenceUpdateOutcome::Ready {
                snapshot: NativePresenceSnapshot {
                    user_id: "@alice:example.org".into(),
                    state: NativePresenceState::Offline,
                    currently_active: false,
                    last_active_ts: None,
                    status_msg: None,
                },
            },
        };
        let raw_ready_update = serde_json::to_value(ready_update).unwrap();
        assert_eq!(raw_ready_update["outcome"]["status"], "ready");
        assert_eq!(raw_ready_update["outcome"]["snapshot"]["state"], "offline");
    }
}
