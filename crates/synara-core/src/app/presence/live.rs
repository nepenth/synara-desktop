//! V-PRESENCE.USER — live native user-presence projection and subscriptions.

use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::{SystemTime, UNIX_EPOCH};

use matrix_sdk::{
    event_handler::EventHandlerDropGuard,
    ruma::{
        api::client::presence::set_presence::v3::Request as SetPresenceRequest,
        events::presence::PresenceEvent, presence::PresenceState as RumaPresenceState, UserId,
    },
    Client, StateStore,
};
use tokio::sync::Mutex;

use super::{
    NativePresenceSnapshot, NativePresenceSnapshotResult, NativePresenceState,
    NativePresenceSubscription, NativePresenceUpdate, NativePresenceUpdateOutcome,
    NativePresenceWriteResult, PresenceIndex, PresenceSnapshot, PresenceState,
    PresenceSubscriptionRegistry, MAX_STATUS_MSG_CHARS,
};

/// Shell-supplied sink for presence updates. Desktop maps this to the
/// existing Tauri event; iOS can map it to a UniFFI callback later.
pub type PresenceUpdateEmit = Arc<dyn Fn(NativePresenceUpdate) + Send + Sync>;

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
        emit: PresenceUpdateEmit,
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
        let emit_for_handler = emit;
        let handler = client.add_event_handler(move |event: PresenceEvent| {
            let index = index_for_handler.clone();
            let subscriptions = subscriptions_for_handler.clone();
            let retired = retired_for_handler.clone();
            let emit = emit_for_handler.clone();
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
                    emit(NativePresenceUpdate {
                        subscription_id: subscription_id.clone(),
                        user_id: user_id.clone(),
                        session_generation,
                        outcome,
                    });
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

    /// PUT own presence through the managed client. Own user id is taken from
    /// the live session; callers never supply a user id. Empty `status_msg`
    /// becomes `None`. Failures stay static and must not echo status text.
    pub async fn set(
        &self,
        state: &str,
        status_msg: Option<String>,
    ) -> Result<NativePresenceWriteResult, &'static str> {
        self.ensure_live()?;
        let user_id = self
            .client
            .user_id()
            .ok_or("v-presence-user-owner-missing")?
            .to_owned();
        let presence = parse_presence_write_state(state)?;
        let status_msg = parse_presence_write_status_msg(status_msg)?;
        let mut request = SetPresenceRequest::new(user_id, presence);
        request.status_msg = status_msg;
        self.client
            .send(request)
            .await
            .map_err(|_| "v-presence-set-sdk-failed")?;
        Ok(NativePresenceWriteResult {
            status: "ok".to_owned(),
        })
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

fn parse_presence_write_state(state: &str) -> Result<RumaPresenceState, &'static str> {
    match state {
        "online" => Ok(RumaPresenceState::Online),
        "offline" => Ok(RumaPresenceState::Offline),
        "unavailable" => Ok(RumaPresenceState::Unavailable),
        _ => Err("v-presence-state-unsupported"),
    }
}

fn parse_presence_write_status_msg(
    status_msg: Option<String>,
) -> Result<Option<String>, &'static str> {
    let Some(raw) = status_msg else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > MAX_STATUS_MSG_CHARS {
        return Err("p4.7-status-msg-cap");
    }
    Ok(Some(trimmed.to_owned()))
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
    fn write_state_accepts_closed_vocabulary_only() {
        assert_eq!(
            parse_presence_write_state("online").unwrap(),
            RumaPresenceState::Online
        );
        assert_eq!(
            parse_presence_write_state("offline").unwrap(),
            RumaPresenceState::Offline
        );
        assert_eq!(
            parse_presence_write_state("unavailable").unwrap(),
            RumaPresenceState::Unavailable
        );
        assert_eq!(
            parse_presence_write_state("custom").unwrap_err(),
            "v-presence-state-unsupported"
        );
        assert_eq!(
            parse_presence_write_state("unknown").unwrap_err(),
            "v-presence-state-unsupported"
        );
    }

    #[test]
    fn write_status_msg_empty_becomes_none_and_rejects_oversize() {
        assert_eq!(parse_presence_write_status_msg(None).unwrap(), None);
        assert_eq!(
            parse_presence_write_status_msg(Some(String::new())).unwrap(),
            None
        );
        assert_eq!(
            parse_presence_write_status_msg(Some("   ".to_owned())).unwrap(),
            None
        );
        assert_eq!(
            parse_presence_write_status_msg(Some("coffee".to_owned())).unwrap(),
            Some("coffee".to_owned())
        );
        assert_eq!(
            parse_presence_write_status_msg(Some("x".repeat(MAX_STATUS_MSG_CHARS + 1)))
                .unwrap_err(),
            "p4.7-status-msg-cap"
        );
    }
}
