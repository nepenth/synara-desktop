//! V-PRESENCE.USER — live native user-presence projection and subscriptions.

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
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

use super::{PresenceIndex, PresenceSnapshot, PresenceState};

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

struct PresenceSubscriptionState {
    user_id: String,
}

/// Owns the global Matrix presence event stream for one authenticated session.
pub struct NativePresenceOwner {
    client: Client,
    session_generation: u64,
    index: Arc<Mutex<PresenceIndex>>,
    subscriptions: Arc<Mutex<HashMap<String, PresenceSubscriptionState>>>,
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
        let subscriptions = Arc::new(Mutex::new(
            HashMap::<String, PresenceSubscriptionState>::new(),
        ));
        let index_for_handler = index.clone();
        let subscriptions_for_handler = subscriptions.clone();
        let app_for_handler = app;
        let handler = client.add_event_handler(move |event: PresenceEvent| {
            let index = index_for_handler.clone();
            let subscriptions = subscriptions_for_handler.clone();
            let app = app_for_handler.clone();
            async move {
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

                let recipients: Vec<String> = subscriptions
                    .lock()
                    .await
                    .iter()
                    .filter(|(_, subscription)| subscription.user_id == user_id)
                    .map(|(subscription_id, _)| subscription_id.clone())
                    .collect();

                for subscription_id in recipients {
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
            next_subscription_id: AtomicU64::new(0),
            _handler: client.event_handler_drop_guard(handler),
        })
    }

    pub async fn snapshot(
        &self,
        user_id: &str,
    ) -> Result<NativePresenceSnapshotResult, &'static str> {
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
            .map_err(|_| "v-presence-projection-failed")?;
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
        UserId::parse(user_id).map_err(|_| "v-presence-invalid-user-id")?;
        let subscription_id = format!(
            "presence-{}-{}",
            self.session_generation,
            self.next_subscription_id.fetch_add(1, Ordering::Relaxed)
        );
        self.subscriptions.lock().await.insert(
            subscription_id.clone(),
            PresenceSubscriptionState {
                user_id: user_id.to_owned(),
            },
        );
        Ok(NativePresenceSubscription {
            subscription_id,
            user_id: user_id.to_owned(),
            session_generation: self.session_generation,
        })
    }

    pub async fn unsubscribe(&self, subscription_id: &str) -> Result<(), &'static str> {
        if subscription_id.trim().is_empty() {
            return Err("v-presence-invalid-subscription-id");
        }
        self.subscriptions.lock().await.remove(subscription_id);
        Ok(())
    }
}

fn project_presence_event(event: &PresenceEvent) -> Result<PresenceSnapshot, &'static str> {
    let state = match event.content.presence.as_str() {
        "online" => PresenceState::Online,
        "unavailable" => PresenceState::Unavailable,
        "offline" => PresenceState::Offline,
        _ => return Err("v-presence-state-unsupported"),
    };
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
        serde_json::from_value(json!({
            "type": "m.presence",
            "sender": "@alice:example.org",
            "content": {
                "presence": state,
                "currently_active": currently_active,
                "last_active_ago": null,
                "status_msg": status_msg,
            }
        }))
        .unwrap()
    }

    #[test]
    fn projection_preserves_supported_state_and_optional_fields() {
        let projected =
            project_presence_event(&event("online", Some(true), Some("coffee".to_owned())))
                .unwrap();
        assert_eq!(projected.user_id, "@alice:example.org");
        assert_eq!(projected.state, PresenceState::Online);
        assert!(projected.currently_active);
        assert_eq!(projected.status_msg.as_deref(), Some("coffee"));
    }

    #[test]
    fn projection_rejects_unsupported_presence_state() {
        assert_eq!(
            project_presence_event(&event("custom", None, None)).unwrap_err(),
            "v-presence-state-unsupported"
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
    }
}
