//! Live homeserver push-rule editor (default room modes, mentions, keywords).
//!
//! Uses `Client::notification_settings()`. No tokens. Keyword strings may
//! cross as method/command arguments; failed errors never echo them.

use matrix_sdk::notification_settings::{IsEncrypted, IsOneToOne, RoomNotificationMode};
use matrix_sdk::ruma::push::RuleKind;
use matrix_sdk::Client;
use serde::{Deserialize, Serialize};

const MAX_KEYWORD_CHARS: usize = 128;
const MAX_KEYWORDS: usize = 64;

const RULE_USER_MENTION: &str = ".m.rule.is_user_mention";
const RULE_DISPLAY_NAME: &str = ".m.rule.contains_display_name";
const RULE_USER_NAME: &str = ".m.rule.contains_user_name";
const RULE_ROOM_MENTION: &str = ".m.rule.is_room_mention";
const RULE_AT_ROOM: &str = ".m.rule.roomnotif";

/// Product snapshot of global push defaults, mention enables, and keywords.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixPushRulesSnapshot {
    pub dm: String,
    pub dm_encrypted: String,
    pub group: String,
    pub group_encrypted: String,
    pub mentions: MatrixPushRuleMentions,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixPushRuleMentions {
    pub user_mention: bool,
    pub display_name: bool,
    pub user_name: bool,
    pub room_mention: bool,
    pub at_room: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixPushRulesWriteResult {
    pub status: &'static str,
}

pub(crate) fn mode_to_wire(mode: RoomNotificationMode) -> &'static str {
    match mode {
        RoomNotificationMode::AllMessages => "all",
        RoomNotificationMode::MentionsAndKeywordsOnly => "mentions",
        RoomNotificationMode::Mute => "mute",
    }
}

pub(crate) fn parse_mode(mode: &str) -> Result<RoomNotificationMode, &'static str> {
    match mode.trim() {
        "all" => Ok(RoomNotificationMode::AllMessages),
        "mentions" => Ok(RoomNotificationMode::MentionsAndKeywordsOnly),
        "mute" => Ok(RoomNotificationMode::Mute),
        _ => Err("v-push.invalid-mode"),
    }
}

fn parse_keyword(keyword: &str) -> Result<String, &'static str> {
    let trimmed = keyword.trim();
    if trimmed.is_empty() {
        return Err("v-push.invalid-keyword");
    }
    if trimmed.chars().count() > MAX_KEYWORD_CHARS {
        return Err("v-push.invalid-keyword");
    }
    Ok(trimmed.to_owned())
}

fn mention_rule(rule_id: &str) -> Result<(RuleKind, &'static str), &'static str> {
    match rule_id.trim() {
        "userMention" => Ok((RuleKind::Override, RULE_USER_MENTION)),
        "displayName" => Ok((RuleKind::Override, RULE_DISPLAY_NAME)),
        "userName" => Ok((RuleKind::Content, RULE_USER_NAME)),
        "roomMention" => Ok((RuleKind::Override, RULE_ROOM_MENTION)),
        "atRoom" => Ok((RuleKind::Override, RULE_AT_ROOM)),
        _ => Err("v-push.invalid-rule"),
    }
}

async fn mention_enabled(
    settings: &matrix_sdk::notification_settings::NotificationSettings,
    kind: RuleKind,
    rule_id: &str,
) -> bool {
    settings
        .is_push_rule_enabled(kind, rule_id)
        .await
        .unwrap_or(true)
}

pub async fn snapshot_push_rules(client: &Client) -> Result<MatrixPushRulesSnapshot, &'static str> {
    let _ = client.user_id().ok_or("v-push.no-session")?;
    let settings = client.notification_settings().await;
    let dm = settings
        .get_default_room_notification_mode(IsEncrypted::No, IsOneToOne::Yes)
        .await;
    let dm_encrypted = settings
        .get_default_room_notification_mode(IsEncrypted::Yes, IsOneToOne::Yes)
        .await;
    let group = settings
        .get_default_room_notification_mode(IsEncrypted::No, IsOneToOne::No)
        .await;
    let group_encrypted = settings
        .get_default_room_notification_mode(IsEncrypted::Yes, IsOneToOne::No)
        .await;
    let user_mention = mention_enabled(&settings, RuleKind::Override, RULE_USER_MENTION).await;
    let display_name = mention_enabled(&settings, RuleKind::Override, RULE_DISPLAY_NAME).await;
    let user_name = mention_enabled(&settings, RuleKind::Content, RULE_USER_NAME).await;
    let room_mention = mention_enabled(&settings, RuleKind::Override, RULE_ROOM_MENTION).await;
    let at_room = mention_enabled(&settings, RuleKind::Override, RULE_AT_ROOM).await;
    let mut keywords: Vec<String> = settings.enabled_keywords().await.into_iter().collect();
    keywords.sort();
    keywords.truncate(MAX_KEYWORDS);
    Ok(MatrixPushRulesSnapshot {
        dm: mode_to_wire(dm).to_owned(),
        dm_encrypted: mode_to_wire(dm_encrypted).to_owned(),
        group: mode_to_wire(group).to_owned(),
        group_encrypted: mode_to_wire(group_encrypted).to_owned(),
        mentions: MatrixPushRuleMentions {
            user_mention,
            display_name,
            user_name,
            room_mention,
            at_room,
        },
        keywords,
    })
}

pub async fn set_default_room_mode(
    client: &Client,
    encrypted: bool,
    one_to_one: bool,
    mode: &str,
) -> Result<MatrixPushRulesWriteResult, &'static str> {
    let _ = client.user_id().ok_or("v-push.no-session")?;
    let mode = parse_mode(mode)?;
    client
        .notification_settings()
        .await
        .set_default_room_notification_mode(encrypted.into(), one_to_one.into(), mode)
        .await
        .map_err(|_| "v-push.sdk-failed")?;
    Ok(MatrixPushRulesWriteResult { status: "ok" })
}

pub async fn set_mention_enabled(
    client: &Client,
    rule_id: &str,
    enabled: bool,
) -> Result<MatrixPushRulesWriteResult, &'static str> {
    let _ = client.user_id().ok_or("v-push.no-session")?;
    let (kind, matrix_rule_id) = mention_rule(rule_id)?;
    client
        .notification_settings()
        .await
        .set_push_rule_enabled(kind, matrix_rule_id, enabled)
        .await
        .map_err(|_| "v-push.sdk-failed")?;
    Ok(MatrixPushRulesWriteResult { status: "ok" })
}

pub async fn add_keyword(
    client: &Client,
    keyword: &str,
) -> Result<MatrixPushRulesWriteResult, &'static str> {
    let _ = client.user_id().ok_or("v-push.no-session")?;
    let keyword = parse_keyword(keyword)?;
    let settings = client.notification_settings().await;
    if settings.enabled_keywords().await.len() >= MAX_KEYWORDS {
        return Err("v-push.invalid-keyword");
    }
    settings
        .add_keyword(keyword)
        .await
        .map_err(|_| "v-push.sdk-failed")?;
    Ok(MatrixPushRulesWriteResult { status: "ok" })
}

pub async fn remove_keyword(
    client: &Client,
    keyword: &str,
) -> Result<MatrixPushRulesWriteResult, &'static str> {
    let _ = client.user_id().ok_or("v-push.no-session")?;
    let keyword = parse_keyword(keyword)?;
    client
        .notification_settings()
        .await
        .remove_keyword(&keyword)
        .await
        .map_err(|_| "v-push.sdk-failed")?;
    Ok(MatrixPushRulesWriteResult { status: "ok" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_round_trip() {
        assert_eq!(
            parse_mode("all").unwrap(),
            RoomNotificationMode::AllMessages
        );
        assert_eq!(
            parse_mode("mentions").unwrap(),
            RoomNotificationMode::MentionsAndKeywordsOnly
        );
        assert_eq!(parse_mode("mute").unwrap(), RoomNotificationMode::Mute);
        assert_eq!(parse_mode("loud").unwrap_err(), "v-push.invalid-mode");
        assert_eq!(mode_to_wire(RoomNotificationMode::AllMessages), "all");
    }

    #[test]
    fn keyword_and_rule_guards() {
        assert_eq!(parse_keyword("   ").unwrap_err(), "v-push.invalid-keyword");
        assert_eq!(
            parse_keyword(&"x".repeat(MAX_KEYWORD_CHARS + 1)).unwrap_err(),
            "v-push.invalid-keyword"
        );
        assert_eq!(parse_keyword("  alert  ").unwrap(), "alert");
        assert_eq!(mention_rule("userMention").unwrap().1, RULE_USER_MENTION);
        assert_eq!(mention_rule("nope").unwrap_err(), "v-push.invalid-rule");
        assert!(!RULE_USER_MENTION.contains('@'));
    }
}
