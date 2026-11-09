use std::collections::BTreeMap;
use std::sync::Mutex;

use serde_json::{json, Value as JsonValue};

use super::*;
use crate::app::members::RoomPermissionCapabilities;

// ---- registry --------------------------------------------------------------

#[test]
fn every_command_has_one_entry_and_its_wire_name() {
    let mut names = Vec::new();
    for spec in COMMANDS {
        let wire = serde_json::to_value(spec.command).unwrap();
        assert_eq!(wire, json!(spec.name), "{}", spec.name);
        assert!(
            spec.usage.starts_with(&format!("/{}", spec.name)),
            "{}",
            spec.name
        );
        assert!(!spec.description.is_empty());
        names.push(spec.name);
        names.extend(spec.aliases);
    }
    let mut unique = names.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), names.len(), "a name or alias is used twice");
}

#[test]
fn aliases_reach_their_command() {
    for (alias, command) in [
        ("j", ComposerCommand::Join),
        ("goto", ComposerCommand::Join),
        ("part", ComposerCommand::Leave),
        ("remove", ComposerCommand::Kick),
    ] {
        assert_eq!(
            lookup_command(alias).map(|spec| spec.command),
            Some(command)
        );
    }
    assert!(lookup_command("rainbow").is_none());
}

#[test]
fn destructive_commands_are_the_irreversible_ones() {
    let destructive: Vec<&str> = COMMANDS
        .iter()
        .filter(|spec| spec.destructive)
        .map(|spec| spec.name)
        .collect();
    assert_eq!(destructive, ["ban", "delete", "upgraderoom"]);
}

// ---- parser ----------------------------------------------------------------

fn command(text: &str) -> (ComposerCommand, CommandArgs) {
    match parse_composer_input(text) {
        ComposerInput::Command { command, args } => (command, args),
        other => panic!("{text}: {other:?}"),
    }
}

fn message(text: &str) -> ComposerMessage {
    match parse_composer_input(text) {
        ComposerInput::Message { message, .. } => message,
        other => panic!("{text}: {other:?}"),
    }
}

#[test]
fn typed_command_names_follow_the_desktop_rule() {
    assert_eq!(
        typed_command_name("/remove @puck:example.org"),
        Some("remove")
    );
    assert_eq!(typed_command_name("/kick @a:b -r spam"), Some("kick"));
    assert_eq!(typed_command_name("/leave"), Some("leave"));
    assert_eq!(typed_command_name("/gif\nhigh five"), Some("gif"));
    assert_eq!(typed_command_name("/usr/bin is missing"), None);
    assert_eq!(typed_command_name("//kick is a command"), None);
    assert_eq!(typed_command_name("please /kick them"), None);
    assert_eq!(typed_command_name("/GIF"), None);
    assert_eq!(typed_command_name("/gifs are fun"), Some("gifs"));
}

#[test]
fn plain_text_paths_and_escaped_slashes_are_messages() {
    assert_eq!(message("hello").body, "hello");
    assert_eq!(message("/usr/bin is missing").body, "/usr/bin is missing");
    let escaped = message("//kick is a command");
    assert_eq!(escaped.body, "/kick is a command");
    assert_eq!(escaped.msgtype, ComposerMsgType::Text);
    assert!(matches!(
        parse_composer_input("/rainbow hi"),
        ComposerInput::Unknown { name } if name == "rainbow"
    ));
}

#[test]
fn message_commands_set_msgtype_prefix_and_formatting() {
    let me = message("/me waves");
    assert_eq!(
        (me.msgtype, me.body.as_str()),
        (ComposerMsgType::Emote, "waves")
    );
    assert_eq!(
        message("/notice back soon").msgtype,
        ComposerMsgType::Notice
    );
    let shrug = message("/shrug oh well");
    assert_eq!(shrug.prefix.as_deref(), Some(SHRUG));
    assert_eq!(shrug.text, "oh well");
    assert_eq!(shrug.body, format!("{SHRUG} oh well"));
    assert_eq!(message("/tableflip").body, TABLEFLIP);
    assert_eq!(message("/unflip").body, UNFLIP);
    assert_eq!(message("/lenny hi").body, format!("{LENNY} hi"));
    let spoiler = message("/spoiler the <end>");
    assert_eq!(
        spoiler.formatted_body.as_deref(),
        Some("<span data-mx-spoiler>the &lt;end&gt;</span>")
    );
    assert!(!spoiler.allow_formatting);
    let plain = message("/plain **not bold**");
    assert_eq!(plain.body, "**not bold**");
    assert!(!plain.allow_formatting && plain.formatted_body.is_none());
    let html = message("/html <b>bold</b> &amp; more");
    assert_eq!(html.body, "bold & more");
    assert_eq!(
        html.formatted_body.as_deref(),
        Some("<b>bold</b> &amp; more")
    );
    assert!(matches!(
        parse_composer_input("/me"),
        ComposerInput::Invalid {
            command: ComposerCommand::Me,
            ..
        }
    ));
}

#[test]
fn flags_split_like_desktop() {
    assert_eq!(
        split_payload_content_and_flags("@a:b @c:d -r being rude"),
        ("@a:b @c:d", Some(" -r being rude"))
    );
    assert_eq!(split_payload_content_and_flags("@a:b"), ("@a:b", None));
    // A dash inside a word is not a flag.
    assert_eq!(split_payload_content_and_flags("@a-b:c"), ("@a-b:c", None));
    let flags = parse_flags(Some(" -past 1d -t m.room.message m.sticker -r spam"));
    assert_eq!(flags.get("past").map(String::as_str), Some("1d"));
    assert_eq!(
        flags.get("t").map(String::as_str),
        Some("m.room.message m.sticker")
    );
    assert_eq!(flags.get("r").map(String::as_str), Some("spam"));
    assert_eq!(parse_flags(Some("-a one.org -ra two.org")).len(), 2);
}

#[test]
fn people_commands_type_users_servers_and_reason() {
    let (kind, args) = command("/kick @a:b example.org @a:b -r spam");
    assert_eq!(kind, ComposerCommand::Kick);
    assert_eq!(
        args,
        CommandArgs::People {
            users: vec!["@a:b".into()],
            servers: vec!["example.org".into()],
            reason: Some("spam".into()),
        }
    );
    assert_eq!(command("/remove @a:b").0, ComposerCommand::Kick);
    assert!(is_server_name("matrix.example.org:8448"));
    assert!(!is_server_name("@a:example.org"));
    assert!(!is_server_name("localhost"));
    assert_eq!(parse_users("@a:b nope @c:d"), vec!["@a:b", "@c:d"]);
    for (text, copy) in [
        (
            "/invite",
            "Add the Matrix ID of each person to invite, like /invite @name:server.",
        ),
        (
            "/ban someone",
            "Add the Matrix ID of each person to ban, like /ban @name:server.",
        ),
        (
            "/join somewhere",
            "Add a room address, like /join #room:server.",
        ),
    ] {
        match parse_composer_input(text) {
            ComposerInput::Invalid { message, .. } => assert_eq!(message, copy),
            other => panic!("{text}: {other:?}"),
        }
    }
}

#[test]
fn other_arguments_are_typed() {
    assert_eq!(
        command("/j #room:example.org").1,
        CommandArgs::Rooms(vec!["#room:example.org".into()])
    );
    assert_eq!(command("/part").1, CommandArgs::Rooms(vec![]));
    assert_eq!(
        command("/op Sam Smith 100").1,
        CommandArgs::PowerLevel {
            user: "Sam Smith".into(),
            level: Some(100)
        }
    );
    assert_eq!(
        command("/op @a:b").1,
        CommandArgs::PowerLevel {
            user: "@a:b".into(),
            level: Some(50)
        }
    );
    assert_eq!(
        command("/deop @a:b").1,
        CommandArgs::PowerLevel {
            user: "@a:b".into(),
            level: None
        }
    );
    assert_eq!(
        command("/msg @a:b hello there").1,
        CommandArgs::User {
            user: "@a:b".into(),
            text: "hello there".into()
        }
    );
    assert_eq!(
        command("/jumptodate 2024-03-01").1,
        CommandArgs::Date {
            timestamp_ms: 1_709_251_200_000
        }
    );
    assert!(matches!(
        parse_composer_input("/jumptodate 2024-02-30"),
        ComposerInput::Invalid { .. }
    ));
    assert_eq!(
        command("/upgraderoom 11").1,
        CommandArgs::RoomVersion("11".into())
    );
    assert_eq!(
        command("/myavatar mxc://s/id").1,
        CommandArgs::Mxc("mxc://s/id".into())
    );
    assert!(matches!(
        parse_composer_input("/myroomavatar http://x"),
        ComposerInput::Invalid { .. }
    ));
    assert_eq!(
        command("/delete @a:b -past 1.5h -t m.room.message").1,
        CommandArgs::Delete {
            users: vec!["@a:b".into()],
            servers: vec![],
            past_ms: 5_400_000,
            event_types: vec!["m.room.message".into()],
            reason: None,
        }
    );
    assert_eq!(parse_duration_ms("30s"), Some(30_000));
    assert_eq!(parse_duration_ms("1w"), None);
    assert_eq!(
        command("/acl -d evil.org -ra old.org").1,
        CommandArgs::Acl {
            allow: vec![],
            deny: vec!["evil.org".into()],
            remove_allow: vec!["old.org".into()],
            remove_deny: vec![],
        }
    );
    match command("/poll Lunch? | Pizza | Soup | max=5").1 {
        CommandArgs::Poll(poll) => {
            assert_eq!(poll.question, "Lunch?");
            assert_eq!(poll.answers, ["Pizza", "Soup"]);
            assert_eq!(poll.max_selections, 2);
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        parse_composer_input("/poll Lunch? | Pizza"),
        ComposerInput::Invalid { .. }
    ));
}

// ---- suggestions -----------------------------------------------------------

fn caps(edit: impl FnOnce(&mut RoomPermissionCapabilities)) -> CommandContext {
    let mut capabilities = RoomPermissionCapabilities {
        can_send_message: true,
        ..Default::default()
    };
    edit(&mut capabilities);
    CommandContext {
        capabilities: Some(capabilities),
        is_direct: false,
    }
}

fn names(suggestions: &[CommandSuggestion]) -> Vec<&str> {
    suggestions.iter().map(|s| s.name.as_str()).collect()
}

#[test]
fn suggestions_hide_what_the_user_cannot_do() {
    let member = caps(|_| {});
    let all = command_suggestions("", &member);
    assert!(!names(&all).contains(&"ban"));
    assert!(!names(&all).contains(&"kick"));
    assert!(!names(&all).contains(&"topic"));
    assert!(names(&all).contains(&"me"));
    assert!(names(&all).contains(&"join"));

    let moderator = caps(|c| {
        c.can_ban = true;
        c.can_kick = true;
    });
    assert_eq!(names(&command_suggestions("/ba", &moderator)), ["ban"]);
    assert!(command_allowed(ComposerCommand::Ban, &moderator));
    assert!(!command_allowed(ComposerCommand::Ban, &member));

    // Unknown power levels offer everything.
    assert!(names(&command_suggestions("ban", &CommandContext::default())).contains(&"ban"));

    let acl = caps(|c| {
        c.event_allowed.insert("m.room.server_acl".into(), true);
    });
    assert!(names(&command_suggestions("acl", &acl)).contains(&"acl"));
    assert!(!names(&command_suggestions("acl", &member)).contains(&"acl"));
}

#[test]
fn suggestions_follow_the_room_kind_and_aliases() {
    let room = caps(|_| {});
    let mut dm = room.clone();
    dm.is_direct = true;
    assert_eq!(
        names(&command_suggestions("convert", &room)),
        ["converttodm"]
    );
    assert_eq!(
        names(&command_suggestions("convert", &dm)),
        ["converttoroom"]
    );
    let part = command_suggestions("pa", &room);
    assert_eq!(names(&part), ["leave"]);
    assert_eq!(part[0].matched_alias.as_deref(), Some("part"));
    // A primary-name match comes before an alias match.
    let j = command_suggestions("j", &room);
    assert_eq!(names(&j), ["join", "jumptodate"]);
    assert!(j[0].matched_alias.is_none());
    assert_eq!(
        command_suggestions("go", &room)[0].matched_alias.as_deref(),
        Some("goto")
    );
}

// ---- typed mentions --------------------------------------------------------

fn candidate(user_id: &str, name: Option<&str>) -> MentionCandidate {
    MentionCandidate {
        user_id: user_id.into(),
        display_name: name.map(Into::into),
    }
}

#[test]
fn typed_mentions_resolve_names_ids_and_localparts() {
    let members = [
        candidate("@puck:example.org", Some("Puck")),
        candidate("@sam:example.org", Some("Sam Smith")),
        candidate("@me:example.org", Some("Me")),
    ];
    let found = resolve_typed_mentions(
        "hi @puck, ask @Sam Smith and @sam:example.org or @me",
        &members,
        "@me:example.org",
    );
    let ids: Vec<(&str, &str)> = found
        .iter()
        .map(|m| (m.user_id.as_str(), m.text.as_str()))
        .collect();
    assert_eq!(
        ids,
        [
            ("@puck:example.org", "@puck"),
            ("@sam:example.org", "@Sam Smith"),
            ("@sam:example.org", "@sam:example.org"),
        ]
    );
    assert_eq!((found[0].start, found[0].end), (3, 8));
}

#[test]
fn typed_mentions_leave_ambiguous_partial_and_unknown_names_as_text() {
    let members = [
        candidate("@a:example.org", Some("Alex")),
        candidate("@b:example.org", Some("Alex")),
        candidate("@c:example.org", Some("Al")),
    ];
    // Two members are called Alex; "@Alexa" is nobody; mid-word "@" is not a mention.
    assert!(resolve_typed_mentions("@Alex @Alexa a@Al @Zed", &members, "").is_empty());
    let found = resolve_typed_mentions("(@al)", &members, "");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].user_id, "@c:example.org");
}

#[test]
fn typed_mention_ranges_count_utf16_units() {
    let members = [candidate("@z:example.org", Some("Zoë 🎉"))];
    let found = resolve_typed_mentions("🎉 @zoë 🎉!", &members, "");
    assert_eq!(found.len(), 1);
    // "🎉 " is three UTF-16 units; "@Zoë 🎉" is seven.
    assert_eq!((found[0].start, found[0].end), (3, 10));
}

// ---- running ---------------------------------------------------------------

#[derive(Default)]
struct FakeOps {
    context: CommandContext,
    members: Vec<ComposerMember>,
    state: BTreeMap<(String, String), JsonValue>,
    dm: Option<String>,
    fail_for: Vec<String>,
    calls: Mutex<Vec<String>>,
}

impl FakeOps {
    fn record(&self, call: String) -> OpResult<()> {
        let failed = self
            .fail_for
            .iter()
            .any(|item| call.contains(item.as_str()));
        self.calls.lock().unwrap().push(call);
        if failed {
            Err("v-rooms-members-moderation-forbidden".into())
        } else {
            Ok(())
        }
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

fn member(user_id: &str, name: &str, membership: &str) -> ComposerMember {
    ComposerMember {
        user_id: user_id.into(),
        display_name: Some(name.into()),
        membership: membership.into(),
    }
}

impl ComposerOps for FakeOps {
    fn own_user_id(&self) -> Option<String> {
        Some("@me:example.org".into())
    }
    fn now_ms(&self) -> u64 {
        10_000_000
    }
    async fn context(&self, _: &str) -> OpResult<CommandContext> {
        Ok(self.context.clone())
    }
    async fn members(&self, _: &str) -> OpResult<Vec<ComposerMember>> {
        Ok(self.members.clone())
    }
    async fn invite(&self, _: &str, user: &str, reason: Option<String>) -> OpResult<()> {
        self.record(format!("invite {user} {reason:?}"))
    }
    async fn kick(&self, _: &str, user: &str, reason: Option<String>) -> OpResult<()> {
        self.record(format!("kick {user} {reason:?}"))
    }
    async fn ban(&self, _: &str, user: &str, _: Option<String>) -> OpResult<()> {
        self.record(format!("ban {user}"))
    }
    async fn unban(&self, _: &str, user: &str) -> OpResult<()> {
        self.record(format!("unban {user}"))
    }
    async fn join(&self, target: &str) -> OpResult<()> {
        self.record(format!("join {target}"))
    }
    async fn leave(&self, room: &str) -> OpResult<()> {
        self.record(format!("leave {room}"))
    }
    async fn ignore(&self, user: &str) -> OpResult<()> {
        self.record(format!("ignore {user}"))
    }
    async fn unignore(&self, user: &str) -> OpResult<()> {
        self.record(format!("unignore {user}"))
    }
    async fn set_room_name(&self, _: &str, name: &str) -> OpResult<()> {
        self.record(format!("name {name}"))
    }
    async fn set_room_topic(&self, _: &str, topic: &str) -> OpResult<()> {
        self.record(format!("topic {topic}"))
    }
    async fn set_display_name(&self, name: &str) -> OpResult<()> {
        self.record(format!("nick {name}"))
    }
    async fn set_avatar(&self, mxc: &str) -> OpResult<()> {
        self.record(format!("avatar {mxc}"))
    }
    async fn state_content(&self, _: &str, kind: &str, key: &str) -> OpResult<Option<JsonValue>> {
        Ok(self.state.get(&(kind.to_owned(), key.to_owned())).cloned())
    }
    async fn send_state(&self, _: &str, kind: &str, key: &str, content: JsonValue) -> OpResult<()> {
        self.record(format!("state {kind} {key} {content}"))
    }
    async fn set_power_level(&self, _: &str, user: &str, level: i64) -> OpResult<()> {
        self.record(format!("power {user} {level}"))
    }
    async fn dm_room(&self, _: &str) -> OpResult<Option<String>> {
        Ok(self.dm.clone())
    }
    async fn create_dm(&self, users: &[String]) -> OpResult<String> {
        self.record(format!("create_dm {}", users.join(",")))?;
        Ok("!new:example.org".into())
    }
    async fn mark_dm(&self, room: &str, user: &str) -> OpResult<()> {
        self.record(format!("mark_dm {room} {user}"))
    }
    async fn unmark_dm(&self, room: &str) -> OpResult<()> {
        self.record(format!("unmark_dm {room}"))
    }
    async fn send_text(&self, room: &str, body: &str) -> OpResult<()> {
        self.record(format!("send {room} {body}"))
    }
    async fn bulk_redact(
        &self,
        _: &str,
        users: &[String],
        since: u64,
        _: &[String],
        _: Option<String>,
    ) -> OpResult<RedactSummary> {
        self.record(format!("redact {} since {since}", users.join(",")))?;
        Ok(RedactSummary {
            redacted: 3,
            failed: 0,
            truncated: false,
        })
    }
    async fn upgrade_room(&self, _: &str, version: &str) -> OpResult<String> {
        self.record(format!("upgrade {version}"))?;
        Ok("!upgraded:example.org".into())
    }
    async fn event_at(&self, _: &str, ts: u64) -> OpResult<Option<String>> {
        Ok((ts > 0).then(|| "$first".to_owned()))
    }
}

fn run(ops: &FakeOps, text: &str, confirmed: bool) -> CommandResult {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(run_command(ops, "!room:example.org", text, confirmed))
}

#[test]
fn message_commands_come_back_to_be_sent() {
    let ops = FakeOps::default();
    match run(&ops, "/me waves", false) {
        Ok(CommandOutcome::SendMessage { message }) => {
            assert_eq!(message.msgtype, ComposerMsgType::Emote)
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        run(&ops, "/poll Q | A | B", false),
        Ok(CommandOutcome::SendPoll { .. })
    ));
    assert_eq!(
        run(&ops, "/gif high five", false),
        Ok(CommandOutcome::OpenGifPicker {
            query: "high five".into()
        })
    );
    let unknown = run(&ops, "/rainbow hi", false).unwrap_err();
    assert_eq!(unknown.kind, ComposerErrorKind::UnknownCommand);
    assert_eq!(
        unknown.message,
        "Synara does not know the command /rainbow."
    );
}

#[test]
fn per_person_commands_report_each_failure() {
    let ops = FakeOps {
        fail_for: vec!["@c:x".into()],
        ..Default::default()
    };
    assert_eq!(
        run(&ops, "/invite @a:x -r welcome", false),
        Ok(CommandOutcome::Done {
            message: "Invited @a:x.".into()
        })
    );
    assert_eq!(ops.calls(), ["invite @a:x Some(\"welcome\")"]);
    let error = run(&ops, "/kick @a:x @b:x @c:x", false).unwrap_err();
    assert_eq!(error.kind, ComposerErrorKind::Partial);
    assert_eq!(
        error.message,
        "Removed 2 of 3 people. Couldn't remove @c:x. Try again. You do not have permission to do that in this room."
    );
}

#[test]
fn server_arguments_expand_to_members() {
    let ops = FakeOps {
        members: vec![
            member("@a:spam.org", "A", "join"),
            member("@b:spam.org", "B", "ban"),
            member("@c:good.org", "C", "join"),
        ],
        ..Default::default()
    };
    run(&ops, "/kick spam.org", false).unwrap();
    assert_eq!(ops.calls(), ["kick @a:spam.org None"]);
}

#[test]
fn destructive_commands_ask_first() {
    let ops = FakeOps::default();
    match run(&ops, "/ban @a:x -r spam", false) {
        Ok(CommandOutcome::NeedsConfirmation {
            title,
            confirm_label,
            ..
        }) => {
            assert_eq!(title, "Ban @a:x?");
            assert_eq!(confirm_label, "Ban");
        }
        other => panic!("{other:?}"),
    }
    assert!(ops.calls().is_empty());
    run(&ops, "/ban @a:x -r spam", true).unwrap();
    assert_eq!(ops.calls(), ["ban @a:x"]);

    assert!(matches!(
        run(&ops, "/upgraderoom 11", false),
        Ok(CommandOutcome::NeedsConfirmation { .. })
    ));
    assert_eq!(
        run(&ops, "/upgraderoom 11", true),
        Ok(CommandOutcome::Navigate {
            room_id: "!upgraded:example.org".into(),
            event_id: None
        })
    );
    run(&ops, "/delete @a:x -past 1h", true).unwrap();
    assert!(ops
        .calls()
        .contains(&format!("redact @a:x since {}", 10_000_000 - 3_600_000)));
}

#[test]
fn commands_the_user_may_not_use_are_refused() {
    let ops = FakeOps {
        context: caps(|_| {}),
        ..Default::default()
    };
    let error = run(&ops, "/ban @a:x", true).unwrap_err();
    assert_eq!(error.kind, ComposerErrorKind::Permission);
    assert_eq!(
        error.message,
        "You do not have permission to use /ban in this room."
    );
    let error = run(&ops, "/converttoroom", false).unwrap_err();
    assert_eq!(error.message, "/converttoroom works in a direct message.");
    assert!(ops.calls().is_empty());
}

#[test]
fn navigation_and_profile_outcomes() {
    let ops = FakeOps {
        members: vec![
            member("@puck:x", "Puck", "join"),
            member("@me:example.org", "Me", "join"),
        ],
        dm: Some("!dm:x".into()),
        ..Default::default()
    };
    assert_eq!(
        run(&ops, "/whois puck", false),
        Ok(CommandOutcome::OpenProfile {
            user_id: "@puck:x".into()
        })
    );
    assert_eq!(
        run(&ops, "/query @Puck", false),
        Ok(CommandOutcome::Navigate {
            room_id: "!dm:x".into(),
            event_id: None
        })
    );
    run(&ops, "/msg Puck hello there", false).unwrap();
    assert!(ops.calls().contains(&"send !dm:x hello there".to_owned()));
    assert_eq!(
        run(&ops, "/jumptodate 2024-03-01", false),
        Ok(CommandOutcome::Navigate {
            room_id: "!room:example.org".into(),
            event_id: Some("$first".into())
        })
    );
    assert_eq!(
        run(&ops, "/whois Nobody", false).unwrap_err().kind,
        ComposerErrorKind::NotFound
    );
    // No DM yet: /startdm creates and marks one.
    let fresh = FakeOps::default();
    run(&fresh, "/startdm @a:x @me:example.org", false).unwrap();
    assert_eq!(
        fresh.calls(),
        ["create_dm @a:x", "mark_dm !new:example.org @a:x"]
    );
    run(&ops, "/converttodm", false).unwrap();
    assert!(ops
        .calls()
        .contains(&"mark_dm !room:example.org @puck:x".to_owned()));
}

#[test]
fn profile_and_power_commands_write_the_right_state() {
    let mut state = BTreeMap::new();
    state.insert(
        ("m.room.member".to_owned(), "@me:example.org".to_owned()),
        json!({ "membership": "join", "displayname": "Old" }),
    );
    state.insert(
        ("m.room.power_levels".to_owned(), String::new()),
        json!({ "users_default": 5 }),
    );
    let ops = FakeOps {
        state,
        members: vec![member("@puck:x", "Puck", "join")],
        ..Default::default()
    };
    run(&ops, "/myroomnick New Name", false).unwrap();
    run(&ops, "/deop Puck", false).unwrap();
    run(&ops, "/op Puck 100", false).unwrap();
    run(&ops, "/nick Global", false).unwrap();
    let calls = ops.calls();
    let content: JsonValue = serde_json::from_str(
        calls[0]
            .strip_prefix("state m.room.member @me:example.org ")
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        content,
        json!({ "membership": "join", "displayname": "New Name" })
    );
    assert_eq!(
        &calls[1..],
        ["power @puck:x 5", "power @puck:x 100", "nick Global"]
    );
    let empty = FakeOps::default();
    assert_eq!(
        run(&empty, "/myroomnick Sam", false).unwrap_err().message,
        "Synara could not read your membership in this room yet. Try again in a moment."
    );
}

#[test]
fn acl_starts_from_allow_all_and_keeps_existing_lists() {
    let fresh = server_acl_content(None, &[], &["evil.org".into()], &[], &[]);
    assert_eq!(fresh, json!({ "allow": ["*"], "deny": ["evil.org"] }));
    let existing = server_acl_content(
        Some(json!({ "allow": ["b.org", "a.org"], "deny": [], "allow_ip_literals": false })),
        &["c.org".into()],
        &[],
        &["b.org".into()],
        &[],
    );
    assert_eq!(
        existing,
        json!({ "allow": ["a.org", "c.org"], "deny": [], "allow_ip_literals": false })
    );
}

#[test]
fn help_lists_what_the_user_can_use() {
    let ops = FakeOps {
        context: caps(|_| {}),
        ..Default::default()
    };
    match run(&ops, "/help", false) {
        Ok(CommandOutcome::Help { commands }) => {
            assert!(names(&commands).contains(&"me"));
            assert!(!names(&commands).contains(&"ban"));
        }
        other => panic!("{other:?}"),
    }
    match run(&ops, "/help part", false) {
        Ok(CommandOutcome::Help { commands }) => assert_eq!(names(&commands), ["leave"]),
        other => panic!("{other:?}"),
    }
}

#[test]
fn failures_keep_the_copy_and_the_diagnostic() {
    let ops = FakeOps {
        fail_for: vec!["topic".into()],
        ..Default::default()
    };
    let error = run(&ops, "/topic Hi", false).unwrap_err();
    assert_eq!(error.kind, ComposerErrorKind::Failed);
    assert_eq!(
        error.message,
        "/topic did not finish: You do not have permission to do that in this room."
    );
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("v-rooms-members-moderation-forbidden")
    );
}
