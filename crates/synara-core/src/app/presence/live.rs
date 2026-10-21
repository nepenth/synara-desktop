//! V-PRESENCE.USER — live native user-presence projection and subscriptions.
//!
//! Sliding sync carries no presence for other users (the SDK's sliding-sync
//! processing always yields an empty presence list), so the sync event
//! handler alone left every other user's presence unknown. Presence for a
//! user someone asked about is read from `GET /presence/{userId}/status`:
//! on a snapshot or a new subscription when the last read is older than
//! [`PRESENCE_FETCH_TTL`], and every [`PRESENCE_REFRESH_INTERVAL`] for users
//! with a live subscription, at most [`PRESENCE_FETCH_CONCURRENCY`] at once.

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures_util::StreamExt;
use matrix_sdk::{
    event_handler::EventHandlerDropGuard,
    ruma::{
        api::client::presence::get_presence, events::presence::PresenceEvent,
        presence::PresenceState as RumaPresenceState, UserId,
    },
    Client,
};
use tokio::sync::Mutex;

use super::{
    NativePresenceSnapshot, NativePresenceSnapshotResult, NativePresenceState,
    NativePresenceSubscription, NativePresenceUpdate, NativePresenceUpdateOutcome,
    NativePresenceWriteResult, PresenceIndex, PresenceSnapshot, PresenceState,
    PresenceSubscriptionRegistry, MAX_STATUS_MSG_CHARS,
};
use crate::app::sync::{spawn_supervised, RestartPolicy, SupervisedTask};

/// Shell-supplied sink for presence updates. Desktop maps this to the
/// existing Tauri event; iOS can map it to a UniFFI callback later.
pub type PresenceUpdateEmit = Arc<dyn Fn(NativePresenceUpdate) + Send + Sync>;

/// A server read of a user's presence is reused for this long, whether it
/// succeeded or not (a server with presence off answers 403/404 every time).
pub const PRESENCE_FETCH_TTL: Duration = Duration::from_secs(50);
/// How often presence of subscribed users is read again.
pub const PRESENCE_REFRESH_INTERVAL: Duration = Duration::from_secs(60);
/// Concurrent presence reads during a refresh.
pub const PRESENCE_FETCH_CONCURRENCY: usize = 4;
const PRESENCE_MAX_FETCH_RECORDS: usize = 1024;
// Each refresh pass must find the previous read expired.
const _: () = assert!(PRESENCE_FETCH_TTL.as_secs() < PRESENCE_REFRESH_INTERVAL.as_secs());

/// State shared by the owner, its sync event handler and its refresh task.
struct PresenceShared {
    client: Client,
    session_generation: u64,
    index: Mutex<PresenceIndex>,
    subscriptions: Mutex<PresenceSubscriptionRegistry>,
    retired: AtomicBool,
    emit: PresenceUpdateEmit,
    /// When each user's presence was last read from the server.
    fetched: std::sync::Mutex<HashMap<String, Instant>>,
}

impl PresenceShared {
    /// Store a projected presence and tell every subscriber of `user_id`.
    /// With `only_if_changed`, an unchanged stored value notifies nobody.
    async fn publish(
        &self,
        user_id: &str,
        projected: Result<PresenceSnapshot, &'static str>,
        only_if_changed: bool,
    ) {
        if self.retired.load(Ordering::Acquire) {
            return;
        }
        let projected = match projected {
            Ok(snapshot) => {
                let mut index = self.index.lock().await;
                let previous = index.get(&snapshot.user_id).cloned();
                let stored = index
                    .set(
                        snapshot.user_id.clone(),
                        snapshot.state,
                        snapshot.currently_active,
                        snapshot.last_active_ts,
                        snapshot.status_msg.clone(),
                    )
                    .map_err(|error| error.diagnostic_id());
                if only_if_changed
                    && stored.as_ref().is_ok_and(|stored| {
                        previous
                            .as_ref()
                            .is_some_and(|previous| same_presence(previous, stored))
                    })
                {
                    return;
                }
                stored
            }
            Err(error) => Err(error),
        };

        let recipients = self
            .subscriptions
            .lock()
            .await
            .recipients(self.session_generation, user_id);
        for subscription_id in recipients {
            if self.retired.load(Ordering::Acquire) {
                return;
            }
            let outcome = match projected.as_ref() {
                Ok(snapshot) if snapshot.state == PresenceState::Unknown => {
                    NativePresenceUpdateOutcome::Unknown
                }
                Ok(snapshot) => NativePresenceUpdateOutcome::Ready {
                    snapshot: NativePresenceSnapshot::from(snapshot.clone()),
                },
                Err(diagnostic_id) => NativePresenceUpdateOutcome::Unavailable { diagnostic_id },
            };
            (self.emit)(NativePresenceUpdate {
                subscription_id: subscription_id.clone(),
                user_id: user_id.to_owned(),
                session_generation: self.session_generation,
                outcome,
            });
        }
    }

    /// Claim a server read for `user_id` unless one happened within
    /// [`PRESENCE_FETCH_TTL`].
    fn claim_fetch(&self, user_id: &str, now: Instant) -> bool {
        let Ok(mut fetched) = self.fetched.lock() else {
            return false;
        };
        if fetched
            .get(user_id)
            .is_some_and(|last| now.saturating_duration_since(*last) < PRESENCE_FETCH_TTL)
        {
            return false;
        }
        if fetched.len() >= PRESENCE_MAX_FETCH_RECORDS {
            fetched.retain(|_, last| now.saturating_duration_since(*last) < PRESENCE_FETCH_TTL);
            if fetched.len() >= PRESENCE_MAX_FETCH_RECORDS {
                fetched.clear();
            }
        }
        fetched.insert(user_id.to_owned(), now);
        true
    }

    /// Read `user_id`'s presence from the server when due and publish it.
    /// `None` when a recent read makes this one unnecessary.
    async fn refresh(
        &self,
        user_id: &UserId,
        only_if_changed: bool,
    ) -> Option<Result<PresenceSnapshot, &'static str>> {
        if self.retired.load(Ordering::Acquire)
            || !self.claim_fetch(user_id.as_str(), Instant::now())
        {
            return None;
        }
        let projected = fetch_presence(&self.client, user_id).await;
        // A failed read (presence disabled, user unknown) leaves the last
        // known value and notifies nobody.
        if projected.is_ok() {
            self.publish(user_id.as_str(), projected.clone(), only_if_changed)
                .await;
        }
        Some(projected)
    }
}

fn same_presence(left: &PresenceSnapshot, right: &PresenceSnapshot) -> bool {
    left.state == right.state
        && left.currently_active == right.currently_active
        && left.status_msg == right.status_msg
}

/// `GET /_matrix/client/v3/presence/{userId}/status`, projected like a sync
/// presence event. Failures stay static and never echo server text.
async fn fetch_presence(
    client: &Client,
    user_id: &UserId,
) -> Result<PresenceSnapshot, &'static str> {
    let response = client
        .send(get_presence::v3::Request::new(user_id.to_owned()))
        .await
        .map_err(|_| "v-presence-fetch-failed")?;
    let state = project_presence_state(&response.presence)?;
    if response
        .status_msg
        .as_ref()
        .is_some_and(|status| status.chars().count() > MAX_STATUS_MSG_CHARS)
    {
        return Err("p4.7-status-msg-cap");
    }
    let last_active_ts = response
        .last_active_ago
        .map(|age| unix_now_ms().saturating_sub(age.as_millis().min(u128::from(u64::MAX)) as u64));
    Ok(PresenceSnapshot {
        user_id: user_id.to_string(),
        state,
        currently_active: response.currently_active.unwrap_or(false),
        last_active_ts,
        status_msg: response.status_msg,
    })
}

fn project_presence_state(state: &RumaPresenceState) -> Result<PresenceState, &'static str> {
    match state.as_str() {
        "online" => Ok(PresenceState::Online),
        "unavailable" => Ok(PresenceState::Unavailable),
        "offline" => Ok(PresenceState::Offline),
        _ => Err("v-presence-state-unsupported"),
    }
}

/// Owns the global Matrix presence event stream for one authenticated session.
pub struct NativePresenceOwner {
    shared: Arc<PresenceShared>,
    next_subscription_id: AtomicU64,
    _handler: EventHandlerDropGuard,
    _refresh: SupervisedTask,
}

impl NativePresenceOwner {
    pub fn start(
        client: &Client,
        emit: PresenceUpdateEmit,
        session_generation: u64,
    ) -> Result<Self, &'static str> {
        client.user_id().ok_or("v-presence-user-owner-missing")?;

        let shared = Arc::new(PresenceShared {
            client: client.clone(),
            session_generation,
            index: Mutex::new(PresenceIndex::new(session_generation)),
            subscriptions: Mutex::new(PresenceSubscriptionRegistry::new(session_generation)),
            retired: AtomicBool::new(false),
            emit,
            fetched: std::sync::Mutex::new(HashMap::new()),
        });
        // Presence events still arrive through full `/sync` (and tests).
        let handler_shared = Arc::downgrade(&shared);
        let handler = client.add_event_handler(move |event: PresenceEvent| {
            let shared = handler_shared.clone();
            async move {
                let Some(shared) = shared.upgrade() else {
                    return;
                };
                let user_id = event.sender.to_string();
                shared
                    .publish(&user_id, project_presence_event(&event), false)
                    .await;
            }
        });
        let refresh_shared = Arc::downgrade(&shared);
        let refresh = spawn_supervised(RestartPolicy::default(), move || {
            let shared = refresh_shared.clone();
            async move {
                let mut tick = tokio::time::interval_at(
                    tokio::time::Instant::now() + PRESENCE_REFRESH_INTERVAL,
                    PRESENCE_REFRESH_INTERVAL,
                );
                tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    tick.tick().await;
                    let Some(shared) = shared.upgrade() else {
                        return std::future::pending().await;
                    };
                    if shared.retired.load(Ordering::Acquire) {
                        return std::future::pending().await;
                    }
                    refresh_subscribed(&shared).await;
                }
            }
        });

        Ok(Self {
            shared,
            next_subscription_id: AtomicU64::new(0),
            _handler: client.event_handler_drop_guard(handler),
            _refresh: refresh,
        })
    }

    fn ensure_live(&self) -> Result<(), &'static str> {
        if self.shared.retired.load(Ordering::Acquire) {
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
        self.shared.retired.store(true, Ordering::Release);
        self.shared.subscriptions.lock().await.retire();
        self.shared.index.lock().await.clear();
    }

    pub async fn snapshot(
        &self,
        user_id: &str,
    ) -> Result<NativePresenceSnapshotResult, &'static str> {
        self.ensure_live()?;
        let user_id = UserId::parse(user_id).map_err(|_| "v-presence-invalid-user-id")?;
        let user_id_string = user_id.to_string();
        let session_generation = self.shared.session_generation;
        let raw = self
            .shared
            .client
            .state_store()
            .get_presence_event(&user_id)
            .await
            .map_err(|_| "v-presence-store-read-failed")?;

        let Some(raw) = raw else {
            // Under sliding sync the store never has another user's
            // presence: read it from the server (or reuse a recent read).
            self.shared.refresh(&user_id, true).await;
            let known = self.shared.index.lock().await.get(&user_id_string).cloned();
            return Ok(match known {
                Some(snapshot) if snapshot.state != PresenceState::Unknown => {
                    NativePresenceSnapshotResult::Ready {
                        session_generation,
                        user_id: user_id_string,
                        snapshot: snapshot.into(),
                    }
                }
                _ => NativePresenceSnapshotResult::Unknown {
                    session_generation,
                    user_id: user_id_string,
                },
            });
        };
        let event: PresenceEvent = raw
            .deserialize()
            .map_err(|_| "v-presence-event-deserialize-failed")?;
        if event.sender != user_id {
            return Err("v-presence-user-mismatch");
        }
        let projected = project_presence_event(&event)?;
        let mut index = self.shared.index.lock().await;
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
            session_generation,
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
        let subscription_id = format!(
            "presence-{}-{}",
            self.shared.session_generation,
            self.next_subscription_id.fetch_add(1, Ordering::Relaxed)
        );
        self.shared
            .subscriptions
            .lock()
            .await
            .register(subscription_id.clone(), user_id.to_string())?;
        // Deliver a first value without waiting for the next refresh.
        let shared = Arc::clone(&self.shared);
        let fetch_user = user_id.clone();
        tokio::spawn(async move {
            shared.refresh(&fetch_user, false).await;
        });
        Ok(NativePresenceSubscription {
            subscription_id,
            user_id: user_id.to_string(),
            session_generation: self.shared.session_generation,
        })
    }

    pub async fn unsubscribe(&self, subscription_id: &str) -> Result<(), &'static str> {
        self.ensure_live()?;
        if subscription_id.trim().is_empty() {
            return Err("v-presence-invalid-subscription-id");
        }
        self.shared
            .subscriptions
            .lock()
            .await
            .unsubscribe(subscription_id)
    }

    /// PUT own presence through the managed client. Own user id is taken from
    /// the live session; callers never supply a user id. Empty `status_msg`
    /// becomes `None`. Failures stay static and must not echo status text.
    ///
    /// Uses `Client::set_presence(..., immediate: true)` so the value is both
    /// sent now and stored as the default for later sliding-sync requests.
    /// A raw presence HTTP call would be overwritten by the next sync, which
    /// defaults to `Online`.
    pub async fn set(
        &self,
        state: &str,
        status_msg: Option<String>,
    ) -> Result<NativePresenceWriteResult, &'static str> {
        self.ensure_live()?;
        let _user_id = self
            .shared
            .client
            .user_id()
            .ok_or("v-presence-user-owner-missing")?;
        let presence = parse_presence_write_state(state)?;
        let status_msg = parse_presence_write_status_msg(status_msg)?;
        self.shared
            .client
            .set_presence(presence, status_msg, true)
            .await
            .map_err(|_| "v-presence-set-sdk-failed")?;
        Ok(NativePresenceWriteResult {
            status: crate::dto::WriteAck::Ok,
        })
    }
}

/// Read every subscribed user's presence again, a few at a time, and notify
/// subscribers of changes.
async fn refresh_subscribed(shared: &Arc<PresenceShared>) {
    let users = shared.subscriptions.lock().await.user_ids();
    futures_util::stream::iter(users)
        .for_each_concurrent(PRESENCE_FETCH_CONCURRENCY, |user_id| {
            let shared = Arc::clone(shared);
            async move {
                if let Ok(user_id) = UserId::parse(&user_id) {
                    shared.refresh(&user_id, true).await;
                }
            }
        })
        .await;
}

impl Drop for NativePresenceOwner {
    fn drop(&mut self) {
        self.shared.retired.store(true, Ordering::Release);
        if let Ok(mut subscriptions) = self.shared.subscriptions.try_lock() {
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
        assert_eq!(
            parse_presence_write_state("☕").unwrap_err(),
            "v-presence-state-unsupported"
        );
        assert_eq!(
            parse_presence_write_state("in a meeting").unwrap_err(),
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

#[cfg(test)]
mod fetch_tests {
    use super::*;
    use matrix_sdk::test_utils::mocks::MatrixMockServer;
    use serde_json::json;
    use wiremock::{
        matchers::{method, path_regex},
        Mock, ResponseTemplate,
    };

    async fn presence_requests(server: &MatrixMockServer, user: &str) -> usize {
        server
            .server()
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .filter(|request| {
                request.url.path().contains("/presence/") && request.url.path().contains(user)
            })
            .count()
    }

    #[tokio::test]
    async fn presence_is_read_from_the_server_when_sync_carries_none() {
        let server = MatrixMockServer::new().await;
        let client = server.client_builder().build().await;
        Mock::given(method("GET"))
            .and(path_regex(
                r"/_matrix/client/v3/presence/[^/]*bob[^/]*/status$",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "presence": "online",
                "currently_active": true,
                "status_msg": "coffee",
                "last_active_ago": 1000
            })))
            .mount(server.server())
            .await;
        Mock::given(method("GET"))
            .and(path_regex(
                r"/_matrix/client/v3/presence/[^/]*carol[^/]*/status$",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "presence": "unavailable"
            })))
            .mount(server.server())
            .await;
        // A server with presence turned off.
        Mock::given(method("GET"))
            .and(path_regex(
                r"/_matrix/client/v3/presence/[^/]*dave[^/]*/status$",
            ))
            .respond_with(ResponseTemplate::new(403).set_body_json(json!({
                "errcode": "M_FORBIDDEN",
                "error": "Presence is disabled"
            })))
            .mount(server.server())
            .await;
        let updates = Arc::new(std::sync::Mutex::new(Vec::<NativePresenceUpdate>::new()));
        let sink = Arc::clone(&updates);
        let owner = NativePresenceOwner::start(
            &client,
            Arc::new(move |update| sink.lock().unwrap().push(update)),
            3,
        )
        .unwrap();

        let NativePresenceSnapshotResult::Ready { snapshot, .. } =
            owner.snapshot("@bob:example.org").await.unwrap()
        else {
            panic!("bob's presence comes from the server");
        };
        assert_eq!(snapshot.state, NativePresenceState::Online);
        assert!(snapshot.currently_active);
        assert_eq!(snapshot.status_msg.as_deref(), Some("coffee"));
        assert!(snapshot.last_active_ts.is_some());
        // A second read within the TTL reuses the first.
        assert!(matches!(
            owner.snapshot("@bob:example.org").await.unwrap(),
            NativePresenceSnapshotResult::Ready { .. }
        ));
        assert_eq!(presence_requests(&server, "bob").await, 1);

        // A new subscription gets a first value without waiting a minute.
        let subscription = owner.subscribe("@carol:example.org").await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            while updates.lock().unwrap().is_empty() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("subscriber notified");
        let update = updates.lock().unwrap()[0].clone();
        assert_eq!(update.subscription_id, subscription.subscription_id);
        assert!(matches!(
            update.outcome,
            NativePresenceUpdateOutcome::Ready { ref snapshot }
                if snapshot.state == NativePresenceState::Unavailable
        ));

        // Presence off: unknown, and not asked again within the TTL.
        assert!(matches!(
            owner.snapshot("@dave:example.org").await.unwrap(),
            NativePresenceSnapshotResult::Unknown { .. }
        ));
        assert!(matches!(
            owner.snapshot("@dave:example.org").await.unwrap(),
            NativePresenceSnapshotResult::Unknown { .. }
        ));
        assert_eq!(presence_requests(&server, "dave").await, 1);
    }

    #[tokio::test]
    async fn subscribed_users_are_refreshed_and_only_changes_notify() {
        let server = MatrixMockServer::new().await;
        let client = server.client_builder().build().await;
        Mock::given(method("GET"))
            .and(path_regex(
                r"/_matrix/client/v3/presence/[^/]*bob[^/]*/status$",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "presence": "online"
            })))
            .mount(server.server())
            .await;
        let updates = Arc::new(std::sync::Mutex::new(Vec::<NativePresenceUpdate>::new()));
        let sink = Arc::clone(&updates);
        let owner = NativePresenceOwner::start(
            &client,
            Arc::new(move |update| sink.lock().unwrap().push(update)),
            3,
        )
        .unwrap();
        owner.subscribe("@bob:example.org").await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            while updates.lock().unwrap().is_empty() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("first value delivered");

        // Within the TTL a refresh pass asks nobody.
        refresh_subscribed(&owner.shared).await;
        assert_eq!(presence_requests(&server, "bob").await, 1);
        // Once the last read is older than the TTL (what the periodic task
        // sees each interval), subscribed users are read again.
        owner.shared.fetched.lock().unwrap().clear();
        refresh_subscribed(&owner.shared).await;
        assert_eq!(presence_requests(&server, "bob").await, 2);
        assert_eq!(
            updates.lock().unwrap().len(),
            1,
            "an unchanged presence notifies nobody"
        );
    }
}
