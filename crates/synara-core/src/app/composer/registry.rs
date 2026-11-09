//! Every slash command the composer knows, with what it needs to run.
//!
//! Desktop and iOS list, filter and explain commands from this one table, so
//! a command added here shows up in both suggestion lists and in /help.

use serde::{Deserialize, Serialize};

/// A slash command. The wire name is the command's primary name.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Enum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComposerCommand {
    Me,
    Notice,
    Shrug,
    TableFlip,
    Unflip,
    Lenny,
    Spoiler,
    Plain,
    Html,
    Poll,
    Gif,
    StartDm,
    Msg,
    Query,
    Join,
    Leave,
    JumpToDate,
    Whois,
    Invite,
    Disinvite,
    Kick,
    Ban,
    Unban,
    Delete,
    Op,
    Deop,
    Ignore,
    Unignore,
    Topic,
    RoomName,
    Acl,
    UpgradeRoom,
    ConvertToDm,
    ConvertToRoom,
    Nick,
    MyRoomNick,
    MyAvatar,
    MyRoomAvatar,
    Help,
}

/// How suggestion lists and /help group commands.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Enum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandCategory {
    Messages,
    Navigation,
    People,
    Moderation,
    RoomSettings,
    Profile,
    Other,
}

/// The kinds of argument a command takes, in order, for usage hints.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Enum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandArgKind {
    /// Free text: the message, a name or a topic.
    Text,
    /// One user: a Matrix ID, or the name of a room member.
    User,
    /// One or more Matrix user IDs.
    Users,
    /// Server names whose members the command acts on.
    Servers,
    /// Room aliases (`#room:server`) or IDs (`!id:server`).
    Rooms,
    /// `-r reason`.
    Reason,
    /// A power level number.
    PowerLevel,
    /// A `YYYY-MM-DD` date.
    Date,
    /// An `mxc://` image address.
    MxcUri,
    /// A room version such as `11`.
    RoomVersion,
    /// `-past 1d|2h|5m|30s`, with optional `-t type`.
    Since,
    /// `-a`, `-d`, `-ra`, `-rd` server lists.
    AclFlags,
    /// `Question | Option | Option [| max=N]`.
    Poll,
    /// A command name.
    Command,
}

/// What the signed-in user must be allowed to do in the room.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Enum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandPermission {
    None,
    SendMessage,
    Invite,
    Kick,
    Ban,
    RedactOthers,
    ChangePowerLevels,
    ChangeName,
    ChangeTopic,
    ServerAcl,
    UpgradeRoom,
}

/// Which rooms a command makes sense in.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Enum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandRoomScope {
    Any,
    /// Group rooms only, not direct messages.
    Rooms,
    /// Direct messages only.
    DirectMessages,
}

/// One registry entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandSpec {
    pub command: ComposerCommand,
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub usage: &'static str,
    pub description: &'static str,
    pub category: CommandCategory,
    pub args: &'static [CommandArgKind],
    /// Asks the user to confirm before it runs.
    pub destructive: bool,
    pub permission: CommandPermission,
    pub scope: CommandRoomScope,
}

pub const SHRUG: &str = "¯\\_(ツ)_/¯";
pub const TABLEFLIP: &str = "(╯°□°)╯︵ ┻━┻";
pub const UNFLIP: &str = "┬─┬ノ( º_ºノ)";
pub const LENNY: &str = "( ͡° ͜ʖ ͡°)";

use CommandArgKind as A;
use CommandCategory as C;
use CommandPermission as P;
use CommandRoomScope as S;
use ComposerCommand as K;

const fn spec(
    command: ComposerCommand,
    name: &'static str,
    usage: &'static str,
    description: &'static str,
    category: CommandCategory,
    args: &'static [CommandArgKind],
    permission: CommandPermission,
) -> CommandSpec {
    CommandSpec {
        command,
        name,
        aliases: &[],
        usage,
        description,
        category,
        args,
        destructive: false,
        permission,
        scope: S::Any,
    }
}

const fn aliases(mut spec: CommandSpec, aliases: &'static [&'static str]) -> CommandSpec {
    spec.aliases = aliases;
    spec
}

const fn destructive(mut spec: CommandSpec) -> CommandSpec {
    spec.destructive = true;
    spec
}

const fn scoped(mut spec: CommandSpec, scope: CommandRoomScope) -> CommandSpec {
    spec.scope = scope;
    spec
}

/// The registry, in the order suggestion lists show it.
pub const COMMANDS: &[CommandSpec] = &[
    spec(
        K::Me,
        "me",
        "/me <action>",
        "Send an action message",
        C::Messages,
        &[A::Text],
        P::SendMessage,
    ),
    spec(
        K::Notice,
        "notice",
        "/notice <message>",
        "Send a notice message",
        C::Messages,
        &[A::Text],
        P::SendMessage,
    ),
    spec(
        K::Shrug,
        "shrug",
        "/shrug [message]",
        "Send ¯\\_(ツ)_/¯ with a message",
        C::Messages,
        &[A::Text],
        P::SendMessage,
    ),
    spec(
        K::TableFlip,
        "tableflip",
        "/tableflip [message]",
        "Send (╯°□°)╯︵ ┻━┻ with a message",
        C::Messages,
        &[A::Text],
        P::SendMessage,
    ),
    spec(
        K::Unflip,
        "unflip",
        "/unflip [message]",
        "Send ┬─┬ノ( º_ºノ) with a message",
        C::Messages,
        &[A::Text],
        P::SendMessage,
    ),
    spec(
        K::Lenny,
        "lenny",
        "/lenny [message]",
        "Send ( ͡° ͜ʖ ͡°) with a message",
        C::Messages,
        &[A::Text],
        P::SendMessage,
    ),
    spec(
        K::Spoiler,
        "spoiler",
        "/spoiler <message>",
        "Send a message hidden as a spoiler",
        C::Messages,
        &[A::Text],
        P::SendMessage,
    ),
    spec(
        K::Plain,
        "plain",
        "/plain <message>",
        "Send a message without formatting",
        C::Messages,
        &[A::Text],
        P::SendMessage,
    ),
    spec(
        K::Html,
        "html",
        "/html <html>",
        "Send a message written in HTML",
        C::Messages,
        &[A::Text],
        P::SendMessage,
    ),
    spec(
        K::Poll,
        "poll",
        "/poll <question> | <option> | <option> [| max=2]",
        "Create a poll",
        C::Messages,
        &[A::Poll],
        P::SendMessage,
    ),
    spec(
        K::Gif,
        "gif",
        "/gif [search]",
        "Search for a GIF to send",
        C::Messages,
        &[A::Text],
        P::SendMessage,
    ),
    spec(
        K::StartDm,
        "startdm",
        "/startdm <@user:server> [more people]",
        "Start a direct message",
        C::Navigation,
        &[A::Users],
        P::None,
    ),
    spec(
        K::Msg,
        "msg",
        "/msg <user> <message>",
        "Send someone a direct message",
        C::Navigation,
        &[A::User, A::Text],
        P::None,
    ),
    spec(
        K::Query,
        "query",
        "/query <user>",
        "Open your direct message with someone",
        C::Navigation,
        &[A::User],
        P::None,
    ),
    aliases(
        spec(
            K::Join,
            "join",
            "/join <#room:server>",
            "Join a room by its address",
            C::Navigation,
            &[A::Rooms],
            P::None,
        ),
        &["j", "goto"],
    ),
    aliases(
        spec(
            K::Leave,
            "leave",
            "/leave [!room:server]",
            "Leave this room",
            C::Navigation,
            &[A::Rooms],
            P::None,
        ),
        &["part"],
    ),
    spec(
        K::JumpToDate,
        "jumptodate",
        "/jumptodate <YYYY-MM-DD>",
        "Jump to the first message on a date",
        C::Navigation,
        &[A::Date],
        P::None,
    ),
    spec(
        K::Whois,
        "whois",
        "/whois <user>",
        "Show someone's profile",
        C::People,
        &[A::User],
        P::None,
    ),
    spec(
        K::Ignore,
        "ignore",
        "/ignore <@user:server> [more people]",
        "Hide everything from someone",
        C::People,
        &[A::Users],
        P::None,
    ),
    spec(
        K::Unignore,
        "unignore",
        "/unignore <@user:server> [more people]",
        "Stop ignoring someone",
        C::People,
        &[A::Users],
        P::None,
    ),
    spec(
        K::Invite,
        "invite",
        "/invite <@user:server> [more people] [-r reason]",
        "Invite people to this room",
        C::Moderation,
        &[A::Users, A::Reason],
        P::Invite,
    ),
    spec(
        K::Disinvite,
        "disinvite",
        "/disinvite <@user:server> [more people] [-r reason]",
        "Withdraw invites to this room",
        C::Moderation,
        &[A::Users, A::Reason],
        P::Kick,
    ),
    aliases(
        spec(
            K::Kick,
            "kick",
            "/kick <@user:server or server> [more] [-r reason]",
            "Remove people from this room",
            C::Moderation,
            &[A::Users, A::Servers, A::Reason],
            P::Kick,
        ),
        &["remove"],
    ),
    destructive(spec(
        K::Ban,
        "ban",
        "/ban <@user:server or server> [more] [-r reason]",
        "Ban people from this room",
        C::Moderation,
        &[A::Users, A::Servers, A::Reason],
        P::Ban,
    )),
    spec(
        K::Unban,
        "unban",
        "/unban <@user:server> [more people]",
        "Lift bans in this room",
        C::Moderation,
        &[A::Users],
        P::Ban,
    ),
    destructive(spec(
        K::Delete,
        "delete",
        "/delete <@user:server or server> -past 1d|2h|5m|30s [-t m.room.message] [-r reason]",
        "Delete recent messages from people",
        C::Moderation,
        &[A::Users, A::Servers, A::Since, A::Reason],
        P::RedactOthers,
    )),
    spec(
        K::Op,
        "op",
        "/op <user> [power level]",
        "Give someone a power level (50 by default)",
        C::Moderation,
        &[A::User, A::PowerLevel],
        P::ChangePowerLevels,
    ),
    spec(
        K::Deop,
        "deop",
        "/deop <user>",
        "Reset someone's power level to the default",
        C::Moderation,
        &[A::User],
        P::ChangePowerLevels,
    ),
    spec(
        K::Topic,
        "topic",
        "/topic <topic>",
        "Set this room's topic",
        C::RoomSettings,
        &[A::Text],
        P::ChangeTopic,
    ),
    spec(
        K::RoomName,
        "roomname",
        "/roomname <name>",
        "Set this room's name",
        C::RoomSettings,
        &[A::Text],
        P::ChangeName,
    ),
    spec(
        K::Acl,
        "acl",
        "/acl [-a server] [-d server] [-ra server] [-rd server]",
        "Change which servers may take part",
        C::RoomSettings,
        &[A::AclFlags],
        P::ServerAcl,
    ),
    destructive(spec(
        K::UpgradeRoom,
        "upgraderoom",
        "/upgraderoom <version>",
        "Upgrade this room to a new room version",
        C::RoomSettings,
        &[A::RoomVersion],
        P::UpgradeRoom,
    )),
    scoped(
        spec(
            K::ConvertToDm,
            "converttodm",
            "/converttodm",
            "Mark this room as a direct message",
            C::RoomSettings,
            &[],
            P::None,
        ),
        S::Rooms,
    ),
    scoped(
        spec(
            K::ConvertToRoom,
            "converttoroom",
            "/converttoroom",
            "Mark this direct message as a room",
            C::RoomSettings,
            &[],
            P::None,
        ),
        S::DirectMessages,
    ),
    spec(
        K::Nick,
        "nick",
        "/nick <name>",
        "Change your display name everywhere",
        C::Profile,
        &[A::Text],
        P::None,
    ),
    spec(
        K::MyRoomNick,
        "myroomnick",
        "/myroomnick <name>",
        "Change your name in this room only",
        C::Profile,
        &[A::Text],
        P::None,
    ),
    spec(
        K::MyAvatar,
        "myavatar",
        "/myavatar <mxc://server/id>",
        "Change your profile picture everywhere",
        C::Profile,
        &[A::MxcUri],
        P::None,
    ),
    spec(
        K::MyRoomAvatar,
        "myroomavatar",
        "/myroomavatar <mxc://server/id>",
        "Change your profile picture in this room only",
        C::Profile,
        &[A::MxcUri],
        P::None,
    ),
    spec(
        K::Help,
        "help",
        "/help [command]",
        "Show the commands you can use here",
        C::Other,
        &[A::Command],
        P::None,
    ),
];

/// The registry entry for `command`.
pub fn command_spec(command: ComposerCommand) -> &'static CommandSpec {
    COMMANDS
        .iter()
        .find(|spec| spec.command == command)
        .expect("every command has a registry entry")
}

/// The command `name` (a primary name or an alias) refers to.
pub fn lookup_command(name: &str) -> Option<&'static CommandSpec> {
    COMMANDS
        .iter()
        .find(|spec| spec.name == name || spec.aliases.contains(&name))
}
