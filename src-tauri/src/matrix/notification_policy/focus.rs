//! Pure app-focus notification suppression policy.

use crate::matrix::dto::RoomId;

/// Whether the application currently has user focus.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum FocusState {
    Focused,
    #[default]
    Background,
}

/// Tracks enough focus context to decide whether a room candidate is suppressed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SuppressionPolicy {
    focus_state: FocusState,
    focused_room_id: Option<RoomId>,
}

impl SuppressionPolicy {
    pub fn new(focus_state: FocusState, focused_room_id: Option<RoomId>) -> Self {
        Self {
            focus_state,
            focused_room_id,
        }
    }

    pub fn focus_state(&self) -> FocusState {
        self.focus_state
    }

    pub fn focused_room(&self) -> Option<&str> {
        self.focused_room_id.as_deref()
    }

    /// Replace the current app and room focus projection.
    ///
    /// A room may remain selected while the app is in the background; background
    /// candidates are still allowed.
    pub fn update(&mut self, focus_state: FocusState, focused_room_id: Option<RoomId>) {
        self.focus_state = focus_state;
        self.focused_room_id = focused_room_id;
    }

    /// Suppress only when the app is focused and the candidate belongs to the
    /// currently focused room.
    pub fn should_suppress(&self, candidate_room_id: &str) -> bool {
        self.focus_state == FocusState::Focused
            && self.focused_room_id.as_deref() == Some(candidate_room_id)
    }
}
