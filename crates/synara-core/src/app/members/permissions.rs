//! What the signed-in user may do in one room, from the room's power levels.
//!
//! Desktop TS and iOS Swift each parsed `m.room.power_levels` JSON and compared
//! numbers themselves, which missed room-version rules such as creators having
//! infinite power (room v12). Core evaluates the SDK's `RoomPowerLevels`, which
//! applies those rules, and both shells read the result.

use std::collections::BTreeMap;

use matrix_sdk::ruma::events::room::power_levels::{RoomPowerLevels, UserPowerLevel};
use matrix_sdk::ruma::events::{MessageLikeEventType, StateEventType};
use matrix_sdk::ruma::UserId;
use serde::{Deserialize, Serialize};

/// The signed-in user's permissions in one room.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct RoomPermissionCapabilities {
    /// Integer power level, or `None` for a room creator, whose power is
    /// unbounded from room version 12.
    pub own_power_level: Option<i64>,
    pub is_creator: bool,
    pub can_send_message: bool,
    pub can_react: bool,
    pub can_redact_own: bool,
    pub can_redact_others: bool,
    pub can_invite: bool,
    pub can_kick: bool,
    pub can_ban: bool,
    pub can_notify_room: bool,
    pub can_change_name: bool,
    pub can_change_topic: bool,
    pub can_change_avatar: bool,
    pub can_change_canonical_alias: bool,
    pub can_change_history_visibility: bool,
    pub can_change_join_rules: bool,
    pub can_enable_encryption: bool,
    pub can_change_power_levels: bool,
    pub can_change_pinned_events: bool,
    pub can_upgrade_room: bool,
    pub can_manage_space_children: bool,
    /// Whether an event type with no specific level may be sent.
    pub events_default_allowed: bool,
    /// Whether a state type with no specific level may be sent.
    pub state_default_allowed: bool,
    /// Every event type the power levels name explicitly, mapped to whether
    /// the user meets its level. Types not listed fall back to the defaults.
    pub event_allowed: BTreeMap<String, bool>,
}

fn level_int(level: UserPowerLevel) -> Option<i64> {
    match level {
        UserPowerLevel::Int(value) => Some(i64::from(value)),
        _ => None,
    }
}

/// Evaluate `levels` for `user_id`.
pub fn room_permission_capabilities(
    levels: &RoomPowerLevels,
    user_id: &UserId,
) -> RoomPermissionCapabilities {
    let own = levels.for_user(user_id);
    let meets = |required: matrix_sdk::ruma::Int| own >= UserPowerLevel::Int(required);
    let state = |kind: StateEventType| levels.user_can_send_state(user_id, kind);
    let event_allowed = levels
        .events
        .iter()
        .map(|(kind, required)| (kind.to_string(), meets(*required)))
        .collect();
    RoomPermissionCapabilities {
        own_power_level: level_int(own),
        is_creator: matches!(own, UserPowerLevel::Infinite),
        can_send_message: levels.user_can_send_message(user_id, MessageLikeEventType::RoomMessage),
        can_react: levels.user_can_send_message(user_id, MessageLikeEventType::Reaction),
        can_redact_own: levels.user_can_redact_own_event(user_id),
        can_redact_others: levels.user_can_redact_event_of_other(user_id),
        can_invite: levels.user_can_invite(user_id),
        can_kick: levels.user_can_kick(user_id),
        can_ban: levels.user_can_ban(user_id),
        can_notify_room: levels.user_can_trigger_room_notification(user_id),
        can_change_name: state(StateEventType::RoomName),
        can_change_topic: state(StateEventType::RoomTopic),
        can_change_avatar: state(StateEventType::RoomAvatar),
        can_change_canonical_alias: state(StateEventType::RoomCanonicalAlias),
        can_change_history_visibility: state(StateEventType::RoomHistoryVisibility),
        can_change_join_rules: state(StateEventType::RoomJoinRules),
        can_enable_encryption: state(StateEventType::RoomEncryption),
        can_change_power_levels: state(StateEventType::RoomPowerLevels),
        can_change_pinned_events: state(StateEventType::RoomPinnedEvents),
        can_upgrade_room: state(StateEventType::RoomTombstone),
        can_manage_space_children: state(StateEventType::SpaceChild),
        events_default_allowed: meets(levels.events_default),
        state_default_allowed: meets(levels.state_default),
        event_allowed,
    }
}

/// The actions the signed-in user may take on one member.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Record))]
pub struct MemberActionPlan {
    pub can_message: bool,
    pub can_ignore: bool,
    pub can_invite: bool,
    pub can_cancel_invite: bool,
    pub can_accept_knock: bool,
    pub can_deny_knock: bool,
    pub can_remove: bool,
    pub can_ban: bool,
    pub can_unban: bool,
    pub can_edit_power_level: bool,
    /// Preset levels (0, 50, 100) the user may assign to this member.
    pub assignable_power_levels: Vec<i64>,
}

/// Inputs for [`plan_member_actions`]. Levels use `None` for a creator's
/// unbounded power.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Record))]
pub struct MemberActionContext {
    pub is_self: bool,
    /// `join`, `invite`, `knock`, `leave` or `ban`.
    pub member_membership: String,
    pub member_power_level: Option<i64>,
    pub own_power_level: Option<i64>,
    pub can_invite: bool,
    pub can_kick: bool,
    pub can_ban: bool,
    pub can_change_power_levels: bool,
}

fn outranks(own: Option<i64>, member: Option<i64>) -> bool {
    match (own, member) {
        (None, None) => false,
        (None, Some(_)) => true,
        (Some(_), None) => false,
        (Some(own), Some(member)) => own > member,
    }
}

/// Plan the member menu. Kick, ban and power changes need a higher level than
/// the member; nobody can act on themselves except to edit their own level.
#[cfg_attr(feature = "full-uniffi", uniffi::export)]
pub fn plan_member_actions(context: MemberActionContext) -> MemberActionPlan {
    let membership = context.member_membership.as_str();
    let other = !context.is_self;
    let higher = outranks(context.own_power_level, context.member_power_level);
    let can_kick = context.can_kick && higher;
    let can_ban = context.can_ban && higher;
    let can_edit_power = context.can_change_power_levels && (context.is_self || higher);
    // Never assign above your own level; a creator may assign any preset,
    // and nobody hands out more than 100 to someone else.
    let max_assignable = match context.own_power_level {
        None => 100,
        Some(own) if context.is_self => own,
        Some(own) => own.min(100),
    };
    let assignable_power_levels = if can_edit_power {
        [0, 50, 100]
            .into_iter()
            .filter(|level| *level <= max_assignable)
            .collect()
    } else {
        Vec::new()
    };
    MemberActionPlan {
        can_message: other,
        can_ignore: other,
        can_invite: other && membership == "leave" && context.can_invite,
        can_cancel_invite: other && membership == "invite" && can_kick,
        can_accept_knock: other && membership == "knock" && context.can_invite,
        can_deny_knock: other && membership == "knock" && can_kick,
        can_remove: other && membership == "join" && can_kick,
        can_ban: other && membership != "ban" && can_ban,
        can_unban: other && membership == "ban" && can_ban,
        can_edit_power_level: other && membership != "ban" && can_edit_power,
        assignable_power_levels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_sdk::ruma::events::room::power_levels::RoomPowerLevelsEventContent;
    use matrix_sdk::ruma::room_version_rules::AuthorizationRules;
    use matrix_sdk::ruma::{int, owned_user_id, user_id};

    fn levels(own: i64, creators: &[&UserId]) -> RoomPowerLevels {
        let mut content = RoomPowerLevelsEventContent::new(&AuthorizationRules::V1);
        content
            .users
            .insert(owned_user_id!("@me:x"), own.try_into().unwrap());
        content.events.insert("m.room.name".into(), int!(75));
        content.events.insert("com.example.custom".into(), int!(10));
        let rules = if creators.is_empty() {
            AuthorizationRules::V1
        } else {
            AuthorizationRules::V12
        };
        RoomPowerLevels::new(
            content.into(),
            &rules,
            creators.iter().map(|id| (*id).to_owned()),
        )
    }

    #[test]
    fn moderator_gets_moderation_but_not_room_name_at_level_75() {
        let caps = room_permission_capabilities(&levels(50, &[]), user_id!("@me:x"));
        assert_eq!(caps.own_power_level, Some(50));
        assert!(!caps.is_creator);
        assert!(caps.can_send_message && caps.can_kick && caps.can_ban && caps.can_redact_others);
        assert!(!caps.can_change_name, "m.room.name needs 75");
        assert_eq!(caps.event_allowed.get("m.room.name"), Some(&false));
        assert_eq!(caps.event_allowed.get("com.example.custom"), Some(&true));
        assert!(caps.state_default_allowed);
    }

    #[test]
    fn plain_member_cannot_moderate() {
        let caps = room_permission_capabilities(&levels(0, &[]), user_id!("@me:x"));
        assert!(caps.can_send_message);
        assert!(!caps.can_kick && !caps.can_ban && !caps.can_redact_others);
        assert!(!caps.can_change_power_levels && !caps.state_default_allowed);
    }

    #[test]
    fn room_v12_creator_has_unbounded_power() {
        let caps =
            room_permission_capabilities(&levels(0, &[user_id!("@me:x")]), user_id!("@me:x"));
        assert!(caps.is_creator);
        assert_eq!(caps.own_power_level, None);
        assert!(caps.can_change_name && caps.can_change_power_levels && caps.can_ban);
    }

    fn context(membership: &str, own: Option<i64>, member: Option<i64>) -> MemberActionContext {
        MemberActionContext {
            is_self: false,
            member_membership: membership.to_owned(),
            member_power_level: member,
            own_power_level: own,
            can_invite: true,
            can_kick: true,
            can_ban: true,
            can_change_power_levels: true,
        }
    }

    #[test]
    fn member_plan_requires_a_higher_level_to_moderate() {
        let plan = plan_member_actions(context("join", Some(50), Some(50)));
        assert!(!plan.can_remove && !plan.can_ban && !plan.can_edit_power_level);
        let plan = plan_member_actions(context("join", Some(100), Some(50)));
        assert!(plan.can_remove && plan.can_ban && plan.can_edit_power_level);
        assert_eq!(plan.assignable_power_levels, [0, 50, 100]);
    }

    #[test]
    fn member_plan_follows_membership_and_never_targets_self() {
        assert!(plan_member_actions(context("invite", Some(100), Some(0))).can_cancel_invite);
        assert!(plan_member_actions(context("knock", Some(100), Some(0))).can_accept_knock);
        assert!(plan_member_actions(context("ban", Some(100), Some(0))).can_unban);
        assert!(plan_member_actions(context("leave", Some(100), Some(0))).can_invite);
        let mut own = context("join", Some(50), Some(50));
        own.is_self = true;
        let plan = plan_member_actions(own);
        assert!(!plan.can_remove && !plan.can_message && !plan.can_edit_power_level);
        assert_eq!(plan.assignable_power_levels, [0, 50]);
    }

    #[test]
    fn creators_outrank_everyone_but_other_creators() {
        assert!(plan_member_actions(context("join", None, Some(100))).can_ban);
        assert!(!plan_member_actions(context("join", None, None)).can_ban);
        assert!(!plan_member_actions(context("join", Some(100), None)).can_ban);
    }
}
