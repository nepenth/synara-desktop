//! Session-scoped widget registry (P9.1 bounds: cap 32, no credential URLs).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::url::is_safe_widget_url;

pub const MAX_WIDGET_SESSIONS: usize = 32;
pub const MATRIX_WIDGETS_MARKER: &str = "matrix-widgets-experimental-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WidgetKind {
    RoomState,
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetSessionRecord {
    pub session_id: String,
    pub widget_id: String,
    pub room_id: String,
    pub name: String,
    pub origin: String,
    pub kind: WidgetKind,
}

#[derive(Debug, Default)]
pub struct WidgetRegistry {
    sessions: BTreeMap<String, WidgetSessionRecord>,
    retired: bool,
}

impl WidgetRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    pub fn get(&self, session_id: &str) -> Option<&WidgetSessionRecord> {
        self.sessions.get(session_id)
    }

    pub fn list(&self) -> Vec<WidgetSessionRecord> {
        self.sessions.values().cloned().collect()
    }

    pub fn insert(&mut self, record: WidgetSessionRecord) -> Result<(), &'static str> {
        if self.retired {
            return Err("experimental-widgets-session-not-live");
        }
        if self.sessions.len() >= MAX_WIDGET_SESSIONS
            && !self.sessions.contains_key(&record.session_id)
        {
            return Err("experimental-widgets-registry-full");
        }
        let allow_loopback = matches!(record.kind, WidgetKind::Agent);
        if !is_safe_widget_url(&record.origin, allow_loopback)
            && !origin_only_is_safe(&record.origin, allow_loopback)
        {
            return Err("experimental-widgets-url-rejected");
        }
        self.sessions.insert(record.session_id.clone(), record);
        Ok(())
    }

    pub fn remove(&mut self, session_id: &str) -> Option<WidgetSessionRecord> {
        self.sessions.remove(session_id)
    }

    pub fn close_all(&mut self) -> Vec<WidgetSessionRecord> {
        let drained: Vec<_> = self.sessions.values().cloned().collect();
        self.sessions.clear();
        drained
    }

    pub fn retire(&mut self) -> Vec<WidgetSessionRecord> {
        self.retired = true;
        self.close_all()
    }

    pub fn retire_idempotent(&mut self) {
        self.retired = true;
        self.sessions.clear();
    }
}

fn origin_only_is_safe(origin: &str, allow_loopback: bool) -> bool {
    is_safe_widget_url(origin, allow_loopback)
        || is_safe_widget_url(&format!("{origin}/"), allow_loopback)
}
