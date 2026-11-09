//! Which commands to offer while the user types `/…`.

use serde::{Deserialize, Serialize};

use super::registry::{
    CommandArgKind, CommandCategory, CommandPermission, CommandRoomScope, CommandSpec,
    ComposerCommand, COMMANDS,
};
use crate::app::members::RoomPermissionCapabilities;

/// What Core knows about the room a command would run in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandContext {
    /// The user's permissions, or `None` while the power levels are unknown;
    /// then every command is offered and the server has the last word.
    pub capabilities: Option<RoomPermissionCapabilities>,
    pub is_direct: bool,
}

/// One entry of the suggestion list or /help.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Record))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandSuggestion {
    pub command: ComposerCommand,
    /// The primary name, without the slash.
    pub name: String,
    pub aliases: Vec<String>,
    /// The alias the typed prefix matched, when it matched an alias only.
    pub matched_alias: Option<String>,
    /// Usage hint, such as `/kick <@user:server or server> [-r reason]`.
    pub usage: String,
    pub description: String,
    pub category: CommandCategory,
    pub args: Vec<CommandArgKind>,
    pub destructive: bool,
}

impl CommandSuggestion {
    fn from_spec(spec: &CommandSpec, matched_alias: Option<&str>) -> Self {
        Self {
            command: spec.command,
            name: spec.name.to_owned(),
            aliases: spec
                .aliases
                .iter()
                .map(|alias| (*alias).to_owned())
                .collect(),
            matched_alias: matched_alias.map(str::to_owned),
            usage: spec.usage.to_owned(),
            description: spec.description.to_owned(),
            category: spec.category,
            args: spec.args.to_vec(),
            destructive: spec.destructive,
        }
    }
}

/// Whether `permission` is granted. Unknown power levels allow everything.
pub fn permission_allowed(permission: CommandPermission, context: &CommandContext) -> bool {
    let Some(caps) = &context.capabilities else {
        return true;
    };
    match permission {
        CommandPermission::None => true,
        CommandPermission::SendMessage => caps.can_send_message,
        CommandPermission::Invite => caps.can_invite,
        CommandPermission::Kick => caps.can_kick,
        CommandPermission::Ban => caps.can_ban,
        CommandPermission::RedactOthers => caps.can_redact_others,
        CommandPermission::ChangePowerLevels => caps.can_change_power_levels,
        CommandPermission::ChangeName => caps.can_change_name,
        CommandPermission::ChangeTopic => caps.can_change_topic,
        CommandPermission::UpgradeRoom => caps.can_upgrade_room,
        CommandPermission::ServerAcl => caps
            .event_allowed
            .get("m.room.server_acl")
            .copied()
            .unwrap_or(caps.state_default_allowed),
    }
}

pub fn scope_allowed(scope: CommandRoomScope, context: &CommandContext) -> bool {
    match scope {
        CommandRoomScope::Any => true,
        CommandRoomScope::Rooms => !context.is_direct,
        CommandRoomScope::DirectMessages => context.is_direct,
    }
}

/// Whether the user may use `spec` in this room.
pub fn command_available(spec: &CommandSpec, context: &CommandContext) -> bool {
    permission_allowed(spec.permission, context) && scope_allowed(spec.scope, context)
}

/// Commands the user may use here whose name or alias starts with
/// `prefix` (with or without the slash), primary-name matches first.
pub fn command_suggestions(prefix: &str, context: &CommandContext) -> Vec<CommandSuggestion> {
    let prefix = prefix.trim().trim_start_matches('/').to_lowercase();
    let available = || {
        COMMANDS
            .iter()
            .filter(|spec| command_available(spec, context))
    };
    let mut suggestions: Vec<CommandSuggestion> = available()
        .filter(|spec| spec.name.starts_with(&prefix))
        .map(|spec| CommandSuggestion::from_spec(spec, None))
        .collect();
    for spec in available() {
        if spec.name.starts_with(&prefix) {
            continue;
        }
        if let Some(alias) = spec.aliases.iter().find(|alias| alias.starts_with(&prefix)) {
            suggestions.push(CommandSuggestion::from_spec(spec, Some(alias)));
        }
    }
    suggestions
}

/// Whether `command` may be used here; the shell's suggestion list and
/// `run_command` use the same rule.
pub fn command_allowed(command: ComposerCommand, context: &CommandContext) -> bool {
    command_available(super::registry::command_spec(command), context)
}
