//! Running a command, and the outcome each shell renders.
//!
//! Execution goes through [`ComposerOps`], the operations Core already has
//! (kick, join, power levels, …). Core implements it over its owners; tests
//! use a fake, so every rule here is covered without a homeserver.

use std::future::Future;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

use super::parse::{
    is_user_id, parse_composer_input, CommandArgs, ComposerInput, ComposerMessage, ComposerPoll,
};
use super::registry::{command_spec, CommandRoomScope, ComposerCommand};
use super::suggest::{
    command_suggestions, permission_allowed, scope_allowed, CommandContext, CommandSuggestion,
};

/// A member as the commands see it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposerMember {
    pub user_id: String,
    pub display_name: Option<String>,
    /// `join`, `invite`, `leave`, `ban` or `knock`.
    pub membership: String,
}

/// A failed operation: Core's closed diagnostic id.
pub type OpResult<T> = Result<T, String>;

/// What a bulk redaction did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RedactSummary {
    pub redacted: u32,
    pub failed: u32,
    pub truncated: bool,
}

/// The operations commands run through. Errors are Core diagnostic ids.
pub trait ComposerOps: Sync {
    fn own_user_id(&self) -> Option<String>;
    fn now_ms(&self) -> u64;
    fn context(&self, room_id: &str) -> impl Future<Output = OpResult<CommandContext>> + Send;
    fn members(&self, room_id: &str) -> impl Future<Output = OpResult<Vec<ComposerMember>>> + Send;
    fn invite(
        &self,
        room_id: &str,
        user_id: &str,
        reason: Option<String>,
    ) -> impl Future<Output = OpResult<()>> + Send;
    fn kick(
        &self,
        room_id: &str,
        user_id: &str,
        reason: Option<String>,
    ) -> impl Future<Output = OpResult<()>> + Send;
    fn ban(
        &self,
        room_id: &str,
        user_id: &str,
        reason: Option<String>,
    ) -> impl Future<Output = OpResult<()>> + Send;
    fn unban(&self, room_id: &str, user_id: &str) -> impl Future<Output = OpResult<()>> + Send;
    fn join(&self, room_id_or_alias: &str) -> impl Future<Output = OpResult<()>> + Send;
    fn leave(&self, room_id: &str) -> impl Future<Output = OpResult<()>> + Send;
    fn ignore(&self, user_id: &str) -> impl Future<Output = OpResult<()>> + Send;
    fn unignore(&self, user_id: &str) -> impl Future<Output = OpResult<()>> + Send;
    fn set_room_name(&self, room_id: &str, name: &str)
        -> impl Future<Output = OpResult<()>> + Send;
    fn set_room_topic(
        &self,
        room_id: &str,
        topic: &str,
    ) -> impl Future<Output = OpResult<()>> + Send;
    fn set_display_name(&self, name: &str) -> impl Future<Output = OpResult<()>> + Send;
    fn set_avatar(&self, mxc: &str) -> impl Future<Output = OpResult<()>> + Send;
    /// The content of a state event, or `None` when the room has none.
    fn state_content(
        &self,
        room_id: &str,
        event_type: &str,
        state_key: &str,
    ) -> impl Future<Output = OpResult<Option<JsonValue>>> + Send;
    fn send_state(
        &self,
        room_id: &str,
        event_type: &str,
        state_key: &str,
        content: JsonValue,
    ) -> impl Future<Output = OpResult<()>> + Send;
    fn set_power_level(
        &self,
        room_id: &str,
        user_id: &str,
        level: i64,
    ) -> impl Future<Output = OpResult<()>> + Send;
    /// The joined direct message with exactly `user_id`, if there is one.
    fn dm_room(&self, user_id: &str) -> impl Future<Output = OpResult<Option<String>>> + Send;
    fn create_dm(&self, user_ids: &[String]) -> impl Future<Output = OpResult<String>> + Send;
    fn mark_dm(&self, room_id: &str, user_id: &str) -> impl Future<Output = OpResult<()>> + Send;
    fn unmark_dm(&self, room_id: &str) -> impl Future<Output = OpResult<()>> + Send;
    fn send_text(&self, room_id: &str, body: &str) -> impl Future<Output = OpResult<()>> + Send;
    fn bulk_redact(
        &self,
        room_id: &str,
        user_ids: &[String],
        since_ts: u64,
        event_types: &[String],
        reason: Option<String>,
    ) -> impl Future<Output = OpResult<RedactSummary>> + Send;
    /// Upgrades the room; returns the replacement room's id.
    fn upgrade_room(
        &self,
        room_id: &str,
        version: &str,
    ) -> impl Future<Output = OpResult<String>> + Send;
    /// The first event at or after `timestamp_ms`.
    fn event_at(
        &self,
        room_id: &str,
        timestamp_ms: u64,
    ) -> impl Future<Output = OpResult<Option<String>>> + Send;
}

/// Why a command did not run. Shells pick an icon or tone from it; the copy
/// is in [`ComposerCommandError::message`].
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Enum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComposerErrorKind {
    /// The arguments are missing or wrong.
    Usage,
    UnknownCommand,
    /// The user may not do this here.
    Permission,
    /// A person or room the command names could not be found.
    NotFound,
    /// Some people or rooms were done, others not.
    Partial,
    /// The homeserver or Core refused or failed.
    Failed,
    NotSignedIn,
}

/// A command that did not run, with user copy.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposerCommandError {
    pub kind: ComposerErrorKind,
    /// What went wrong and what to do next.
    pub message: String,
    /// Core's diagnostic id of the first failure, when there was one.
    pub diagnostic_id: Option<String>,
}

impl ComposerCommandError {
    fn new(kind: ComposerErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            diagnostic_id: None,
        }
    }
}

/// What the shell should do after `run_command`.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Enum))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CommandOutcome {
    /// The command ran; `message` is a short confirmation.
    Done {
        message: String,
    },
    /// A destructive command: ask, then run again with `confirmed`.
    NeedsConfirmation {
        title: String,
        message: String,
        confirm_label: String,
    },
    /// Send this message in the room (with the shell's reply or thread).
    SendMessage {
        message: ComposerMessage,
    },
    /// Send this poll in the room.
    SendPoll {
        poll: ComposerPoll,
    },
    /// Open the GIF picker searching for `query`.
    OpenGifPicker {
        query: String,
    },
    /// Show this room, at `event_id` when set.
    Navigate {
        room_id: String,
        event_id: Option<String>,
    },
    OpenProfile {
        user_id: String,
    },
    /// List these commands (the ones the user may use here).
    Help {
        commands: Vec<CommandSuggestion>,
    },
}

pub type CommandResult = Result<CommandOutcome, ComposerCommandError>;

/// [`CommandResult`] on the desktop wire: exactly one field is set.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposerRunResponse {
    pub outcome: Option<CommandOutcome>,
    pub error: Option<ComposerCommandError>,
}

impl From<CommandResult> for ComposerRunResponse {
    fn from(result: CommandResult) -> Self {
        match result {
            Ok(outcome) => Self {
                outcome: Some(outcome),
                error: None,
            },
            Err(error) => Self {
                outcome: None,
                error: Some(error),
            },
        }
    }
}

// ---- user copy -------------------------------------------------------------

const NOT_SIGNED_IN: &str =
    "Synara is not signed in to your Matrix account. Sign in again, then send your message.";
const MEMBERSHIP_UNREADABLE: &str =
    "Synara could not read your membership in this room yet. Try again in a moment.";
const MEMBERSHIP_WRITE_FAILED: &str = "Synara could not update your room profile. Try again.";

/// Copy for Core diagnostics by their shared endings, as desktop maps them.
fn diagnostic_copy(diagnostic: &str) -> Option<&'static str> {
    const SUFFIXES: &[(&str, &str)] = &[
        ("-no-session", NOT_SIGNED_IN),
        ("-requires-session", NOT_SIGNED_IN),
        (
            "-rate-limited",
            "Your homeserver is limiting requests. Wait a moment and try again.",
        ),
        (
            "-network-failed",
            "Synara could not reach your homeserver. Check your connection and try again.",
        ),
        (
            "-forbidden",
            "You do not have permission to do that in this room.",
        ),
        ("-too-large", "That is larger than your homeserver allows."),
        ("-not-found", "Matrix could not find that room or user."),
    ];
    SUFFIXES
        .iter()
        .find(|(suffix, _)| diagnostic.ends_with(suffix))
        .map(|(_, copy)| *copy)
}

fn is_no_session(diagnostic: &str) -> bool {
    diagnostic.ends_with("-no-session") || diagnostic.ends_with("-requires-session")
}

/// "/invite did not finish: …", keeping the diagnostic for reports.
fn op_failed(command: ComposerCommand, diagnostic: String) -> ComposerCommandError {
    let name = command_spec(command).name;
    let kind = if is_no_session(&diagnostic) {
        ComposerErrorKind::NotSignedIn
    } else {
        ComposerErrorKind::Failed
    };
    let message = match diagnostic_copy(&diagnostic) {
        Some(copy) if kind == ComposerErrorKind::NotSignedIn => copy.to_owned(),
        Some(copy) => format!("/{name} did not finish: {copy}"),
        None => format!(
            "/{name} did not finish: Synara could not run the command. Try again. (Error code: {diagnostic})"
        ),
    };
    ComposerCommandError {
        kind,
        message,
        diagnostic_id: Some(diagnostic),
    }
}

/// "A", "A and B", "A, B and C", "A, B, C and 2 more".
fn list_labels(labels: &[String]) -> String {
    const MAX: usize = 3;
    match labels.len() {
        0 => String::new(),
        1 => labels[0].clone(),
        n if n <= MAX => format!("{} and {}", labels[..n - 1].join(", "), labels[n - 1]),
        n => format!("{} and {} more", labels[..MAX].join(", "), n - MAX),
    }
}

/// Copy for a per-person command that finished for each person.
struct BulkCopy {
    /// Past tense, e.g. "Invited".
    done: &'static str,
    /// Base verb, e.g. "invite".
    verb: &'static str,
}

/// Run `action` for each item in order; a failure does not stop the rest.
async fn run_each<'a, F, Fut>(items: &'a [String], mut action: F) -> Vec<(String, String)>
where
    F: FnMut(&'a str) -> Fut,
    Fut: Future<Output = OpResult<()>>,
{
    let mut failures = Vec::new();
    for item in items {
        // Sequential on purpose: the server rate-limits bursts.
        if let Err(diagnostic) = action(item).await {
            failures.push((item.clone(), diagnostic));
        }
    }
    failures
}

/// The outcome of a per-person command: a confirmation, or the bulk error
/// desktop showed ("Invited 2 of 3 people. Couldn't invite @c. Try again.").
fn bulk_outcome(
    command: ComposerCommand,
    items: &[String],
    failures: Vec<(String, String)>,
    copy: BulkCopy,
) -> CommandResult {
    let total = items.len();
    if failures.is_empty() {
        let who = if total == 1 {
            items[0].clone()
        } else {
            format!("{total} people")
        };
        return Ok(CommandOutcome::Done {
            message: format!("{} {who}.", copy.done),
        });
    }
    let first = failures[0].1.clone();
    if failures.len() == total && is_no_session(&first) {
        return Err(op_failed(command, first));
    }
    let failed: Vec<String> = failures.into_iter().map(|(item, _)| item).collect();
    let succeeded = total - failed.len();
    let summary = if succeeded == 0 {
        format!(
            "Couldn't {} {}. Try again.",
            copy.verb,
            list_labels(&failed)
        )
    } else {
        format!(
            "{} {succeeded} of {total} people. Couldn't {} {}. Try again.",
            copy.done,
            copy.verb,
            list_labels(&failed)
        )
    };
    let message = match diagnostic_copy(&first) {
        Some(reason) => format!("{summary} {reason}"),
        None => summary,
    };
    Err(ComposerCommandError {
        kind: if succeeded == 0 {
            ComposerErrorKind::Failed
        } else {
            ComposerErrorKind::Partial
        },
        message,
        diagnostic_id: Some(first),
    })
}

// ---- running ---------------------------------------------------------------

/// Run what the composer holds in `room_id`. Message commands come back as
/// [`CommandOutcome::SendMessage`] so the shell sends them with its reply
/// and thread; destructive commands need `confirmed`.
pub async fn run_command<O: ComposerOps>(
    ops: &O,
    room_id: &str,
    text: &str,
    confirmed: bool,
) -> CommandResult {
    let (command, args) = match parse_composer_input(text) {
        ComposerInput::Message { message, command } => {
            if let Some(command) = command {
                check_allowed(ops, room_id, command).await?;
            }
            return Ok(CommandOutcome::SendMessage { message });
        }
        ComposerInput::Unknown { name } => {
            return Err(ComposerCommandError::new(
                ComposerErrorKind::UnknownCommand,
                format!("Synara does not know the command /{name}."),
            ));
        }
        ComposerInput::Invalid { message, .. } => {
            return Err(ComposerCommandError::new(ComposerErrorKind::Usage, message));
        }
        ComposerInput::Command { command, args } => (command, args),
    };
    let context = check_allowed(ops, room_id, command).await?;
    run_parsed(ops, room_id, command, args, confirmed, context).await
}

/// Refuses a command the user may not use here. Unknown power levels let
/// it through; the server decides.
async fn check_allowed<O: ComposerOps>(
    ops: &O,
    room_id: &str,
    command: ComposerCommand,
) -> Result<CommandContext, ComposerCommandError> {
    let context = ops.context(room_id).await.unwrap_or_default();
    let spec = command_spec(command);
    if !permission_allowed(spec.permission, &context) {
        return Err(ComposerCommandError::new(
            ComposerErrorKind::Permission,
            format!(
                "You do not have permission to use /{} in this room.",
                spec.name
            ),
        ));
    }
    if !scope_allowed(spec.scope, &context) {
        let message = match spec.scope {
            CommandRoomScope::DirectMessages => {
                format!("/{} works in a direct message.", spec.name)
            }
            _ => format!(
                "/{} works in a room that is not a direct message.",
                spec.name
            ),
        };
        return Err(ComposerCommandError::new(ComposerErrorKind::Usage, message));
    }
    Ok(context)
}

fn usage(message: &str) -> ComposerCommandError {
    ComposerCommandError::new(ComposerErrorKind::Usage, message)
}

fn confirm(title: String, message: &str, confirm_label: &str) -> CommandResult {
    Ok(CommandOutcome::NeedsConfirmation {
        title,
        message: message.to_owned(),
        confirm_label: confirm_label.to_owned(),
    })
}

/// "@a:b", "@a:b and @c:d", "3 people".
fn people_label(users: &[String]) -> String {
    match users {
        [one] => one.clone(),
        [a, b] => format!("{a} and {b}"),
        _ => format!("{} people", users.len()),
    }
}

/// Users named directly plus the members of the named servers, as desktop
/// did. `skip_banned` leaves out members already banned (for /kick).
async fn expand_servers<O: ComposerOps>(
    ops: &O,
    command: ComposerCommand,
    room_id: &str,
    mut users: Vec<String>,
    servers: &[String],
    skip_banned: bool,
) -> Result<Vec<String>, ComposerCommandError> {
    if servers.is_empty() {
        return Ok(users);
    }
    let members = ops
        .members(room_id)
        .await
        .map_err(|diagnostic| op_failed(command, diagnostic))?;
    let own = ops.own_user_id();
    for member in members {
        let on_server = servers
            .iter()
            .any(|server| member.user_id.ends_with(&format!(":{server}")));
        let skip = (skip_banned && member.membership == "ban")
            || own.as_deref() == Some(member.user_id.as_str());
        if on_server && !skip && !users.contains(&member.user_id) {
            users.push(member.user_id);
        }
    }
    Ok(users)
}

/// A person named by Matrix ID, or by a member's name, display name or
/// localpart when exactly one member matches.
async fn resolve_user<O: ComposerOps>(
    ops: &O,
    command: ComposerCommand,
    room_id: &str,
    input: &str,
) -> Result<String, ComposerCommandError> {
    if is_user_id(input) {
        return Ok(input.to_owned());
    }
    let wanted = input.trim_start_matches('@').to_lowercase();
    let members = ops
        .members(room_id)
        .await
        .map_err(|diagnostic| op_failed(command, diagnostic))?;
    let matches: Vec<&ComposerMember> = members
        .iter()
        .filter(|member| member.membership == "join" || member.membership == "invite")
        .filter(|member| {
            let name = member.display_name.as_deref().unwrap_or_default().trim();
            let localpart = member
                .user_id
                .trim_start_matches('@')
                .split(':')
                .next()
                .unwrap_or_default();
            name.to_lowercase() == wanted || localpart.to_lowercase() == wanted
        })
        .collect();
    match matches.as_slice() {
        [member] => Ok(member.user_id.clone()),
        [] => Err(ComposerCommandError::new(
            ComposerErrorKind::NotFound,
            format!("No one in this room is called {input}. Use their Matrix ID, like @name:server."),
        )),
        _ => Err(ComposerCommandError::new(
            ComposerErrorKind::NotFound,
            format!("More than one person here is called {input}. Use their Matrix ID, like @name:server."),
        )),
    }
}

/// The joined direct message with `user_id`, created when there is none.
async fn dm_with<O: ComposerOps>(
    ops: &O,
    command: ComposerCommand,
    user_id: &str,
) -> Result<String, ComposerCommandError> {
    let fail = |diagnostic| op_failed(command, diagnostic);
    if let Some(room_id) = ops.dm_room(user_id).await.map_err(fail)? {
        return Ok(room_id);
    }
    let users = vec![user_id.to_owned()];
    let room_id = ops.create_dm(&users).await.map_err(fail)?;
    ops.mark_dm(&room_id, user_id).await.map_err(fail)?;
    Ok(room_id)
}

async fn run_parsed<O: ComposerOps>(
    ops: &O,
    room_id: &str,
    command: ComposerCommand,
    args: CommandArgs,
    confirmed: bool,
    context: CommandContext,
) -> CommandResult {
    use ComposerCommand as K;
    let fail = |diagnostic| op_failed(command, diagnostic);
    match (command, args) {
        (K::Poll, CommandArgs::Poll(poll)) => Ok(CommandOutcome::SendPoll { poll }),
        (K::Gif, CommandArgs::Text(query)) => Ok(CommandOutcome::OpenGifPicker { query }),
        (K::Help, CommandArgs::Text(topic)) => {
            let commands = command_suggestions("", &context)
                .into_iter()
                .filter(|suggestion| {
                    topic.is_empty()
                        || suggestion.name == topic.trim_start_matches('/')
                        || suggestion
                            .aliases
                            .iter()
                            .any(|alias| alias == topic.trim_start_matches('/'))
                })
                .collect::<Vec<_>>();
            if commands.is_empty() {
                return Err(usage(&format!(
                    "There is no /{} you can use here. Type /help to list the commands.",
                    topic.trim_start_matches('/')
                )));
            }
            Ok(CommandOutcome::Help { commands })
        }
        (K::StartDm, CommandArgs::People { users, .. }) => {
            let own = ops.own_user_id();
            let users: Vec<String> = users
                .into_iter()
                .filter(|user| own.as_deref() != Some(user.as_str()))
                .collect();
            match users.as_slice() {
                [] => Err(usage(
                    "Add the Matrix ID of the person to message, like /startdm @name:server.",
                )),
                [user] => Ok(CommandOutcome::Navigate {
                    room_id: dm_with(ops, command, user).await?,
                    event_id: None,
                }),
                _ => {
                    let room_id = ops.create_dm(&users).await.map_err(fail)?;
                    ops.mark_dm(&room_id, &users[0]).await.map_err(fail)?;
                    Ok(CommandOutcome::Navigate {
                        room_id,
                        event_id: None,
                    })
                }
            }
        }
        (K::Query, CommandArgs::User { user, .. }) => {
            let user = resolve_user(ops, command, room_id, &user).await?;
            Ok(CommandOutcome::Navigate {
                room_id: dm_with(ops, command, &user).await?,
                event_id: None,
            })
        }
        (K::Msg, CommandArgs::User { user, text }) => {
            let user = resolve_user(ops, command, room_id, &user).await?;
            let dm = dm_with(ops, command, &user).await?;
            ops.send_text(&dm, &text).await.map_err(fail)?;
            Ok(CommandOutcome::Navigate {
                room_id: dm,
                event_id: None,
            })
        }
        (K::Whois, CommandArgs::User { user, .. }) => Ok(CommandOutcome::OpenProfile {
            user_id: resolve_user(ops, command, room_id, &user).await?,
        }),
        (K::Join, CommandArgs::Rooms(rooms)) => {
            let failures = run_each(&rooms, |target| ops.join(target)).await;
            if let ([room], true) = (rooms.as_slice(), failures.is_empty()) {
                if room.starts_with('!') {
                    return Ok(CommandOutcome::Navigate {
                        room_id: room.clone(),
                        event_id: None,
                    });
                }
            }
            rooms_outcome(command, &rooms, failures, "Joined", "join")
        }
        (K::Leave, CommandArgs::Rooms(rooms)) => {
            let rooms = if rooms.is_empty() {
                vec![room_id.to_owned()]
            } else {
                rooms
            };
            let failures = run_each(&rooms, |target| ops.leave(target)).await;
            if failures.is_empty() && rooms.len() == 1 && rooms[0] == room_id {
                return Ok(CommandOutcome::Done {
                    message: "You left the room.".to_owned(),
                });
            }
            rooms_outcome(command, &rooms, failures, "Left", "leave")
        }
        (K::Invite, CommandArgs::People { users, reason, .. }) => {
            let failures = run_each(&users, |user| ops.invite(room_id, user, reason.clone())).await;
            bulk_outcome(
                command,
                &users,
                failures,
                BulkCopy {
                    done: "Invited",
                    verb: "invite",
                },
            )
        }
        (K::Disinvite, CommandArgs::People { users, reason, .. }) => {
            let failures = run_each(&users, |user| ops.kick(room_id, user, reason.clone())).await;
            bulk_outcome(
                command,
                &users,
                failures,
                BulkCopy {
                    done: "Withdrew invites for",
                    verb: "withdraw the invite for",
                },
            )
        }
        (
            K::Kick,
            CommandArgs::People {
                users,
                servers,
                reason,
            },
        ) => {
            let users = expand_servers(ops, command, room_id, users, &servers, true).await?;
            if users.is_empty() {
                return Err(usage(
                    "Add the Matrix ID of each person to remove, like /kick @name:server.",
                ));
            }
            let failures = run_each(&users, |user| ops.kick(room_id, user, reason.clone())).await;
            bulk_outcome(
                command,
                &users,
                failures,
                BulkCopy {
                    done: "Removed",
                    verb: "remove",
                },
            )
        }
        (
            K::Ban,
            CommandArgs::People {
                users,
                servers,
                reason,
            },
        ) => {
            let users = expand_servers(ops, command, room_id, users, &servers, false).await?;
            if users.is_empty() {
                return Err(usage(
                    "Add the Matrix ID of each person to ban, like /ban @name:server.",
                ));
            }
            if !confirmed {
                return confirm(
                    format!("Ban {}?", people_label(&users)),
                    "They will be removed from this room and can't come back until someone lifts the ban.",
                    "Ban",
                );
            }
            let failures = run_each(&users, |user| ops.ban(room_id, user, reason.clone())).await;
            bulk_outcome(
                command,
                &users,
                failures,
                BulkCopy {
                    done: "Banned",
                    verb: "ban",
                },
            )
        }
        (K::Unban, CommandArgs::People { users, .. }) => {
            let failures = run_each(&users, |user| ops.unban(room_id, user)).await;
            bulk_outcome(
                command,
                &users,
                failures,
                BulkCopy {
                    done: "Unbanned",
                    verb: "unban",
                },
            )
        }
        (K::Ignore, CommandArgs::People { users, .. }) => {
            let failures = run_each(&users, |user| ops.ignore(user)).await;
            bulk_outcome(
                command,
                &users,
                failures,
                BulkCopy {
                    done: "Ignored",
                    verb: "ignore",
                },
            )
        }
        (K::Unignore, CommandArgs::People { users, .. }) => {
            let failures = run_each(&users, |user| ops.unignore(user)).await;
            bulk_outcome(
                command,
                &users,
                failures,
                BulkCopy {
                    done: "Stopped ignoring",
                    verb: "unignore",
                },
            )
        }
        (
            K::Delete,
            CommandArgs::Delete {
                users,
                servers,
                past_ms,
                event_types,
                reason,
            },
        ) => {
            let users = expand_servers(ops, command, room_id, users, &servers, false).await?;
            if users.is_empty() {
                return Err(usage(
                    "Add the Matrix ID or server of the people whose messages to delete.",
                ));
            }
            if !confirmed {
                return confirm(
                    format!("Delete messages from {}?", people_label(&users)),
                    "Their messages from this period will be removed for everyone. This can't be undone.",
                    "Delete",
                );
            }
            let since = ops.now_ms().saturating_sub(past_ms);
            let summary = ops
                .bulk_redact(room_id, &users, since, &event_types, reason)
                .await
                .map_err(fail)?;
            let mut message = match summary.redacted {
                1 => "Deleted 1 message.".to_owned(),
                n => format!("Deleted {n} messages."),
            };
            if summary.failed > 0 {
                message.push_str(&format!(" {} could not be deleted.", summary.failed));
            }
            if summary.truncated {
                message.push_str(
                    " Older messages were not searched; run the command again to continue.",
                );
            }
            Ok(CommandOutcome::Done { message })
        }
        (K::Op | K::Deop, CommandArgs::PowerLevel { user, level }) => {
            let user = resolve_user(ops, command, room_id, &user).await?;
            let level = match level {
                Some(level) => level,
                None => ops
                    .state_content(room_id, "m.room.power_levels", "")
                    .await
                    .map_err(fail)?
                    .and_then(|content| content.get("users_default").and_then(JsonValue::as_i64))
                    .unwrap_or(0),
            };
            ops.set_power_level(room_id, &user, level)
                .await
                .map_err(fail)?;
            Ok(CommandOutcome::Done {
                message: format!("Set {user}'s power level to {level}."),
            })
        }
        (K::Topic, CommandArgs::Text(topic)) => {
            ops.set_room_topic(room_id, &topic).await.map_err(fail)?;
            done("Changed the topic.")
        }
        (K::RoomName, CommandArgs::Text(name)) => {
            ops.set_room_name(room_id, &name).await.map_err(fail)?;
            done("Renamed the room.")
        }
        (K::Nick, CommandArgs::Text(name)) => {
            ops.set_display_name(&name).await.map_err(fail)?;
            Ok(CommandOutcome::Done {
                message: format!("Your display name is now {name}."),
            })
        }
        (K::MyAvatar, CommandArgs::Mxc(mxc)) => {
            ops.set_avatar(&mxc).await.map_err(fail)?;
            done("Changed your profile picture.")
        }
        (K::MyRoomNick, CommandArgs::Text(name)) => {
            set_own_member_field(ops, command, room_id, "displayname", &name).await?;
            done("Changed your name in this room.")
        }
        (K::MyRoomAvatar, CommandArgs::Mxc(mxc)) => {
            set_own_member_field(ops, command, room_id, "avatar_url", &mxc).await?;
            done("Changed your profile picture in this room.")
        }
        (
            K::Acl,
            CommandArgs::Acl {
                allow,
                deny,
                remove_allow,
                remove_deny,
            },
        ) => {
            let current = ops
                .state_content(room_id, "m.room.server_acl", "")
                .await
                .map_err(fail)?;
            let content = server_acl_content(current, &allow, &deny, &remove_allow, &remove_deny);
            ops.send_state(room_id, "m.room.server_acl", "", content)
                .await
                .map_err(fail)?;
            done("Updated the server access list.")
        }
        (K::UpgradeRoom, CommandArgs::RoomVersion(version)) => {
            if !confirmed {
                return confirm(
                    format!("Upgrade this room to version {version}?"),
                    "Synara creates a new room and closes this one. Members have to join the new room, and this can't be undone.",
                    "Upgrade",
                );
            }
            let replacement = ops.upgrade_room(room_id, &version).await.map_err(fail)?;
            Ok(CommandOutcome::Navigate {
                room_id: replacement,
                event_id: None,
            })
        }
        (K::JumpToDate, CommandArgs::Date { timestamp_ms }) => {
            match ops.event_at(room_id, timestamp_ms).await.map_err(fail)? {
                Some(event_id) => Ok(CommandOutcome::Navigate {
                    room_id: room_id.to_owned(),
                    event_id: Some(event_id),
                }),
                None => Err(ComposerCommandError::new(
                    ComposerErrorKind::NotFound,
                    "There are no messages in this room from that date on.",
                )),
            }
        }
        (K::ConvertToDm, CommandArgs::None) => {
            let own = ops.own_user_id();
            let members = ops.members(room_id).await.map_err(fail)?;
            let others: Vec<&ComposerMember> = members
                .iter()
                .filter(|member| {
                    member.membership == "join" && own.as_deref() != Some(member.user_id.as_str())
                })
                .collect();
            let [other] = others.as_slice() else {
                return Err(usage(
                    "Synara could not tell who this direct message is with. /converttodm works in a room with one other member.",
                ));
            };
            ops.mark_dm(room_id, &other.user_id).await.map_err(fail)?;
            done("This room is now a direct message.")
        }
        (K::ConvertToRoom, CommandArgs::None) => {
            ops.unmark_dm(room_id).await.map_err(fail)?;
            done("This direct message is now a room.")
        }
        // The parser pairs every command with its argument shape.
        (command, _) => Err(usage(&format!(
            "/{} could not read its arguments.",
            command_spec(command).name
        ))),
    }
}

fn done(message: &str) -> CommandResult {
    Ok(CommandOutcome::Done {
        message: message.to_owned(),
    })
}

fn rooms_outcome(
    command: ComposerCommand,
    rooms: &[String],
    failures: Vec<(String, String)>,
    done_word: &str,
    verb: &str,
) -> CommandResult {
    if failures.is_empty() {
        let what = match rooms {
            [room] => room.clone(),
            _ => format!("{} rooms", rooms.len()),
        };
        return Ok(CommandOutcome::Done {
            message: format!("{done_word} {what}."),
        });
    }
    if failures.len() == rooms.len() && rooms.len() == 1 {
        return Err(op_failed(command, failures[0].1.clone()));
    }
    let failed: Vec<String> = failures.iter().map(|(room, _)| room.clone()).collect();
    let mut message = format!("Couldn't {verb} {}. Try again.", list_labels(&failed));
    if let Some(reason) = diagnostic_copy(&failures[0].1) {
        message.push(' ');
        message.push_str(reason);
    }
    Err(ComposerCommandError {
        kind: if failures.len() == rooms.len() {
            ComposerErrorKind::Failed
        } else {
            ComposerErrorKind::Partial
        },
        message,
        diagnostic_id: Some(failures[0].1.clone()),
    })
}

/// Rewrites one field of the user's own `m.room.member` event, keeping the
/// rest (membership, the other profile field) as it is.
async fn set_own_member_field<O: ComposerOps>(
    ops: &O,
    command: ComposerCommand,
    room_id: &str,
    field: &str,
    value: &str,
) -> Result<(), ComposerCommandError> {
    let Some(own) = ops.own_user_id() else {
        return Err(ComposerCommandError::new(
            ComposerErrorKind::NotSignedIn,
            NOT_SIGNED_IN,
        ));
    };
    let content = ops.state_content(room_id, "m.room.member", &own).await;
    let Ok(Some(JsonValue::Object(mut content))) = content else {
        return Err(ComposerCommandError::new(
            ComposerErrorKind::Failed,
            MEMBERSHIP_UNREADABLE,
        ));
    };
    content.insert(field.to_owned(), JsonValue::String(value.to_owned()));
    ops.send_state(room_id, "m.room.member", &own, JsonValue::Object(content))
        .await
        .map_err(|diagnostic| {
            let mut error = op_failed(command, diagnostic);
            if error.kind == ComposerErrorKind::Failed {
                error.message = MEMBERSHIP_WRITE_FAILED.to_owned();
            }
            error
        })
}

/// The new `m.room.server_acl` content. A room without an ACL allows every
/// server, so it starts from `allow: ["*"]` rather than an empty list that
/// would shut everyone out.
pub fn server_acl_content(
    current: Option<JsonValue>,
    allow: &[String],
    deny: &[String],
    remove_allow: &[String],
    remove_deny: &[String],
) -> JsonValue {
    let list = |content: &Option<JsonValue>, key: &str| -> Option<Vec<String>> {
        content.as_ref()?.get(key)?.as_array().map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(str::to_owned))
                .collect()
        })
    };
    let mut allowed = list(&current, "allow").unwrap_or_else(|| vec!["*".to_owned()]);
    let mut denied = list(&current, "deny").unwrap_or_default();
    for server in allow {
        if !allowed.contains(server) {
            allowed.push(server.clone());
        }
    }
    for server in deny {
        if !denied.contains(server) {
            denied.push(server.clone());
        }
    }
    allowed.retain(|server| !remove_allow.contains(server));
    denied.retain(|server| !remove_deny.contains(server));
    allowed.sort();
    denied.sort();
    let mut content = json!({ "allow": allowed, "deny": denied });
    if let Some(literals) = current
        .as_ref()
        .and_then(|content| content.get("allow_ip_literals"))
        .and_then(JsonValue::as_bool)
    {
        content["allow_ip_literals"] = JsonValue::Bool(literals);
    }
    content
}
