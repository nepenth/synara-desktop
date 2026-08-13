//! Credential-free V-PRESENCE.USER presentation DTOs and subscription registry.
//!
//! Live Client stream and Tauri subscribe stay in the desktop shell.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Tauri event: presence may have changed; UI re-snapshots via matrix_get_* commands.
/// Signal only — never carries secret material.
pub const PRESENCE_UPDATED_EVENT: &str = "matrix-presence-updated";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativePresenceState {
    Unknown,
    Offline,
    Online,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
pub struct PresenceSubscriptionRegistry {
    session_generation: u64,
    subscriptions: HashMap<String, PresenceSubscriptionState>,
    retired: bool,
}

impl PresenceSubscriptionRegistry {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            subscriptions: HashMap::new(),
            retired: false,
        }
    }

    pub fn register(
        &mut self,
        subscription_id: String,
        user_id: String,
    ) -> Result<(), &'static str> {
        if self.retired {
            return Err("v-presence-session-not-live");
        }
        self.subscriptions
            .insert(subscription_id, PresenceSubscriptionState { user_id });
        Ok(())
    }

    pub fn recipients(&self, session_generation: u64, user_id: &str) -> Vec<String> {
        if self.retired || session_generation != self.session_generation {
            return Vec::new();
        }
        self.subscriptions
            .iter()
            .filter(|(_, subscription)| subscription.user_id == user_id)
            .map(|(subscription_id, _)| subscription_id.clone())
            .collect()
    }

    pub fn unsubscribe(&mut self, subscription_id: &str) -> Result<(), &'static str> {
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

    pub fn retire(&mut self) {
        self.retired = true;
        self.subscriptions.clear();
    }
}

pub fn subscription_id_generation(subscription_id: &str) -> Option<u64> {
    let suffix = subscription_id.strip_prefix("presence-")?;
    let (generation, counter) = suffix.split_once('-')?;
    if counter.is_empty() || !counter.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    generation.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let raw = serde_json::to_value(&result).unwrap();
        assert_eq!(raw["status"], "unknown");
        assert!(raw.get("snapshot").is_none());
        let back: NativePresenceSnapshotResult = serde_json::from_value(raw).unwrap();
        assert_eq!(back, result);
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
