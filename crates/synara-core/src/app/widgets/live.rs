//! Live experimental widget owner: registry, driver, and URL generation.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use matrix_sdk::ruma::{events::StateEventType, RoomId};
use matrix_sdk::widget::{ClientProperties, WidgetDriver, WidgetDriverHandle, WidgetSettings};
use matrix_sdk::{deserialized_responses::RawAnySyncOrStrippedState, Client, Room, RoomState};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use super::capabilities::{SynaraWidgetCapabilitiesProvider, WidgetGrantPolicy};
use super::registry::{WidgetKind, WidgetRegistry, WidgetSessionRecord, MAX_WIDGET_SESSIONS};
use super::url::{is_safe_widget_url, widget_target_origin};

const ROOM_WIDGET_TYPES: &[&str] = &["im.vector.modular.widgets", "m.widget"];

pub type WidgetToWebviewEmit = Arc<dyn Fn(String, String) + Send + Sync>;
pub type WidgetWindowDestroy = Arc<dyn Fn(String) + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListedWidget {
    pub widget_id: String,
    pub name: String,
    pub origin: String,
    pub url: String,
    pub kind: WidgetKind,
    pub init_on_content_load: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetListSnapshot {
    pub session_generation: u64,
    pub room_id: String,
    pub widgets: Vec<ListedWidget>,
    pub open_sessions: Vec<WidgetSessionRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetOpenResult {
    pub session_id: String,
    pub widget_id: String,
    pub room_id: String,
    pub name: String,
    pub webview_url: String,
    pub target_origin: String,
    pub session_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentWidgetEntry {
    pub id: String,
    pub name: String,
    pub url: String,
}

struct LiveDriver {
    handle: Arc<WidgetDriverHandle>,
    join: JoinHandle<()>,
}

pub struct NativeWidgetOwner {
    client: Client,
    session_generation: u64,
    registry: Arc<Mutex<WidgetRegistry>>,
    drivers: Arc<Mutex<HashMap<String, LiveDriver>>>,
    grants: Arc<Mutex<HashMap<String, WidgetGrantPolicy>>>,
    to_webview: WidgetToWebviewEmit,
    destroy_window: WidgetWindowDestroy,
    next_session: AtomicU64,
    retired: Arc<AtomicBool>,
}

impl NativeWidgetOwner {
    pub fn start(
        client: &Client,
        to_webview: WidgetToWebviewEmit,
        destroy_window: WidgetWindowDestroy,
        session_generation: u64,
    ) -> Result<Self, &'static str> {
        client
            .user_id()
            .ok_or("experimental-widgets-owner-missing")?;
        Ok(Self {
            client: client.clone(),
            session_generation,
            registry: Arc::new(Mutex::new(WidgetRegistry::new())),
            drivers: Arc::new(Mutex::new(HashMap::new())),
            grants: Arc::new(Mutex::new(HashMap::new())),
            to_webview,
            destroy_window,
            next_session: AtomicU64::new(1),
            retired: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub async fn list(
        &self,
        enabled: bool,
        room_id: &str,
        agent_widgets: &[AgentWidgetEntry],
    ) -> Result<WidgetListSnapshot, &'static str> {
        self.ensure_live()?;
        ensure_enabled(enabled)?;
        let mut widgets = self.list_room_state_widgets(room_id).await?;
        for entry in agent_widgets.iter().take(MAX_WIDGET_SESSIONS) {
            if !is_safe_widget_url(&entry.url, true) {
                continue;
            }
            let Some(origin) = widget_target_origin(&entry.url) else {
                continue;
            };
            widgets.push(ListedWidget {
                widget_id: entry.id.clone(),
                name: entry.name.clone(),
                origin,
                url: entry.url.clone(),
                kind: WidgetKind::Agent,
                init_on_content_load: true,
            });
        }
        widgets.truncate(MAX_WIDGET_SESSIONS);
        let open_sessions = self.registry.lock().await.list();
        Ok(WidgetListSnapshot {
            session_generation: self.session_generation,
            room_id: room_id.to_owned(),
            widgets,
            open_sessions,
        })
    }

    pub async fn open(
        &self,
        enabled: bool,
        room_id: &str,
        widget_id: &str,
        name: &str,
        url: &str,
        kind: WidgetKind,
        init_on_content_load: bool,
        policy: WidgetGrantPolicy,
    ) -> Result<WidgetOpenResult, &'static str> {
        self.ensure_live()?;
        ensure_enabled(enabled)?;
        let allow_loopback = matches!(kind, WidgetKind::Agent);
        if matches!(kind, WidgetKind::RoomState) && !is_safe_widget_url(url, false) {
            return Err("experimental-widgets-room-state-url-rejected");
        }
        if !is_safe_widget_url(url, allow_loopback) {
            return Err("experimental-widgets-url-rejected");
        }
        let room = self.joined_room(room_id)?;
        let settings = WidgetSettings::new(widget_id.to_owned(), init_on_content_load, url)
            .map_err(|_| "experimental-widgets-url-rejected")?;
        let client_props = ClientProperties::new("com.whylandcreative.synara.desktop", None, None);
        let webview_url = settings
            .generate_webview_url(&room, client_props)
            .await
            .map_err(|_| "experimental-widgets-url-rejected")?;
        let webview_url = webview_url.to_string();
        if !is_safe_widget_url(&webview_url, allow_loopback) {
            return Err("experimental-widgets-url-rejected");
        }
        let target_origin = settings
            .base_url()
            .map(|url| url.to_string())
            .or_else(|| widget_target_origin(&webview_url))
            .ok_or("experimental-widgets-url-rejected")?;

        let session_id = format!("w{}", self.next_session.fetch_add(1, Ordering::SeqCst));
        let record = WidgetSessionRecord {
            session_id: session_id.clone(),
            widget_id: widget_id.to_owned(),
            room_id: room_id.to_owned(),
            name: name.to_owned(),
            origin: target_origin.clone(),
            kind,
        };
        self.registry.lock().await.insert(record)?;
        self.grants.lock().await.insert(
            grant_key(self.session_generation, room_id, widget_id, &target_origin),
            policy,
        );

        let (driver, handle) = WidgetDriver::new(settings);
        let handle = Arc::new(handle);
        let provider = SynaraWidgetCapabilitiesProvider::new(policy);
        let join = tokio::spawn({
            let room = room.clone();
            async move {
                let _ = driver.run(room, provider).await;
            }
        });
        tokio::spawn({
            let handle = handle.clone();
            let emit = self.to_webview.clone();
            let session_id = session_id.clone();
            let retired = self.retired.clone();
            async move {
                while let Some(message) = handle.recv().await {
                    if retired.load(Ordering::Acquire) {
                        break;
                    }
                    emit(session_id.clone(), message);
                }
            }
        });
        self.drivers.lock().await.insert(
            session_id.clone(),
            LiveDriver {
                handle,
                join,
            },
        );

        Ok(WidgetOpenResult {
            session_id,
            widget_id: widget_id.to_owned(),
            room_id: room_id.to_owned(),
            name: name.to_owned(),
            webview_url,
            target_origin,
            session_generation: self.session_generation,
        })
    }

    pub async fn post(
        &self,
        enabled: bool,
        session_id: &str,
        message: String,
    ) -> Result<(), &'static str> {
        self.ensure_live()?;
        ensure_enabled(enabled)?;
        let drivers = self.drivers.lock().await;
        let live = drivers
            .get(session_id)
            .ok_or("experimental-widgets-session-missing")?;
        if !live.handle.send(message) {
            return Err("experimental-widgets-driver-stopped");
        }
        Ok(())
    }

    pub async fn close(&self, session_id: Option<&str>) -> Result<Vec<String>, &'static str> {
        if session_id.is_none() {
            return Ok(self.close_all().await);
        }
        let session_id = session_id.unwrap_or("");
        self.close_one(session_id).await;
        Ok(vec![session_id.to_owned()])
    }

    pub async fn close_all(&self) -> Vec<String> {
        let records = {
            let mut registry = self.registry.lock().await;
            registry.close_all()
        };
        let mut closed = Vec::new();
        for record in records {
            self.teardown_session(&record.session_id).await;
            closed.push(record.session_id);
        }
        closed
    }

    pub async fn subscribe_snapshot(
        &self,
        enabled: bool,
    ) -> Result<Vec<WidgetSessionRecord>, &'static str> {
        self.ensure_live()?;
        ensure_enabled(enabled)?;
        Ok(self.registry.lock().await.list())
    }

    pub fn retire(&self) {
        self.retired.store(true, Ordering::Release);
    }

    pub async fn retire_and_close(&self) {
        self.retire();
        let _ = self.close_all().await;
        self.registry.lock().await.retire_idempotent();
    }

    async fn close_one(&self, session_id: &str) {
        self.registry.lock().await.remove(session_id);
        self.teardown_session(session_id).await;
    }

    async fn teardown_session(&self, session_id: &str) {
        if let Some(live) = self.drivers.lock().await.remove(session_id) {
            live.join.abort();
        }
        (self.destroy_window)(session_id.to_owned());
    }

    fn ensure_live(&self) -> Result<(), &'static str> {
        if self.retired.load(Ordering::Acquire) {
            Err("experimental-widgets-session-not-live")
        } else {
            Ok(())
        }
    }

    fn joined_room(&self, room_id: &str) -> Result<Room, &'static str> {
        let parsed = RoomId::parse(room_id).map_err(|_| "experimental-widgets-invalid-room")?;
        let room = self
            .client
            .get_room(&parsed)
            .ok_or("experimental-widgets-room-missing")?;
        if room.state() != RoomState::Joined {
            return Err("experimental-widgets-room-missing");
        }
        Ok(room)
    }

    async fn list_room_state_widgets(
        &self,
        room_id: &str,
    ) -> Result<Vec<ListedWidget>, &'static str> {
        let room = self.joined_room(room_id)?;
        let mut widgets = Vec::new();
        for event_type in ROOM_WIDGET_TYPES {
            let raw_events = room
                .get_state_events(StateEventType::from(*event_type))
                .await
                .map_err(|_| "experimental-widgets-state-read-failed")?;
            for raw in raw_events {
                if let Some(widget) = parse_room_widget(room_id, &raw) {
                    widgets.push(widget);
                }
            }
        }
        widgets.truncate(MAX_WIDGET_SESSIONS);
        Ok(widgets)
    }
}

impl Drop for NativeWidgetOwner {
    fn drop(&mut self) {
        self.retired.store(true, Ordering::Release);
        if let Ok(mut drivers) = self.drivers.try_lock() {
            for (session_id, live) in drivers.drain() {
                live.join.abort();
                (self.destroy_window)(session_id);
            }
        }
    }
}

fn ensure_enabled(enabled: bool) -> Result<(), &'static str> {
    if enabled {
        Ok(())
    } else {
        Err("experimental-widgets-disabled")
    }
}

fn grant_key(session_generation: u64, room_id: &str, widget_id: &str, origin: &str) -> String {
    format!("{session_generation}\u{1f}{room_id}\u{1f}{widget_id}\u{1f}{origin}")
}

#[derive(Deserialize)]
struct RoomWidgetContent {
    url: Option<String>,
    name: Option<String>,
    #[serde(rename = "waitForIframeLoad")]
    wait_for_iframe_load: Option<bool>,
}

fn parse_room_widget(_room_id: &str, raw: &RawAnySyncOrStrippedState) -> Option<ListedWidget> {
    let RawAnySyncOrStrippedState::Sync(raw_ev) = raw else {
        return None;
    };
    let value: JsonValue = raw_ev.deserialize_as_unchecked().ok()?;
    let state_key = value.get("state_key")?.as_str()?.to_owned();
    let content = value.get("content")?;
    let parsed: RoomWidgetContent = serde_json::from_value(content.clone()).ok()?;
    let url = parsed.url?;
    if !is_safe_widget_url(&url, false) {
        return None;
    }
    let origin = widget_target_origin(&url)?;
    Some(ListedWidget {
        widget_id: state_key,
        name: parsed
            .name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| origin.clone()),
        origin,
        url,
        kind: WidgetKind::RoomState,
        init_on_content_load: parsed.wait_for_iframe_load.unwrap_or(true),
    })
}

#[cfg(test)]
mod drop_tests {
    use super::*;

    #[test]
    fn grant_key_is_session_room_widget_origin() {
        assert_eq!(
            grant_key(7, "!r:ex", "w1", "https://w.example"),
            "7\u{1f}!r:ex\u{1f}w1\u{1f}https://w.example"
        );
    }

    #[test]
    fn setting_off_fails_closed() {
        assert_eq!(ensure_enabled(false), Err("experimental-widgets-disabled"));
        assert_eq!(ensure_enabled(true), Ok(()));
    }
}
