//! Shared policy for Hermes clarifying questions ("question cards").
//!
//! Hermes asks a clarifying question as an `m.room.message` whose content
//! carries `org.hermes.clarify` (structured), or, before that ships, as a
//! plain-text prompt that starts with `❓` (fallback). Recognition, the
//! answer route (keycap reaction or reply message) and its validation live
//! here so desktop and iOS answer the same way. Hermes stays the authority
//! that binds an answer to a live pending question.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Custom content key of a structured question card.
pub const AGENT_QUESTION_CONTENT_KEY: &str = "org.hermes.clarify";
/// Reaction Hermes seeds (and accepts) to skip a structured question.
pub const AGENT_QUESTION_SKIP_REACTION: &str = "⏭️";
/// Text-presentation form of the skip reaction some keyboards produce.
pub const AGENT_QUESTION_SKIP_REACTION_TEXT: &str = "⏭";
/// Keycap reactions for single-select choices; index 1 is `1️⃣`.
pub const AGENT_QUESTION_KEYCAP_REACTIONS: [&str; 10] =
    ["1️⃣", "2️⃣", "3️⃣", "4️⃣", "5️⃣", "6️⃣", "7️⃣", "8️⃣", "9️⃣", "🔟"];
/// Fallback-mode skip answer, as Hermes' prompt asks for it.
pub const AGENT_QUESTION_FALLBACK_SKIP_BODY: &str = "skip";

const MAX_QUESTION_BODY_CHARS: usize = 20_000;
const MAX_QUESTION_CHARS: usize = 4_000;
const MAX_CHOICES: usize = 20;
const MAX_CHOICE_LABEL_CHARS: usize = 500;
/// Free-text answers are ordinary messages; keep them well below the
/// outbound text bound so validation fails here with a precise diagnostic.
pub const AGENT_QUESTION_MAX_ANSWER_CHARS: usize = 4_000;

/// Where the card came from. Structured cards carry settled state and
/// accept reactions; fallback cards are parsed from the prompt text.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentQuestionSource {
    Structured,
    Fallback,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentQuestionState {
    Pending,
    Answered,
    Skipped,
    Expired,
    Cancelled,
}

impl AgentQuestionState {
    fn from_wire(value: &str) -> Option<Self> {
        Some(match value {
            "pending" => Self::Pending,
            "answered" => Self::Answered,
            "skipped" => Self::Skipped,
            "expired" => Self::Expired,
            "cancelled" => Self::Cancelled,
            _ => return None,
        })
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Answered => "answered",
            Self::Skipped => "skipped",
            Self::Expired => "expired",
            Self::Cancelled => "cancelled",
        }
    }
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentQuestionChoice {
    /// 1-based, as Hermes numbers it.
    pub index: u32,
    pub label: String,
    pub recommended: bool,
}

/// A recognised clarifying question, from either source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentQuestion {
    pub source: AgentQuestionSource,
    pub clarify_id: Option<String>,
    pub question: String,
    pub choices: Vec<AgentQuestionChoice>,
    pub multi_select: bool,
    pub allow_other: bool,
    pub allow_skip: bool,
    pub requester_user_id: Option<String>,
    pub expires_at_ms: Option<u64>,
    pub state: AgentQuestionState,
    pub answer: Option<String>,
}

/// The answer a presenter asks Core to send.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AgentQuestionAnswer {
    /// One choice, by its 1-based index.
    Choice {
        index: u32,
    },
    /// Several choices of a multi-select question.
    Choices {
        indices: Vec<u32>,
    },
    /// Free text ("Other…").
    Other {
        text: String,
    },
    Skip,
}

/// How an answer reaches Hermes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentQuestionAnswerPlan {
    /// Annotate the card with this key.
    Reaction(&'static str),
    /// Send an `m.text` reply to the card with this body.
    Reply(String),
}

/// Parse the structured card from a message's content object. `None` when
/// the key is missing or the payload breaks the v1 contract.
pub fn agent_question_from_content(content: &JsonValue) -> Option<AgentQuestion> {
    let payload = content.get(AGENT_QUESTION_CONTENT_KEY)?.as_object()?;
    if payload.get("v").and_then(JsonValue::as_u64) != Some(1) {
        return None;
    }
    let question = bounded_text(payload.get("question")?.as_str()?, MAX_QUESTION_CHARS)?;
    let mut choices = Vec::new();
    if let Some(raw_choices) = payload.get("choices") {
        for raw in raw_choices.as_array()?.iter().take(MAX_CHOICES) {
            let Some(index) = raw
                .get("index")
                .and_then(JsonValue::as_u64)
                .and_then(|index| u32::try_from(index).ok())
                .filter(|index| *index >= 1)
            else {
                continue;
            };
            let Some(label) = raw
                .get("label")
                .and_then(JsonValue::as_str)
                .and_then(|label| bounded_text(label, MAX_CHOICE_LABEL_CHARS))
            else {
                continue;
            };
            if choices
                .iter()
                .any(|choice: &AgentQuestionChoice| choice.index == index)
            {
                continue;
            }
            choices.push(AgentQuestionChoice {
                index,
                label,
                recommended: raw
                    .get("recommended")
                    .and_then(JsonValue::as_bool)
                    .unwrap_or(false),
            });
        }
        choices.sort_by_key(|choice| choice.index);
    }
    let state = match payload.get("state") {
        None | Some(JsonValue::Null) => AgentQuestionState::Pending,
        Some(value) => AgentQuestionState::from_wire(value.as_str()?)?,
    };
    let flag = |key: &str| payload.get(key).and_then(JsonValue::as_bool) == Some(true);
    Some(AgentQuestion {
        source: AgentQuestionSource::Structured,
        clarify_id: payload
            .get("clarify_id")
            .and_then(JsonValue::as_str)
            .filter(|id| !id.is_empty() && id.len() <= 255)
            .map(str::to_owned),
        question,
        multi_select: flag("multi_select") && !choices.is_empty(),
        choices,
        allow_other: flag("allow_other"),
        allow_skip: flag("allow_skip"),
        requester_user_id: payload
            .get("requester_user_id")
            .and_then(JsonValue::as_str)
            .filter(|id| !id.is_empty() && id.len() <= 255)
            .map(str::to_owned),
        expires_at_ms: payload.get("expires_at_ms").and_then(JsonValue::as_u64),
        state,
        answer: payload
            .get("answer")
            .and_then(JsonValue::as_str)
            .and_then(|answer| bounded_text(answer, AGENT_QUESTION_MAX_ANSWER_CHARS)),
    })
}

/// Parse the structured card from a raw event (original or `m.replace`).
/// An edit's `m.new_content` is the replaced content, so its key wins; a
/// latest edit without the key yields `None` and the caller falls back to the
/// (edited) body.
pub fn agent_question_from_event_json(event_json: &str) -> Option<AgentQuestion> {
    if !event_json.contains(AGENT_QUESTION_CONTENT_KEY) {
        return None;
    }
    let event = serde_json::from_str::<JsonValue>(event_json).ok()?;
    let content = event.get("content")?;
    match content.get("m.new_content") {
        Some(new_content) => agent_question_from_content(new_content),
        None => agent_question_from_content(content),
    }
}

/// Recognise today's plain-text Hermes prompt:
///
/// ```text
/// ❓ <question>
/// Reply "skip" to skip this question.
///
/// 1. <choice> (Recommended)
/// 2. <choice>
///
/// Reply with the number, the option text, or your own answer.
/// ```
///
/// The `❓` line must be the first non-empty line, and the prompt must carry
/// either numbered choices or a `Reply …` instruction line.
pub fn agent_question_from_fallback_body(body: &str) -> Option<AgentQuestion> {
    // Every text row is probed: reject on the leading marker before any
    // whole-body work.
    if !body
        .trim_start()
        .trim_start_matches('*')
        .trim_start()
        .starts_with('❓')
        || body.len() > MAX_QUESTION_BODY_CHARS * 4
        || body.chars().count() > MAX_QUESTION_BODY_CHARS
    {
        return None;
    }
    let mut lines = body
        .lines()
        .map(str::trim)
        .skip_while(|line| line.is_empty());
    let first = lines.next()?.trim_matches('*').trim();
    let question = first
        .strip_prefix('❓')?
        .trim_start_matches('\u{FE0F}')
        .trim()
        .trim_matches('*')
        .trim();
    let mut question = question.to_owned();
    let mut choices: Vec<AgentQuestionChoice> = Vec::new();
    let mut allow_skip = false;
    let mut has_instruction = false;
    let mut in_question = true;
    for line in lines {
        if line.is_empty() {
            in_question = false;
            continue;
        }
        let lower = line.to_lowercase();
        if lower.starts_with("reply ") {
            in_question = false;
            has_instruction = true;
            if lower.contains("\"skip\"") || lower.contains("“skip”") || lower.contains("'skip'")
            {
                allow_skip = true;
            }
            continue;
        }
        if let Some((index, label)) = numbered_choice(line) {
            in_question = false;
            if index as usize == choices.len() + 1 && choices.len() < MAX_CHOICES {
                let (label, recommended) = strip_recommended(label);
                if let Some(label) = bounded_text(label, MAX_CHOICE_LABEL_CHARS) {
                    choices.push(AgentQuestionChoice {
                        index,
                        label,
                        recommended,
                    });
                }
            }
            continue;
        }
        if in_question {
            // A wrapped question continues until the first blank line.
            question.push(' ');
            question.push_str(line);
        }
    }
    if choices.is_empty() && !has_instruction {
        return None;
    }
    let question = bounded_text(&question, MAX_QUESTION_CHARS)?;
    Some(AgentQuestion {
        source: AgentQuestionSource::Fallback,
        clarify_id: None,
        question,
        choices,
        multi_select: false,
        // Hermes accepts "your own answer" for every fallback prompt.
        allow_other: true,
        allow_skip,
        requester_user_id: None,
        expires_at_ms: None,
        state: AgentQuestionState::Pending,
        answer: None,
    })
}

/// Structured content first, then the plain-text fallback. `msgtype` must be
/// `m.text` or `m.notice`.
pub fn recognize_agent_question(event_json: Option<&str>, body: &str) -> Option<AgentQuestion> {
    event_json
        .and_then(agent_question_from_event_json)
        .or_else(|| agent_question_from_fallback_body(body))
}

/// Whether `current_user_id` may answer a card sent by `sender_id`. Like
/// approvals, the card must come from another account; when Hermes names a
/// requester, only that account answers.
pub fn can_answer_agent_question(
    question: &AgentQuestion,
    sender_id: &str,
    current_user_id: &str,
) -> bool {
    !sender_id.is_empty()
        && !current_user_id.is_empty()
        && sender_id != current_user_id
        && question
            .requester_user_id
            .as_deref()
            .is_none_or(|requester| requester == current_user_id)
}

/// Keycap reaction key for a 1-based choice index.
pub fn keycap_reaction(index: u32) -> Option<&'static str> {
    usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_sub(1))
        .and_then(|slot| AGENT_QUESTION_KEYCAP_REACTIONS.get(slot).copied())
}

/// Choice index for a keycap reaction key (with or without VS16).
pub fn keycap_reaction_index(key: &str) -> Option<u32> {
    if key == "🔟" {
        return Some(10);
    }
    let mut chars = key.chars().filter(|ch| *ch != '\u{FE0F}');
    let digit = chars.next()?.to_digit(10)?;
    (chars.next() == Some('\u{20E3}') && chars.next().is_none() && digit >= 1).then_some(digit)
}

pub fn is_skip_reaction(key: &str) -> bool {
    key == AGENT_QUESTION_SKIP_REACTION || key == AGENT_QUESTION_SKIP_REACTION_TEXT
}

/// This account's own reaction answer on a structured card: the earliest of
/// `(key, timestamp_ms)` that is a choice keycap or skip. Fallback cards have
/// no reaction route, so own reactions there are never answers.
pub fn own_reaction_answer<'a>(
    question: &AgentQuestion,
    own_reactions: impl IntoIterator<Item = (&'a str, Option<u64>)>,
) -> Option<AgentQuestionOwnAnswer> {
    if question.source != AgentQuestionSource::Structured {
        return None;
    }
    own_reactions
        .into_iter()
        .filter_map(|(key, at)| {
            let answer = if is_skip_reaction(key) {
                AgentQuestionOwnAnswer::Skipped
            } else {
                let index = keycap_reaction_index(key)?;
                question
                    .choices
                    .iter()
                    .any(|choice| choice.index == index)
                    .then_some(())?;
                AgentQuestionOwnAnswer::Choice { index }
            };
            Some((at.unwrap_or(u64::MAX), answer))
        })
        .min_by_key(|(at, _)| *at)
        .map(|(_, answer)| answer)
}

/// This account's own answer, derived from its reactions on the card.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentQuestionOwnAnswer {
    Choice { index: u32 },
    Skipped,
}

/// Validate `answer` against the card and choose its route. `now_ms` folds
/// `expires_at_ms` into the pending check.
pub fn plan_agent_question_answer(
    question: &AgentQuestion,
    answer: &AgentQuestionAnswer,
    sender_id: &str,
    current_user_id: &str,
    now_ms: u64,
) -> Result<AgentQuestionAnswerPlan, &'static str> {
    if !can_answer_agent_question(question, sender_id, current_user_id) {
        return Err("agent-question-not-requester");
    }
    if question.state != AgentQuestionState::Pending {
        return Err("agent-question-settled");
    }
    if question
        .expires_at_ms
        .is_some_and(|expires_at| expires_at <= now_ms)
    {
        return Err("agent-question-expired");
    }
    let structured = question.source == AgentQuestionSource::Structured;
    let has_choice = |index: u32| question.choices.iter().any(|choice| choice.index == index);
    match answer {
        AgentQuestionAnswer::Choice { index } => {
            if !has_choice(*index) {
                return Err("agent-question-choice-invalid");
            }
            if structured && !question.multi_select {
                if let Some(key) = keycap_reaction(*index) {
                    return Ok(AgentQuestionAnswerPlan::Reaction(key));
                }
            }
            Ok(AgentQuestionAnswerPlan::Reply(index.to_string()))
        }
        AgentQuestionAnswer::Choices { indices } => {
            if !question.multi_select {
                return Err("agent-question-not-multi-select");
            }
            let mut indices = indices.clone();
            indices.sort_unstable();
            indices.dedup();
            if indices.is_empty() || !indices.iter().all(|index| has_choice(*index)) {
                return Err("agent-question-choice-invalid");
            }
            Ok(AgentQuestionAnswerPlan::Reply(
                indices
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(", "),
            ))
        }
        AgentQuestionAnswer::Other { text } => {
            if !question.allow_other {
                return Err("agent-question-other-not-allowed");
            }
            let text = text.trim();
            if text.is_empty() || text.chars().count() > AGENT_QUESTION_MAX_ANSWER_CHARS {
                return Err("agent-question-text-invalid");
            }
            Ok(AgentQuestionAnswerPlan::Reply(text.to_owned()))
        }
        AgentQuestionAnswer::Skip => {
            if !question.allow_skip {
                return Err("agent-question-skip-not-allowed");
            }
            Ok(if structured {
                AgentQuestionAnswerPlan::Reaction(AGENT_QUESTION_SKIP_REACTION)
            } else {
                AgentQuestionAnswerPlan::Reply(AGENT_QUESTION_FALLBACK_SKIP_BODY.to_owned())
            })
        }
    }
}

fn bounded_text(value: &str, max_chars: usize) -> Option<String> {
    let value = value.trim();
    (!value.is_empty() && value.chars().count() <= max_chars).then(|| value.to_owned())
}

fn numbered_choice(line: &str) -> Option<(u32, &str)> {
    let digits = line.len()
        - line
            .trim_start_matches(|ch: char| ch.is_ascii_digit())
            .len();
    if digits == 0 || digits > 2 {
        return None;
    }
    let index = line[..digits].parse::<u32>().ok()?;
    let rest = line[digits..].strip_prefix(['.', ')'])?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let label = rest.trim();
    (!label.is_empty()).then_some((index, label))
}

fn strip_recommended(label: &str) -> (&str, bool) {
    const MARKER: &str = "(recommended)";
    let trimmed = label.trim_end().trim_end_matches('*').trim_end();
    if trimmed.len() >= MARKER.len() {
        let split = trimmed.len() - MARKER.len();
        if trimmed.is_char_boundary(split) && trimmed[split..].eq_ignore_ascii_case(MARKER) {
            return (
                trimmed[..split].trim_end().trim_end_matches('*').trim_end(),
                true,
            );
        }
    }
    (label, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const FALLBACK: &str = "❓ Which database should I use?\nReply \"skip\" to skip this question.\n\n1. Postgres (Recommended)\n2. SQLite\n\nReply with the number, the option text, or your own answer.";

    fn structured(extra: JsonValue) -> JsonValue {
        let mut payload = json!({
            "v": 1,
            "clarify_id": "c1",
            "question": "Pick a colour",
            "choices": [
                {"index": 2, "label": "Blue", "recommended": false},
                {"index": 1, "label": "Red", "recommended": true}
            ],
            "multi_select": false,
            "allow_other": true,
            "allow_skip": true,
            "requester_user_id": "@me:example.org",
            "expires_at_ms": null,
            "state": "pending",
            "answer": null
        });
        for (key, value) in extra.as_object().unwrap() {
            payload[key] = value.clone();
        }
        json!({"msgtype": "m.text", "body": "Pick a colour", AGENT_QUESTION_CONTENT_KEY: payload})
    }

    #[test]
    fn fallback_prompt_parses_question_choices_and_recommendation() {
        let question = agent_question_from_fallback_body(FALLBACK).expect("fallback card");
        assert_eq!(question.source, AgentQuestionSource::Fallback);
        assert_eq!(question.question, "Which database should I use?");
        assert_eq!(
            question.choices,
            vec![
                AgentQuestionChoice {
                    index: 1,
                    label: "Postgres".into(),
                    recommended: true
                },
                AgentQuestionChoice {
                    index: 2,
                    label: "SQLite".into(),
                    recommended: false
                },
            ]
        );
        assert!(question.allow_skip);
        assert!(question.allow_other);
        assert!(!question.multi_select);
        assert_eq!(question.state, AgentQuestionState::Pending);
    }

    #[test]
    fn fallback_open_ended_prompt_needs_an_instruction_line() {
        let open = agent_question_from_fallback_body(
            "❓ What should the release be called?\nReply \"skip\" to skip this question.",
        )
        .expect("open-ended card");
        assert!(open.choices.is_empty());
        assert!(open.allow_skip);
        assert!(agent_question_from_fallback_body("❓ just a thought").is_none());
        assert!(agent_question_from_fallback_body("Hello\n❓ not first\n1. a").is_none());
        assert!(agent_question_from_fallback_body("ordinary message").is_none());
    }

    #[test]
    fn fallback_ignores_out_of_sequence_numbers() {
        let question =
            agent_question_from_fallback_body("❓ Pick\n\n1. One\n3. Three\n2. Two\n").unwrap();
        assert_eq!(
            question
                .choices
                .iter()
                .map(|choice| choice.label.as_str())
                .collect::<Vec<_>>(),
            ["One", "Two"]
        );
        assert!(!question.allow_skip);
    }

    #[test]
    fn structured_content_parses_and_sorts_choices() {
        let question = agent_question_from_content(&structured(json!({}))).expect("card");
        assert_eq!(question.source, AgentQuestionSource::Structured);
        assert_eq!(question.clarify_id.as_deref(), Some("c1"));
        assert_eq!(question.choices[0].index, 1);
        assert!(question.choices[0].recommended);
        assert_eq!(
            question.requester_user_id.as_deref(),
            Some("@me:example.org")
        );
        assert!(agent_question_from_content(&structured(json!({"v": 2}))).is_none());
        assert!(agent_question_from_content(&structured(json!({"state": "weird"}))).is_none());
        assert!(agent_question_from_content(&json!({"body": "x"})).is_none());
    }

    #[test]
    fn latest_edit_new_content_wins() {
        let original = json!({"type": "m.room.message", "content": structured(json!({}))});
        let edit = json!({
            "type": "m.room.message",
            "content": {
                "body": "* Pick a colour",
                "m.new_content": structured(json!({"state": "answered", "answer": "Red"})),
                "m.relates_to": {"rel_type": "m.replace", "event_id": "$card"}
            }
        });
        let pending = agent_question_from_event_json(&original.to_string()).unwrap();
        assert_eq!(pending.state, AgentQuestionState::Pending);
        let settled = agent_question_from_event_json(&edit.to_string()).unwrap();
        assert_eq!(settled.state, AgentQuestionState::Answered);
        assert_eq!(settled.answer.as_deref(), Some("Red"));
        // Structured content outranks a body that also parses as fallback.
        let recognized = recognize_agent_question(Some(&edit.to_string()), FALLBACK).unwrap();
        assert_eq!(recognized.source, AgentQuestionSource::Structured);
        assert_eq!(
            recognize_agent_question(None, FALLBACK).unwrap().source,
            AgentQuestionSource::Fallback
        );
    }

    #[test]
    fn requester_and_sender_rules_gate_answers() {
        let question = agent_question_from_content(&structured(json!({}))).unwrap();
        assert!(can_answer_agent_question(
            &question,
            "@hermes:example.org",
            "@me:example.org"
        ));
        assert!(!can_answer_agent_question(
            &question,
            "@me:example.org",
            "@me:example.org"
        ));
        assert!(!can_answer_agent_question(
            &question,
            "@hermes:example.org",
            "@other:example.org"
        ));
        let open = agent_question_from_fallback_body(FALLBACK).unwrap();
        assert!(can_answer_agent_question(
            &open,
            "@hermes:example.org",
            "@other:example.org"
        ));
    }

    #[test]
    fn structured_single_choice_and_skip_use_reactions() {
        let question = agent_question_from_content(&structured(json!({}))).unwrap();
        let plan = |answer| {
            plan_agent_question_answer(&question, &answer, "@hermes:x", "@me:example.org", 10)
        };
        assert_eq!(
            plan(AgentQuestionAnswer::Choice { index: 2 }),
            Ok(AgentQuestionAnswerPlan::Reaction("2️⃣"))
        );
        assert_eq!(
            plan(AgentQuestionAnswer::Skip),
            Ok(AgentQuestionAnswerPlan::Reaction("⏭️"))
        );
        assert_eq!(
            plan(AgentQuestionAnswer::Other {
                text: "  Green ".into()
            }),
            Ok(AgentQuestionAnswerPlan::Reply("Green".into()))
        );
        assert_eq!(
            plan(AgentQuestionAnswer::Choice { index: 3 }),
            Err("agent-question-choice-invalid")
        );
        assert_eq!(
            plan(AgentQuestionAnswer::Choices { indices: vec![1] }),
            Err("agent-question-not-multi-select")
        );
    }

    #[test]
    fn multi_select_replies_with_sorted_indices() {
        let question =
            agent_question_from_content(&structured(json!({"multi_select": true}))).unwrap();
        assert_eq!(
            plan_agent_question_answer(
                &question,
                &AgentQuestionAnswer::Choices {
                    indices: vec![2, 1, 2]
                },
                "@hermes:x",
                "@me:example.org",
                10,
            ),
            Ok(AgentQuestionAnswerPlan::Reply("1, 2".into()))
        );
        assert_eq!(
            plan_agent_question_answer(
                &question,
                &AgentQuestionAnswer::Skip,
                "@hermes:x",
                "@me:example.org",
                10,
            ),
            Ok(AgentQuestionAnswerPlan::Reaction("⏭️"))
        );
    }

    #[test]
    fn fallback_answers_are_always_replies() {
        let question = agent_question_from_fallback_body(FALLBACK).unwrap();
        let plan = |answer| plan_agent_question_answer(&question, &answer, "@hermes:x", "@me:x", 1);
        assert_eq!(
            plan(AgentQuestionAnswer::Choice { index: 1 }),
            Ok(AgentQuestionAnswerPlan::Reply("1".into()))
        );
        assert_eq!(
            plan(AgentQuestionAnswer::Skip),
            Ok(AgentQuestionAnswerPlan::Reply("skip".into()))
        );
    }

    #[test]
    fn settled_expired_and_disallowed_answers_are_rejected() {
        let settled =
            agent_question_from_content(&structured(json!({"state": "cancelled"}))).unwrap();
        let answer = AgentQuestionAnswer::Choice { index: 1 };
        assert_eq!(
            plan_agent_question_answer(&settled, &answer, "@h:x", "@me:example.org", 1),
            Err("agent-question-settled")
        );
        let expiring =
            agent_question_from_content(&structured(json!({"expires_at_ms": 100}))).unwrap();
        assert_eq!(
            plan_agent_question_answer(&expiring, &answer, "@h:x", "@me:example.org", 100),
            Err("agent-question-expired")
        );
        let strict = agent_question_from_content(&structured(
            json!({"allow_other": false, "allow_skip": false}),
        ))
        .unwrap();
        assert_eq!(
            plan_agent_question_answer(
                &strict,
                &AgentQuestionAnswer::Other { text: "x".into() },
                "@h:x",
                "@me:example.org",
                1
            ),
            Err("agent-question-other-not-allowed")
        );
        assert_eq!(
            plan_agent_question_answer(
                &strict,
                &AgentQuestionAnswer::Skip,
                "@h:x",
                "@me:example.org",
                1
            ),
            Err("agent-question-skip-not-allowed")
        );
    }

    #[test]
    fn keycaps_round_trip_and_own_reaction_answer_is_earliest() {
        for index in 1..=10 {
            assert_eq!(
                keycap_reaction_index(keycap_reaction(index).unwrap()),
                Some(index)
            );
        }
        assert_eq!(keycap_reaction_index("1\u{20E3}"), Some(1));
        assert_eq!(keycap_reaction_index("0️⃣"), None);
        assert_eq!(keycap_reaction(0), None);
        assert_eq!(keycap_reaction(11), None);
        let question = agent_question_from_content(&structured(json!({}))).unwrap();
        assert_eq!(
            own_reaction_answer(
                &question,
                [("👍", Some(1)), ("2️⃣", Some(5)), ("⏭️", Some(9))]
            ),
            Some(AgentQuestionOwnAnswer::Choice { index: 2 })
        );
        assert_eq!(
            own_reaction_answer(&question, [("⏭", Some(1))]),
            Some(AgentQuestionOwnAnswer::Skipped)
        );
        assert_eq!(own_reaction_answer(&question, [("4️⃣", Some(1))]), None);
        let fallback = agent_question_from_fallback_body(FALLBACK).unwrap();
        assert_eq!(own_reaction_answer(&fallback, [("1️⃣", Some(1))]), None);
    }

    #[test]
    fn answer_wire_shape_is_tagged() {
        let answer: AgentQuestionAnswer =
            serde_json::from_value(json!({"kind": "choices", "indices": [1, 3]})).unwrap();
        assert_eq!(
            answer,
            AgentQuestionAnswer::Choices {
                indices: vec![1, 3]
            }
        );
        assert_eq!(
            serde_json::to_value(AgentQuestionAnswer::Skip).unwrap(),
            json!({"kind": "skip"})
        );
        assert!(serde_json::from_value::<AgentQuestionAnswer>(
            json!({"kind": "choice", "index": 1, "extra": true})
        )
        .is_err());
    }
}
