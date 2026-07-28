//! Widget / Element Call session registry (P9.1 harness foundation).
//!
//! Pure projection of Synara [`WidgetSession`] DTOs. No SDK widget APIs,
//! no dual-backend. URLs must not embed tokens (validated).

use std::collections::HashMap;

use crate::matrix::dto::{RoomId, WidgetId, WidgetKind, WidgetSession, WidgetSessionState};

use super::error::WidgetError;

/// Soft cap on concurrent widget sessions (UI safety).
pub const MAX_WIDGET_SESSIONS: usize = 32;

/// Forbidden substrings in widget URLs (token / secret leakage).
const FORBIDDEN_URL_MARKERS: &[&str] = &[
    "access_token=",
    "accessToken=",
    "refresh_token=",
    "password=",
    "recovery_key=",
    "private_key=",
];

/// Session-generation-stamped widget session registry.
#[derive(Debug, Default)]
pub struct WidgetRegistry {
    session_generation: u64,
    by_id: HashMap<WidgetId, WidgetSession>,
    next_seq: u64,
}

impl WidgetRegistry {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            by_id: HashMap::new(),
            next_seq: 0,
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    fn validate(session: &WidgetSession) -> Result<(), WidgetError> {
        if session.widget_id.is_empty() {
            return Err(WidgetError::Invalid {
                diagnostic_id: "p9.1-empty-widget-id",
            });
        }
        if session.room_id.is_empty() || !session.room_id.starts_with('!') {
            return Err(WidgetError::Invalid {
                diagnostic_id: "p9.1-invalid-room-id",
            });
        }
        if let Some(url) = &session.url {
            let lower = url.to_ascii_lowercase();
            for marker in FORBIDDEN_URL_MARKERS {
                if lower.contains(&marker.to_ascii_lowercase()) {
                    return Err(WidgetError::Invalid {
                        diagnostic_id: "p9.1-forbidden-url-secret",
                    });
                }
            }
        }
        Ok(())
    }

    fn alloc_id(&mut self) -> WidgetId {
        self.next_seq = self.next_seq.saturating_add(1);
        format!("widget-{}", self.next_seq)
    }

    /// Upsert a widget session (host maps SDK → DTO).
    pub fn upsert(&mut self, mut session: WidgetSession) -> Result<WidgetId, WidgetError> {
        if session.widget_id.is_empty() {
            session.widget_id = self.alloc_id();
        }
        Self::validate(&session)?;
        if !self.by_id.contains_key(&session.widget_id) && self.by_id.len() >= MAX_WIDGET_SESSIONS {
            return Err(WidgetError::Invalid {
                diagnostic_id: "p9.1-session-cap",
            });
        }
        let id = session.widget_id.clone();
        self.by_id.insert(id.clone(), session);
        Ok(id)
    }

    /// Create a new Element Call / custom session in Creating state.
    pub fn begin(
        &mut self,
        room_id: impl Into<String>,
        kind: WidgetKind,
    ) -> Result<WidgetId, WidgetError> {
        let room_id = room_id.into();
        let session = WidgetSession {
            widget_id: String::new(),
            room_id,
            kind,
            state: WidgetSessionState::Creating,
            url: None,
            has_active_call: kind == WidgetKind::ElementCall,
        };
        self.upsert(session)
    }

    pub fn get(&self, widget_id: &str) -> Option<&WidgetSession> {
        self.by_id.get(widget_id)
    }

    pub fn set_state(
        &mut self,
        widget_id: &str,
        state: WidgetSessionState,
    ) -> Result<(), WidgetError> {
        let s = self.by_id.get_mut(widget_id).ok_or(WidgetError::Invalid {
            diagnostic_id: "p9.1-unknown-widget-id",
        })?;
        s.state = state;
        if matches!(
            state,
            WidgetSessionState::Ending | WidgetSessionState::Failed
        ) {
            s.has_active_call = false;
        }
        if state == WidgetSessionState::Active && s.kind == WidgetKind::ElementCall {
            s.has_active_call = true;
        }
        Ok(())
    }

    pub fn set_url(&mut self, widget_id: &str, url: Option<String>) -> Result<(), WidgetError> {
        let s = self.by_id.get_mut(widget_id).ok_or(WidgetError::Invalid {
            diagnostic_id: "p9.1-unknown-widget-id",
        })?;
        if let Some(ref u) = url {
            let probe = WidgetSession {
                url: Some(u.clone()),
                ..s.clone()
            };
            Self::validate(&probe)?;
        }
        s.url = url;
        Ok(())
    }

    /// List sessions, optional room filter; active calls first.
    pub fn list(&self, room_id: Option<&str>) -> Vec<&WidgetSession> {
        let mut v: Vec<_> = self
            .by_id
            .values()
            .filter(|s| room_id.is_none_or(|r| s.room_id == r))
            .collect();
        v.sort_by(|a, b| {
            b.has_active_call
                .cmp(&a.has_active_call)
                .then_with(|| a.widget_id.cmp(&b.widget_id))
        });
        v
    }

    pub fn active_call_in_room(&self, room_id: &str) -> Option<&WidgetSession> {
        self.by_id.values().find(|s| {
            s.room_id == room_id
                && s.has_active_call
                && matches!(
                    s.state,
                    WidgetSessionState::Creating | WidgetSessionState::Active
                )
        })
    }

    pub fn remove(&mut self, widget_id: &str) -> bool {
        self.by_id.remove(widget_id).is_some()
    }

    pub fn clear(&mut self) {
        self.by_id.clear();
    }

    /// Bump generation and wipe (logout / account switch).
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.by_id.clear();
        self.next_seq = 0;
    }
}
