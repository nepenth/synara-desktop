//! Live room-profile state owned by the managed native Matrix session.
//!
//! This module deliberately projects only the bounded join-rule vocabulary
//! needed by the room-settings Publish-to-Directory gate. It does not own a
//! join-rule writer and never sends SDK event objects over the application
//! event boundary.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use matrix_sdk::{
    event_handler::EventHandlerDropGuard,
    ruma::{events::room::join_rules::SyncRoomJoinRulesEvent, room::JoinRule},
    Client, Room, RoomState,
};

use super::NativeRoomJoinRuleUpdate;

/// Shell-supplied sink for join-rule updates. Desktop maps this to the
/// existing Tauri event; iOS can map it to a UniFFI callback later.
pub type JoinRuleUpdateEmit = Arc<dyn Fn(NativeRoomJoinRuleUpdate) + Send + Sync>;

/// Project the SDK/Ruma enum into the exact product union. Custom rules are
/// intentionally unavailable; in particular they must never become a
/// publishable rule through a string fallback.
pub fn project_join_rule(rule: &JoinRule) -> Option<&'static str> {
    match rule {
        JoinRule::Public => Some("public"),
        JoinRule::Knock => Some("knock"),
        JoinRule::Invite => Some("invite"),
        JoinRule::Restricted(_) => Some("restricted"),
        JoinRule::KnockRestricted(_) => Some("knock_restricted"),
        JoinRule::Private => Some("private"),
        _ => None,
    }
}

/// Owns the native m.room.join_rules update boundary for one managed session.
/// Dropping the guard removes the SDK handler; `retire` makes the lifecycle
/// boundary explicit and prevents a late event from the old generation from
/// reaching the webview during teardown.
pub struct NativeRoomJoinRuleOwner {
    session_generation: u64,
    retired: Arc<AtomicBool>,
    _handler: EventHandlerDropGuard,
}

impl NativeRoomJoinRuleOwner {
    pub fn start(
        client: &Client,
        emit: JoinRuleUpdateEmit,
        session_generation: u64,
    ) -> Result<Self, &'static str> {
        if session_generation == 0 {
            return Err("v-send.r-room-profile-join-rule-owner-invalid-generation");
        }
        client
            .user_id()
            .ok_or("v-send.r-room-profile-join-rule-owner-no-user")?;

        let retired = Arc::new(AtomicBool::new(false));
        let retired_for_handler = retired.clone();
        let handler = client.add_event_handler(move |event: SyncRoomJoinRulesEvent, room: Room| {
            let emit = emit.clone();
            let retired = retired_for_handler.clone();
            async move {
                if retired.load(Ordering::Acquire) {
                    return;
                }

                let room_id = room.room_id().to_string();
                let join_rule = if room.state() == RoomState::Joined {
                    project_join_rule(event.join_rule())
                } else {
                    None
                };
                let update = match join_rule {
                    Some(join_rule) => NativeRoomJoinRuleUpdate::Ready {
                        room_id,
                        session_generation,
                        join_rule,
                    },
                    None => NativeRoomJoinRuleUpdate::Unavailable {
                        room_id,
                        session_generation,
                    },
                };
                emit(update);
            }
        });

        Ok(Self {
            session_generation,
            retired,
            _handler: client.event_handler_drop_guard(handler),
        })
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn retire(&self) {
        self.retired.store(true, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn project_join_rule_is_closed_and_fail_closed() {
        let cases = [
            (JoinRule::Public, Some("public")),
            (JoinRule::Knock, Some("knock")),
            (JoinRule::Invite, Some("invite")),
            (JoinRule::Restricted(Default::default()), Some("restricted")),
            (
                JoinRule::KnockRestricted(Default::default()),
                Some("knock_restricted"),
            ),
            (JoinRule::Private, Some("private")),
        ];
        for (rule, expected) in cases {
            assert_eq!(project_join_rule(&rule), expected);
        }

        let custom = serde_json::from_value::<JoinRule>(json!({
            "join_rule": "org.example.custom"
        }))
        .expect("custom join rule is representable by Ruma");
        assert_eq!(project_join_rule(&custom), None);
    }
}
