//! The shared composer core: slash commands and typed mentions.
//!
//! Desktop parsed and ran slash commands in TypeScript and iOS had none.
//! Both shells now ask Core: which commands to suggest for a prefix (filtered
//! by the user's power levels and the room kind), what typed text means, and
//! what running it did. Shells only render the outcome.

mod mentions;
mod parse;
mod registry;
mod run;
mod suggest;
#[cfg(test)]
mod tests;

pub use mentions::{resolve_typed_mentions, MentionCandidate, TypedMention};
pub use parse::{
    is_room_alias, is_room_id, is_server_name, is_user_id, parse_composer_input, parse_date_ms,
    parse_duration_ms, parse_flags, parse_poll, parse_servers, parse_users,
    split_payload_content_and_flags, split_with_space, typed_command_name, CommandArgs,
    ComposerInput, ComposerMessage, ComposerMsgType, ComposerPoll,
};
pub use registry::{
    command_spec, lookup_command, CommandArgKind, CommandCategory, CommandPermission,
    CommandRoomScope, CommandSpec, ComposerCommand, COMMANDS, LENNY, SHRUG, TABLEFLIP, UNFLIP,
};
pub use run::{
    run_command, server_acl_content, CommandOutcome, CommandResult, ComposerCommandError,
    ComposerErrorKind, ComposerMember, ComposerOps, ComposerRunResponse, OpResult, RedactSummary,
};
pub use suggest::{
    command_allowed, command_available, command_suggestions, permission_allowed, scope_allowed,
    CommandContext, CommandSuggestion,
};
