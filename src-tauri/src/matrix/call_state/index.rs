//! Pure MatrixRTC membership and call-state projection (P10.4).

use std::collections::BTreeMap;

use crate::matrix::dto::RoomId;

use super::error::CallStateError;

/// Maximum projected members in one call session.
pub const MAX_CALL_MEMBERS: usize = 256;

const MAX_CALL_ID_CHARS: usize = 255;
const MAX_DEVICE_LABEL_CHARS: usize = 255;
const MAX_USER_LOCALPART_CHARS: usize = 255;

/// Coarse membership state needed by call UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CallMembership {
    Join,
    Leave,
    Invite,
}

/// Coarse call lifecycle phase needed by call UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CallPhase {
    Idle,
    Ringing,
    Active,
    Ended,
}

/// One privacy-minimized member projection.
///
/// This type intentionally has no `Debug` or `Display` implementation. The
/// localpart is not a full MXID, and the optional device label is opaque UI
/// data rather than a device id, key, token, or other credential.
#[derive(Clone, PartialEq, Eq)]
pub struct CallMember {
    pub user_localpart: String,
    pub membership: CallMembership,
    pub device_label: Option<String>,
}

/// Product-facing summary of one MatrixRTC call session.
///
/// This type intentionally has no `Debug` or `Display` implementation so room,
/// call, member, and opaque device values cannot be accidentally logged.
#[derive(Clone, PartialEq, Eq)]
pub struct CallSessionSummary {
    pub room_id: RoomId,
    pub call_id: String,
    /// Stable ascending order by `user_localpart`.
    pub members: Vec<CallMember>,
    pub phase: CallPhase,
}

type SessionKey = (RoomId, String);

/// Session-generation-stamped call-state summary index.
///
/// The index is a pure harness: it does not start MatrixRTC, enable widgets, or
/// communicate with the SDK or product runtime.
#[derive(Default)]
pub struct CallStateIndex {
    session_generation: u64,
    sessions: BTreeMap<SessionKey, CallSessionSummary>,
}

impl CallStateIndex {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            sessions: BTreeMap::new(),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    pub fn get(&self, room_id: &str, call_id: &str) -> Option<&CallSessionSummary> {
        self.sessions.get(&(room_id.to_owned(), call_id.to_owned()))
    }

    /// Insert or replace a session after validating and ordering all members.
    pub fn upsert_session(
        &mut self,
        mut session: CallSessionSummary,
    ) -> Result<Option<CallSessionSummary>, CallStateError> {
        validate_session_identity(&session.room_id, &session.call_id)?;
        if session.members.len() > MAX_CALL_MEMBERS {
            return Err(CallStateError::Invalid {
                diagnostic_id: "p10.4-member-cap",
            });
        }
        for member in &session.members {
            validate_member(member)?;
        }
        session
            .members
            .sort_by(|left, right| left.user_localpart.cmp(&right.user_localpart));
        if session
            .members
            .windows(2)
            .any(|members| members[0].user_localpart == members[1].user_localpart)
        {
            return Err(CallStateError::Invalid {
                diagnostic_id: "p10.4-duplicate-member",
            });
        }

        let key = (session.room_id.clone(), session.call_id.clone());
        Ok(self.sessions.insert(key, session))
    }

    /// Convenience alias matching the other Matrix projection indexes.
    pub fn upsert(
        &mut self,
        session: CallSessionSummary,
    ) -> Result<Option<CallSessionSummary>, CallStateError> {
        self.upsert_session(session)
    }

    /// Add or replace one member, keeping the session's member list ordered.
    pub fn update_member(
        &mut self,
        room_id: &str,
        call_id: &str,
        member: CallMember,
    ) -> Result<Option<CallMember>, CallStateError> {
        validate_member(&member)?;
        let session = self
            .sessions
            .get_mut(&(room_id.to_owned(), call_id.to_owned()))
            .ok_or(CallStateError::NotFound {
                diagnostic_id: "p10.4-session-not-found",
            })?;

        match session
            .members
            .binary_search_by(|existing| existing.user_localpart.cmp(&member.user_localpart))
        {
            Ok(index) => Ok(Some(std::mem::replace(&mut session.members[index], member))),
            Err(index) => {
                if session.members.len() >= MAX_CALL_MEMBERS {
                    return Err(CallStateError::Invalid {
                        diagnostic_id: "p10.4-member-cap",
                    });
                }
                session.members.insert(index, member);
                Ok(None)
            }
        }
    }

    /// Sessions in one room, deterministically ordered by call id.
    pub fn list_room(&self, room_id: &str) -> Vec<&CallSessionSummary> {
        self.sessions
            .range((room_id.to_owned(), String::new())..)
            .take_while(|((session_room_id, _), _)| session_room_id == room_id)
            .map(|(_, session)| session)
            .collect()
    }

    pub fn clear(&mut self) {
        self.sessions.clear();
    }

    /// Bump generation and wipe on logout or account switch.
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.clear();
    }
}

fn validate_session_identity(room_id: &str, call_id: &str) -> Result<(), CallStateError> {
    if room_id.is_empty() || !room_id.starts_with('!') {
        return Err(CallStateError::Invalid {
            diagnostic_id: "p10.4-invalid-room-id",
        });
    }
    if call_id.is_empty()
        || call_id.chars().count() > MAX_CALL_ID_CHARS
        || call_id.chars().any(char::is_control)
    {
        return Err(CallStateError::Invalid {
            diagnostic_id: "p10.4-invalid-call-id",
        });
    }
    Ok(())
}

fn validate_member(member: &CallMember) -> Result<(), CallStateError> {
    if member.user_localpart.is_empty()
        || member.user_localpart.chars().count() > MAX_USER_LOCALPART_CHARS
        || member.user_localpart.contains(['@', ':'])
        || member.user_localpart.chars().any(char::is_control)
    {
        return Err(CallStateError::Invalid {
            diagnostic_id: "p10.4-invalid-user-localpart",
        });
    }
    if member.device_label.as_ref().is_some_and(|label| {
        label.is_empty()
            || label.chars().count() > MAX_DEVICE_LABEL_CHARS
            || label.chars().any(char::is_control)
    }) {
        return Err(CallStateError::Invalid {
            diagnostic_id: "p10.4-invalid-device-label",
        });
    }
    Ok(())
}
