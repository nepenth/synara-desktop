//! Timeline focus / event-context opening (P5.4 harness foundation).
//!
//! Pure controller for live vs unread-marker vs event-focused modes and
//! navigation phases. No SDK `TimelineFocus` attach, no production Tauri
//! commands, no dual-backend, no event plaintext in errors.
//!
//! Aligns with `docs/timeline-room-state-reliability-contract.md`:
//! `TimelineMode = live | unread(markerEventId) | focused(eventId)`.

use serde::{Deserialize, Serialize};

use super::error::TimelineError;
use super::registry::TimelineKey;

/// How the timeline window is focused (product projection of SDK focus modes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TimelineMode {
    /// Live room/thread end (follow latest).
    Live,
    /// Bounded context around the shared unread / fully-read frontier.
    Unread {
        /// Marker event id (fully-read / receipt frontier). Id only — no body.
        marker_event_id: String,
    },
    /// Explicit route / deep-link event context with highlight target.
    Focused {
        /// Target event id to highlight. Id only — no body.
        event_id: String,
    },
}

impl TimelineMode {
    pub fn as_kind_str(&self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Unread { .. } => "unread",
            Self::Focused { .. } => "focused",
        }
    }

    pub fn target_event_id(&self) -> Option<&str> {
        match self {
            Self::Live => None,
            Self::Unread { marker_event_id } => Some(marker_event_id.as_str()),
            Self::Focused { event_id } => Some(event_id.as_str()),
        }
    }
}

/// Navigation / open phase for one timeline stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NavigationPhase {
    /// Idle between navigations (or freshly constructed).
    Idle,
    /// Host is loading a context window (focused / unread).
    LoadingContext,
    /// Leaving a focused/unread window and rebinding to live.
    RebindingLive,
    /// Layout settling after a successful open (UI may pin scroll).
    SettlingLayout,
    /// Live bottom confirmed / focused window ready for interaction.
    BottomConfirmed,
    /// Last open failed; may retry after clear_failure.
    Error,
}

impl NavigationPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::LoadingContext => "loading_context",
            Self::RebindingLive => "rebinding_live",
            Self::SettlingLayout => "settling_layout",
            Self::BottomConfirmed => "bottom_confirmed",
            Self::Error => "error",
        }
    }

    pub fn is_busy(self) -> bool {
        matches!(
            self,
            Self::LoadingContext | Self::RebindingLive | Self::SettlingLayout
        )
    }
}

/// Soft bounds for surrounding context (items around target). Host/SDK may
/// return fewer; foundation only validates the request shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextWindow {
    /// Events before the target (older).
    pub before: u32,
    /// Events after the target (newer).
    pub after: u32,
}

impl ContextWindow {
    pub const DEFAULT_BEFORE: u32 = 25;
    pub const DEFAULT_AFTER: u32 = 25;
    pub const MAX_SIDE: u32 = 100;

    pub fn default_bounded() -> Self {
        Self {
            before: Self::DEFAULT_BEFORE,
            after: Self::DEFAULT_AFTER,
        }
    }

    pub fn validate(self) -> Result<(), TimelineError> {
        if self.before > Self::MAX_SIDE || self.after > Self::MAX_SIDE {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.4-invalid-context-window",
            });
        }
        Ok(())
    }
}

impl Default for ContextWindow {
    fn default() -> Self {
        Self::default_bounded()
    }
}

/// Privacy-safe request to open a focused / unread context window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusOpenRequest {
    pub mode: TimelineMode,
    #[serde(default)]
    pub window: ContextWindow,
}

impl FocusOpenRequest {
    pub fn live() -> Self {
        Self {
            mode: TimelineMode::Live,
            window: ContextWindow::default_bounded(),
        }
    }

    pub fn focused(event_id: impl Into<String>) -> Self {
        Self {
            mode: TimelineMode::Focused {
                event_id: event_id.into(),
            },
            window: ContextWindow::default_bounded(),
        }
    }

    pub fn unread(marker_event_id: impl Into<String>) -> Self {
        Self {
            mode: TimelineMode::Unread {
                marker_event_id: marker_event_id.into(),
            },
            window: ContextWindow::default_bounded(),
        }
    }

    pub fn with_window(mut self, window: ContextWindow) -> Self {
        self.window = window;
        self
    }

    pub fn validate(&self) -> Result<(), TimelineError> {
        self.window.validate()?;
        match &self.mode {
            TimelineMode::Live => Ok(()),
            TimelineMode::Unread { marker_event_id } => validate_event_id(marker_event_id),
            TimelineMode::Focused { event_id } => validate_event_id(event_id),
        }
    }
}

fn validate_event_id(event_id: &str) -> Result<(), TimelineError> {
    let t = event_id.trim();
    if t.is_empty() || !t.starts_with('$') {
        return Err(TimelineError::Invalid {
            diagnostic_id: "p5.4-invalid-event-id",
        });
    }
    // Reject whitespace / control only; no body content expected.
    if t.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(TimelineError::Invalid {
            diagnostic_id: "p5.4-invalid-event-id",
        });
    }
    Ok(())
}

/// Outcome of a completed focus/context open (counts only — items via P5.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusOpenOutcome {
    /// How many items the host applied around the focus.
    pub items_applied: u32,
    /// True when the target event was present in the applied window.
    pub target_found: bool,
    /// True when open settled at the live bottom (jump-latest / live mode).
    pub at_live_bottom: bool,
}

/// Focus / navigation controller for one timeline stream.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineFocus {
    key: TimelineKey,
    session_generation: u64,
    mode: TimelineMode,
    phase: NavigationPhase,
    last_window: Option<ContextWindow>,
    opens_completed: u32,
    highlight_event_id: Option<String>,
    failure_diagnostic_id: Option<&'static str>,
}

impl TimelineFocus {
    pub fn new(key: TimelineKey, session_generation: u64) -> Self {
        Self {
            key,
            session_generation,
            mode: TimelineMode::Live,
            phase: NavigationPhase::Idle,
            last_window: None,
            opens_completed: 0,
            highlight_event_id: None,
            failure_diagnostic_id: None,
        }
    }

    pub fn key(&self) -> &TimelineKey {
        &self.key
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn mode(&self) -> &TimelineMode {
        &self.mode
    }

    pub fn phase(&self) -> NavigationPhase {
        self.phase
    }

    pub fn highlight_event_id(&self) -> Option<&str> {
        self.highlight_event_id.as_deref()
    }

    pub fn opens_completed(&self) -> u32 {
        self.opens_completed
    }

    pub fn failure_diagnostic_id(&self) -> Option<&'static str> {
        self.failure_diagnostic_id
    }

    pub fn is_busy(&self) -> bool {
        self.phase.is_busy()
    }

    pub fn is_live(&self) -> bool {
        matches!(self.mode, TimelineMode::Live)
    }

    /// Begin opening a focus mode. Rejects invalid ids, busy navigation.
    pub fn begin_open(&mut self, request: FocusOpenRequest) -> Result<(), TimelineError> {
        request.validate()?;
        if self.phase.is_busy() {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.4-navigation-busy",
            });
        }
        self.failure_diagnostic_id = None;
        self.last_window = Some(request.window);
        match &request.mode {
            TimelineMode::Live => {
                self.mode = TimelineMode::Live;
                self.highlight_event_id = None;
                self.phase = NavigationPhase::RebindingLive;
            }
            TimelineMode::Unread { marker_event_id } => {
                self.mode = TimelineMode::Unread {
                    marker_event_id: marker_event_id.clone(),
                };
                self.highlight_event_id = Some(marker_event_id.clone());
                self.phase = NavigationPhase::LoadingContext;
            }
            TimelineMode::Focused { event_id } => {
                self.mode = TimelineMode::Focused {
                    event_id: event_id.clone(),
                };
                self.highlight_event_id = Some(event_id.clone());
                self.phase = NavigationPhase::LoadingContext;
            }
        }
        Ok(())
    }

    /// Explicit jump-to-latest (rebinding live regardless of current mode).
    pub fn begin_jump_latest(&mut self) -> Result<(), TimelineError> {
        self.begin_open(FocusOpenRequest::live())
    }

    /// Host reports context load finished; enter settling then ready path.
    pub fn complete_open(&mut self, outcome: FocusOpenOutcome) -> Result<(), TimelineError> {
        match self.phase {
            NavigationPhase::LoadingContext | NavigationPhase::RebindingLive => {}
            _ => {
                return Err(TimelineError::Invalid {
                    diagnostic_id: "p5.4-complete-not-in-flight",
                });
            }
        }
        if matches!(
            self.mode,
            TimelineMode::Focused { .. } | TimelineMode::Unread { .. }
        ) && !outcome.target_found
        {
            self.phase = NavigationPhase::Error;
            self.failure_diagnostic_id = Some("p5.4-target-not-found");
            return Err(TimelineError::NotFound {
                diagnostic_id: "p5.4-target-not-found",
            });
        }
        if matches!(self.mode, TimelineMode::Live) && !outcome.at_live_bottom {
            // Live open without bottom confirmation still settles; UI may show
            // jump-latest affordance — host sets at_live_bottom when known.
        }
        if outcome.at_live_bottom {
            self.mode = TimelineMode::Live;
            self.highlight_event_id = None;
        }
        self.phase = NavigationPhase::SettlingLayout;
        self.opens_completed = self.opens_completed.saturating_add(1);
        Ok(())
    }

    /// UI finished layout settle (pin bottom / highlight target).
    pub fn confirm_ready(&mut self) -> Result<(), TimelineError> {
        if self.phase != NavigationPhase::SettlingLayout {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.4-confirm-not-settling",
            });
        }
        self.phase = NavigationPhase::BottomConfirmed;
        Ok(())
    }

    /// Fail an in-flight open with a privacy-safe diagnostic id.
    pub fn fail(&mut self, diagnostic_id: &'static str) -> Result<(), TimelineError> {
        if diagnostic_id.is_empty() {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.4-empty-failure-id",
            });
        }
        if diagnostic_id.contains("access_token")
            || diagnostic_id.contains("secret")
            || diagnostic_id.contains("password")
        {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.4-forbidden-diagnostic",
            });
        }
        match self.phase {
            NavigationPhase::LoadingContext | NavigationPhase::RebindingLive => {}
            _ => {
                return Err(TimelineError::Invalid {
                    diagnostic_id: "p5.4-fail-not-in-flight",
                });
            }
        }
        self.phase = NavigationPhase::Error;
        self.failure_diagnostic_id = Some(diagnostic_id);
        Ok(())
    }

    /// Clear error phase so a new open can begin.
    pub fn clear_failure(&mut self) -> Result<(), TimelineError> {
        if self.phase != NavigationPhase::Error {
            return Err(TimelineError::Invalid {
                diagnostic_id: "p5.4-not-in-error",
            });
        }
        self.phase = NavigationPhase::Idle;
        self.failure_diagnostic_id = None;
        Ok(())
    }

    /// Retire to a new session generation; cancel in-flight navigation.
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.mode = TimelineMode::Live;
        self.highlight_event_id = None;
        self.last_window = None;
        self.opens_completed = 0;
        if self.phase.is_busy() {
            self.phase = NavigationPhase::Error;
            self.failure_diagnostic_id = Some("p5.4-stale-generation-cancelled");
        } else {
            self.phase = NavigationPhase::Idle;
            self.failure_diagnostic_id = None;
        }
    }
}
