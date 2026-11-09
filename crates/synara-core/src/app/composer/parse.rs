//! Composer text to a message or a typed command.
//!
//! Argument rules follow the desktop parser this replaces (`parseUsers`,
//! `parseServers`, `splitPayloadContentAndFlags`, `parseFlags`), so text that
//! worked before still does.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::registry::{lookup_command, ComposerCommand, LENNY, SHRUG, TABLEFLIP, UNFLIP};

/// The `msgtype` a message is sent with.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Enum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComposerMsgType {
    Text,
    Emote,
    Notice,
}

impl ComposerMsgType {
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::Text => "m.text",
            Self::Emote => "m.emote",
            Self::Notice => "m.notice",
        }
    }
}

/// A message the composer should send, after any message command.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Record))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposerMessage {
    pub msgtype: ComposerMsgType,
    /// The user's text without the command, for the shell to format.
    pub text: String,
    /// Text that goes before `text` unformatted, such as ¯\_(ツ)_/¯.
    pub prefix: Option<String>,
    /// Plain body to send: `prefix` and `text` joined by a space.
    pub body: String,
    /// HTML Core built (/spoiler, /html). The shell sends it as is.
    pub formatted_body: Option<String>,
    /// False when the shell must not turn Markdown in `text` into HTML
    /// (/plain, /spoiler, /html).
    pub allow_formatting: bool,
}

impl ComposerMessage {
    fn new(msgtype: ComposerMsgType, text: &str, prefix: Option<&str>) -> Self {
        let body = match prefix {
            Some(prefix) if text.is_empty() => prefix.to_owned(),
            Some(prefix) => format!("{prefix} {text}"),
            None => text.to_owned(),
        };
        Self {
            msgtype,
            text: text.to_owned(),
            prefix: prefix.map(str::to_owned),
            body,
            formatted_body: None,
            allow_formatting: true,
        }
    }
}

/// A parsed poll, with the desktop limits applied.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Record))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposerPoll {
    pub question: String,
    pub answers: Vec<String>,
    pub max_selections: u32,
}

/// Arguments of a known command, typed by what the command takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandArgs {
    None,
    Text(String),
    /// User IDs, server names and `-r reason`, for the people commands.
    People {
        users: Vec<String>,
        servers: Vec<String>,
        reason: Option<String>,
    },
    Rooms(Vec<String>),
    /// One person (a Matrix ID or a member's name) and optional text.
    User {
        user: String,
        text: String,
    },
    PowerLevel {
        user: String,
        level: Option<i64>,
    },
    Date {
        timestamp_ms: u64,
    },
    Mxc(String),
    RoomVersion(String),
    Delete {
        users: Vec<String>,
        servers: Vec<String>,
        past_ms: u64,
        event_types: Vec<String>,
        reason: Option<String>,
    },
    Acl {
        allow: Vec<String>,
        deny: Vec<String>,
        remove_allow: Vec<String>,
        remove_deny: Vec<String>,
    },
    Poll(ComposerPoll),
}

/// What the composer holds, once parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComposerInput {
    /// Send a message. `command` is the message command that shaped it.
    Message {
        command: Option<ComposerCommand>,
        message: ComposerMessage,
    },
    Command {
        command: ComposerCommand,
        args: CommandArgs,
    },
    /// A known command whose arguments are missing or wrong; `message` is
    /// user copy saying what to type.
    Invalid {
        command: ComposerCommand,
        message: String,
    },
    Unknown {
        name: String,
    },
}

/// The command name `text` starts with: `/name` followed by a space or the
/// end. `/usr/bin` and `//escaped` are not commands.
pub fn typed_command_name(text: &str) -> Option<&str> {
    let rest = text.strip_prefix('/')?;
    let end = rest
        .char_indices()
        .find(|(_, c)| !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_' || *c == '-'))
        .map_or(rest.len(), |(index, _)| index);
    let name = &rest[..end];
    let first_ok = name.chars().next().is_some_and(|c| c.is_ascii_lowercase());
    let boundary_ok = rest[end..].chars().next().is_none_or(char::is_whitespace);
    (first_ok && boundary_ok).then_some(name)
}

/// Parse what the composer holds. Leading and trailing space is ignored.
pub fn parse_composer_input(text: &str) -> ComposerInput {
    let text = text.trim();
    // "//text" sends "/text": the escape for a message that starts with a slash.
    if let Some(rest) = text.strip_prefix("//") {
        return plain_message(&format!("/{rest}"));
    }
    let Some(name) = typed_command_name(text) else {
        return plain_message(text);
    };
    let payload = text[1 + name.len()..].trim();
    let Some(spec) = lookup_command(name) else {
        return ComposerInput::Unknown {
            name: name.to_owned(),
        };
    };
    parse_command(spec.command, payload)
}

fn plain_message(text: &str) -> ComposerInput {
    ComposerInput::Message {
        command: None,
        message: ComposerMessage::new(ComposerMsgType::Text, text, None),
    }
}

fn invalid(command: ComposerCommand, message: &str) -> ComposerInput {
    ComposerInput::Invalid {
        command,
        message: message.to_owned(),
    }
}

fn parse_command(command: ComposerCommand, payload: &str) -> ComposerInput {
    use ComposerCommand as K;
    let message = |msgtype, prefix: Option<&str>| ComposerInput::Message {
        command: Some(command),
        message: ComposerMessage::new(msgtype, payload, prefix),
    };
    let args = |args| ComposerInput::Command { command, args };
    match command {
        K::Me if payload.is_empty() => invalid(command, "Add the action to send, like /me waves."),
        K::Me => message(ComposerMsgType::Emote, None),
        K::Notice if payload.is_empty() => {
            invalid(command, "Add the notice to send, like /notice Back soon.")
        }
        K::Notice => message(ComposerMsgType::Notice, None),
        K::Shrug => message(ComposerMsgType::Text, Some(SHRUG)),
        K::TableFlip => message(ComposerMsgType::Text, Some(TABLEFLIP)),
        K::Unflip => message(ComposerMsgType::Text, Some(UNFLIP)),
        K::Lenny => message(ComposerMsgType::Text, Some(LENNY)),
        K::Spoiler | K::Plain | K::Html if payload.is_empty() => {
            invalid(command, "Add the message to send after the command.")
        }
        K::Spoiler => {
            let mut message = ComposerMessage::new(ComposerMsgType::Text, payload, None);
            message.formatted_body = Some(format!(
                "<span data-mx-spoiler>{}</span>",
                escape_html(payload)
            ));
            message.allow_formatting = false;
            ComposerInput::Message {
                command: Some(command),
                message,
            }
        }
        K::Plain => {
            let mut message = ComposerMessage::new(ComposerMsgType::Text, payload, None);
            message.allow_formatting = false;
            ComposerInput::Message {
                command: Some(command),
                message,
            }
        }
        K::Html => {
            let mut message =
                ComposerMessage::new(ComposerMsgType::Text, &html_to_text(payload), None);
            message.formatted_body = Some(payload.to_owned());
            message.allow_formatting = false;
            ComposerInput::Message {
                command: Some(command),
                message,
            }
        }
        K::Poll => match parse_poll(payload) {
            Some(poll) => args(CommandArgs::Poll(poll)),
            None => invalid(
                command,
                "Write the poll as /poll Question | Option 1 | Option 2, with at least two options.",
            ),
        },
        K::Gif => args(CommandArgs::Text(payload.to_owned())),
        K::StartDm | K::Ignore | K::Unignore | K::Unban => {
            let users = parse_users(payload);
            if users.is_empty() {
                return invalid(command, people_usage(command));
            }
            args(CommandArgs::People {
                users,
                servers: Vec::new(),
                reason: None,
            })
        }
        K::Invite | K::Disinvite => {
            let (content, flags) = split_payload_content_and_flags(payload);
            let users = parse_users(content);
            if users.is_empty() {
                return invalid(command, people_usage(command));
            }
            args(CommandArgs::People {
                users,
                servers: Vec::new(),
                reason: reason_flag(&parse_flags(flags)),
            })
        }
        K::Kick | K::Ban => {
            let (content, flags) = split_payload_content_and_flags(payload);
            let users = parse_users(content);
            let servers = parse_servers(content);
            if users.is_empty() && servers.is_empty() {
                return invalid(command, people_usage(command));
            }
            args(CommandArgs::People {
                users,
                servers,
                reason: reason_flag(&parse_flags(flags)),
            })
        }
        K::Delete => {
            let (content, flags) = split_payload_content_and_flags(payload);
            let flags = parse_flags(flags);
            let Some(past_ms) = flags
                .get("past")
                .and_then(|value| parse_duration_ms(value))
            else {
                return invalid(
                    command,
                    "Add how far back to delete with -past, like -past 1d, 2h, 5m or 30s.",
                );
            };
            let users = parse_users(content);
            let servers = parse_servers(content);
            if users.is_empty() && servers.is_empty() {
                return invalid(
                    command,
                    "Add the Matrix ID or server of the people whose messages to delete.",
                );
            }
            args(CommandArgs::Delete {
                users,
                servers,
                past_ms,
                event_types: flags
                    .get("t")
                    .map(|types| split_with_space(types).into_iter().map(str::to_owned).collect())
                    .unwrap_or_default(),
                reason: reason_flag(&flags),
            })
        }
        K::Join => {
            let rooms: Vec<String> = split_with_space(payload)
                .into_iter()
                .filter(|id| is_room_id(id) || is_room_alias(id))
                .map(str::to_owned)
                .collect();
            if rooms.is_empty() {
                return invalid(command, "Add a room address, like /join #room:server.");
            }
            args(CommandArgs::Rooms(rooms))
        }
        K::Leave => {
            let rooms: Vec<String> = split_with_space(payload)
                .into_iter()
                .filter(|id| is_room_id(id))
                .map(str::to_owned)
                .collect();
            if rooms.is_empty() && !payload.is_empty() {
                return invalid(
                    command,
                    "Add room IDs like !id:server, or use /leave alone to leave this room.",
                );
            }
            args(CommandArgs::Rooms(rooms))
        }
        K::Msg => {
            let (user, text) = split_first_word(payload);
            if user.is_empty() {
                return invalid(command, user_usage(command));
            }
            if text.is_empty() {
                return invalid(
                    command,
                    "Add the message after the person, like /msg @name:server Hi.",
                );
            }
            args(CommandArgs::User {
                user: user.to_owned(),
                text: text.to_owned(),
            })
        }
        // The whole argument is the person, so a member's name may have spaces.
        K::Query | K::Whois | K::Deop if payload.is_empty() => {
            invalid(command, user_usage(command))
        }
        K::Deop => args(CommandArgs::PowerLevel {
            user: payload.to_owned(),
            level: None,
        }),
        K::Query | K::Whois => args(CommandArgs::User {
            user: payload.to_owned(),
            text: String::new(),
        }),
        K::Op => {
            // The level is the last word when it is a number, so a member
            // name with spaces still works: /op Sam Smith 100.
            let (user, level) = match payload.rsplit_once(char::is_whitespace) {
                Some((user, level)) if level.parse::<i64>().is_ok() => {
                    (user.trim(), level.parse::<i64>().ok())
                }
                _ => (payload, None),
            };
            if user.is_empty() {
                return invalid(command, user_usage(command));
            }
            args(CommandArgs::PowerLevel {
                user: user.to_owned(),
                level: Some(level.unwrap_or(50)),
            })
        }
        K::JumpToDate => match parse_date_ms(payload) {
            Some(timestamp_ms) => args(CommandArgs::Date { timestamp_ms }),
            None => invalid(command, "Add a date as YYYY-MM-DD, like /jumptodate 2024-05-01."),
        },
        K::MyAvatar | K::MyRoomAvatar => {
            if is_mxc_uri(payload) {
                return args(CommandArgs::Mxc(payload.to_owned()));
            }
            invalid(
                command,
                if command == K::MyAvatar {
                    "Add an mxc:// image address, like /myavatar mxc://server/id."
                } else {
                    "Add an mxc:// image address, like /myroomavatar mxc://server/id."
                },
            )
        }
        K::UpgradeRoom => {
            let valid = !payload.is_empty()
                && payload
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
            if valid {
                return args(CommandArgs::RoomVersion(payload.to_owned()));
            }
            invalid(command, "Add the room version to upgrade to, like /upgraderoom 11.")
        }
        K::Topic | K::RoomName | K::Nick | K::MyRoomNick if payload.is_empty() => {
            invalid(command, text_usage(command))
        }
        K::Topic | K::RoomName | K::Nick | K::MyRoomNick | K::Help => {
            args(CommandArgs::Text(payload.to_owned()))
        }
        K::Acl => {
            let (_, flags) = split_payload_content_and_flags(payload);
            let flags = parse_flags(flags);
            let list = |key: &str| -> Vec<String> {
                flags
                    .get(key)
                    .map(|value| split_with_space(value).into_iter().map(str::to_owned).collect())
                    .unwrap_or_default()
            };
            let acl = CommandArgs::Acl {
                allow: list("a"),
                deny: list("d"),
                remove_allow: list("ra"),
                remove_deny: list("rd"),
            };
            if matches!(&acl, CommandArgs::Acl { allow, deny, remove_allow, remove_deny }
                if allow.is_empty() && deny.is_empty() && remove_allow.is_empty() && remove_deny.is_empty())
            {
                return invalid(
                    command,
                    "Add servers to change, like /acl -d spam.example or /acl -ra old.example.",
                );
            }
            args(acl)
        }
        K::ConvertToDm | K::ConvertToRoom => args(CommandArgs::None),
    }
}

fn people_usage(command: ComposerCommand) -> &'static str {
    use ComposerCommand as K;
    match command {
        K::StartDm => "Add the Matrix ID of the person to message, like /startdm @name:server.",
        K::Ignore => "Add the Matrix ID of each person to ignore, like /ignore @name:server.",
        K::Unignore => "Add the Matrix ID of each person to unignore, like /unignore @name:server.",
        K::Unban => "Add the Matrix ID of each person to unban, like /unban @name:server.",
        K::Invite => "Add the Matrix ID of each person to invite, like /invite @name:server.",
        K::Disinvite => {
            "Add the Matrix ID of each invite to withdraw, like /disinvite @name:server."
        }
        K::Ban => "Add the Matrix ID of each person to ban, like /ban @name:server.",
        _ => "Add the Matrix ID of each person to remove, like /kick @name:server.",
    }
}

fn user_usage(command: ComposerCommand) -> &'static str {
    use ComposerCommand as K;
    match command {
        K::Msg => "Add the person and the message, like /msg @name:server Hi.",
        K::Query => "Add the person to message, like /query @name:server.",
        K::Whois => "Add the person to look up, like /whois @name:server.",
        K::Op => "Add the person and a power level, like /op @name:server 50.",
        _ => "Add the person, like /deop @name:server.",
    }
}

fn text_usage(command: ComposerCommand) -> &'static str {
    use ComposerCommand as K;
    match command {
        K::Topic => "Add the topic to set, like /topic Weekly planning.",
        K::RoomName => "Add the name to set, like /roomname Team chat.",
        K::Nick => "Add the display name to use, like /nick Sam.",
        _ => "Add the nickname to use, like /myroomnick Sam.",
    }
}

fn reason_flag(flags: &BTreeMap<String, String>) -> Option<String> {
    flags
        .get("r")
        .map(|reason| reason.trim().to_owned())
        .filter(|reason| !reason.is_empty())
}

fn split_first_word(payload: &str) -> (&str, &str) {
    match payload.split_once(char::is_whitespace) {
        Some((first, rest)) => (first, rest.trim()),
        None => (payload, ""),
    }
}

/// Desktop's `splitWithSpace`: single spaces separate words.
pub fn split_with_space(content: &str) -> Vec<&str> {
    let content = content.trim();
    if content.is_empty() {
        return Vec::new();
    }
    content.split(' ').collect()
}

/// Flag starts: `-word` at the start or after whitespace. Byte offsets of the
/// match (including the whitespace before it), the key, and the match end.
fn flag_matches(text: &str) -> Vec<(usize, String, usize)> {
    let mut matches = Vec::new();
    let mut previous: Option<char> = None;
    let mut iter = text.char_indices().peekable();
    while let Some((index, c)) = iter.next() {
        let starts = previous.is_none() || previous.is_some_and(char::is_whitespace);
        if c == '-' && starts {
            let key_start = index + 1;
            let key_end = text[key_start..]
                .char_indices()
                .find(|(_, c)| !(c.is_ascii_alphanumeric() || *c == '_'))
                .map_or(text.len(), |(offset, _)| key_start + offset);
            if key_end > key_start {
                // The match includes the whitespace before the dash.
                let start = match previous {
                    Some(space) => index - space.len_utf8(),
                    None => index,
                };
                matches.push((start, text[key_start..key_end].to_owned(), key_end));
                while iter.peek().is_some_and(|(next, _)| *next < key_end) {
                    iter.next();
                }
                previous = text[..key_end].chars().next_back();
                continue;
            }
        }
        previous = Some(c);
    }
    matches
}

/// Desktop's `splitPayloadContentAndFlags`: the text before the first flag,
/// and the flags.
pub fn split_payload_content_and_flags(payload: &str) -> (&str, Option<&str>) {
    match flag_matches(payload).first() {
        Some((start, _, _)) => (&payload[..*start], Some(&payload[*start..])),
        None => (payload, None),
    }
}

/// Desktop's `parseFlags`: each flag's value runs to the next flag.
pub fn parse_flags(flags: Option<&str>) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    let Some(flags) = flags else {
        return result;
    };
    let matches = flag_matches(flags);
    for (index, (_, key, end)) in matches.iter().enumerate() {
        let next = matches
            .get(index + 1)
            .map_or(flags.len(), |(start, _, _)| *start);
        result.insert(key.clone(), flags[*end..next].trim().to_owned());
    }
    result
}

/// `@localpart:server`.
pub fn is_user_id(id: &str) -> bool {
    is_mx_id(id, '@')
}

pub fn is_room_alias(id: &str) -> bool {
    is_mx_id(id, '#')
}

pub fn is_room_id(id: &str) -> bool {
    id.starts_with('!')
}

fn is_mx_id(id: &str, sigil: char) -> bool {
    let Some(rest) = id.strip_prefix(sigil) else {
        return false;
    };
    let Some((localpart, server)) = rest.split_once(':') else {
        return false;
    };
    !localpart.is_empty() && !server.is_empty() && !id.chars().any(char::is_whitespace)
}

/// A server name such as `example.org` or `example.org:8448`.
pub fn is_server_name(name: &str) -> bool {
    if name.starts_with(['@', '#', '!']) {
        return false;
    }
    let host = match name.rsplit_once(':') {
        Some((host, port)) if !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) => host,
        Some(_) => return false,
        None => name,
    };
    let labels: Vec<&str> = host.split('.').collect();
    labels.len() >= 2
        && labels.iter().all(|label| {
            !label.is_empty() && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
        && labels
            .last()
            .is_some_and(|tld| tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic()))
}

pub fn parse_users(payload: &str) -> Vec<String> {
    let mut users: Vec<String> = Vec::new();
    for item in split_with_space(payload) {
        if is_user_id(item) && !users.iter().any(|user| user == item) {
            users.push(item.to_owned());
        }
    }
    users
}

pub fn parse_servers(payload: &str) -> Vec<String> {
    let mut servers: Vec<String> = Vec::new();
    for item in split_with_space(payload) {
        if is_server_name(item) && !servers.iter().any(|server| server == item) {
            servers.push(item.to_owned());
        }
    }
    servers
}

/// `1.5d`, `2h`, `5m`, `30s` as milliseconds.
pub fn parse_duration_ms(input: &str) -> Option<u64> {
    let input = input.trim();
    let unit = input.chars().last()?;
    let number = &input[..input.len() - unit.len_utf8()];
    let valid_number = !number.is_empty()
        && !number.starts_with('.')
        && !number.ends_with('.')
        && number.chars().filter(|c| *c == '.').count() <= 1
        && number.chars().all(|c| c.is_ascii_digit() || c == '.');
    if !valid_number {
        return None;
    }
    let value: f64 = number.parse().ok()?;
    let unit_ms = match unit {
        'd' => 86_400_000.0,
        'h' => 3_600_000.0,
        'm' => 60_000.0,
        's' => 1_000.0,
        _ => return None,
    };
    let ms = value * unit_ms;
    (ms > 0.0 && ms.is_finite()).then_some(ms as u64)
}

/// Midnight UTC at the start of a `YYYY-MM-DD` date, in ms since the epoch.
pub fn parse_date_ms(input: &str) -> Option<u64> {
    let mut parts = input.trim().split('-');
    let (year, month, day) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return None;
    }
    let year: i64 = year.parse().ok()?;
    let month: u32 = month.parse().ok()?;
    let day: u32 = day.parse().ok()?;
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if year < 1970 || day == 0 || day > days_in_month {
        return None;
    }
    // Days from the civil date (Howard Hinnant's algorithm).
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (i64::from(month) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400_000).ok()
}

pub fn is_mxc_uri(input: &str) -> bool {
    input
        .strip_prefix("mxc://")
        .is_some_and(|rest| !rest.is_empty() && !rest.chars().any(char::is_whitespace))
}

const MAX_POLL_ANSWERS: usize = 20;
const MAX_POLL_TEXT_CHARS: usize = 240;
const MAX_POLL_SELECTIONS: u32 = 10;

fn trim_poll_text(value: &str) -> String {
    value.trim().chars().take(MAX_POLL_TEXT_CHARS).collect()
}

fn parse_max_selections(value: &str) -> Option<u32> {
    let lower = value.to_ascii_lowercase();
    let number = ["max_selections=", "maxselections=", "max="]
        .iter()
        .find_map(|prefix| lower.strip_prefix(prefix))?;
    if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(number.parse::<u64>().ok()?.min(u64::from(u32::MAX)) as u32)
}

/// Desktop's `parsePollCommand`: `Question | Option | Option [| max=N]`.
pub fn parse_poll(payload: &str) -> Option<ComposerPoll> {
    let mut parts = payload.split('|');
    let question = trim_poll_text(parts.next().unwrap_or_default());
    let mut max_selections = 1;
    let mut answers = Vec::new();
    for answer in parts
        .map(trim_poll_text)
        .filter(|answer| !answer.is_empty())
    {
        match parse_max_selections(&answer) {
            Some(max) => max_selections = max,
            None => answers.push(answer),
        }
    }
    answers.truncate(MAX_POLL_ANSWERS);
    if question.is_empty() || answers.len() < 2 {
        return None;
    }
    // At least two answers here, so the upper bound is never below 1.
    let max_selections = max_selections.clamp(1, (answers.len() as u32).min(MAX_POLL_SELECTIONS));
    Some(ComposerPoll {
        question,
        answers,
        max_selections,
    })
}

pub(crate) fn escape_html(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

/// A plain body for /html: tags dropped, common entities decoded.
fn html_to_text(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    let text = text
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&");
    let trimmed = text.trim();
    if trimmed.is_empty() {
        html.to_owned()
    } else {
        trimmed.to_owned()
    }
}
