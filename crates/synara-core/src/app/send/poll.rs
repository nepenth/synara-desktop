//! V-SEND.3 — native poll start/response content builders and validation.
//!
//! Matches the desktop JS poll answer-id algorithm so composer-created polls
//! remain readable by the retained PollContent renderer.

use matrix_sdk::ruma::{
    events::{
        poll::{
            start::PollKind,
            unstable_response::UnstablePollResponseEventContent,
            unstable_start::{
                NewUnstablePollStartEventContent, UnstablePollAnswer, UnstablePollAnswers,
                UnstablePollStartContentBlock,
            },
        },
        relation::{Reply, Thread},
        room::message::RelationWithoutReplacement,
    },
    OwnedEventId, UInt,
};

pub const MAX_POLL_ANSWERS: usize = 20;
pub const MAX_POLL_TEXT_CHARS: usize = 240;
pub const MAX_POLL_SELECTIONS: usize = 10;
pub const MIN_POLL_ANSWERS: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedPoll {
    pub question: String,
    pub answers: Vec<(String, String)>,
    pub max_selections: u32,
    pub fallback_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollSendError {
    Question,
    Answers,
    PollEventId,
    AnswerIds,
}

impl PollSendError {
    pub fn diagnostic_id(self) -> &'static str {
        match self {
            Self::Question => "v-send.3-poll-invalid-question",
            Self::Answers => "v-send.3-poll-invalid-answers",
            Self::PollEventId => "v-send.3-poll-invalid-event-id",
            Self::AnswerIds => "v-send.3-poll-invalid-answer-ids",
        }
    }
}

fn trim_poll_text(value: &str) -> String {
    value.trim().chars().take(MAX_POLL_TEXT_CHARS).collect()
}

/// Same algorithm as `synara/src/app/utils/polls.ts` `pollAnswerId`.
pub fn poll_answer_id(index: usize, answer: &str) -> String {
    let mut hash: u64 = 0;
    for unit in answer.encode_utf16() {
        hash = (hash.wrapping_mul(31).wrapping_add(u64::from(unit))) % 2_147_483_647;
    }
    format!("a{}_{}", index + 1, to_base36(hash))
}

fn to_base36(mut n: u64) -> String {
    if n == 0 {
        return "0".into();
    }
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = Vec::new();
    while n > 0 {
        out.push(DIGITS[(n % 36) as usize]);
        n /= 36;
    }
    out.reverse();
    String::from_utf8(out).expect("base36 digits are ascii")
}

pub fn normalize_poll(
    question: &str,
    answers: &[String],
    max_selections: u32,
) -> Result<NormalizedPoll, PollSendError> {
    let question = trim_poll_text(question);
    if question.is_empty() {
        return Err(PollSendError::Question);
    }

    let clean_answers: Vec<String> = answers
        .iter()
        .map(|answer| trim_poll_text(answer))
        .filter(|answer| !answer.is_empty())
        .take(MAX_POLL_ANSWERS)
        .collect();
    if clean_answers.len() < MIN_POLL_ANSWERS {
        return Err(PollSendError::Answers);
    }

    let max_allowed = clean_answers.len().clamp(1, MAX_POLL_SELECTIONS) as u32;
    // Clamp like JS `normalizePollParts` (composer / slash-command).
    let max_selections = max_selections.clamp(1, max_allowed);

    let answers: Vec<(String, String)> = clean_answers
        .iter()
        .enumerate()
        .map(|(index, text)| (poll_answer_id(index, text), text.clone()))
        .collect();

    let fallback_text = format!(
        "{}\n{}",
        question,
        answers
            .iter()
            .enumerate()
            .map(|(index, (_, text))| format!("{}. {}", index + 1, text))
            .collect::<Vec<_>>()
            .join("\n")
    );

    Ok(NormalizedPoll {
        question,
        answers,
        max_selections,
        fallback_text,
    })
}

pub fn poll_start_content(
    normalized: &NormalizedPoll,
) -> Result<NewUnstablePollStartEventContent, PollSendError> {
    let answers = UnstablePollAnswers::try_from(
        normalized
            .answers
            .iter()
            .map(|(id, text)| UnstablePollAnswer::new(id.clone(), text.clone()))
            .collect::<Vec<_>>(),
    )
    .map_err(|_| PollSendError::Answers)?;

    let mut block = UnstablePollStartContentBlock::new(normalized.question.clone(), answers);
    block.kind = PollKind::Disclosed;
    block.max_selections = UInt::from(normalized.max_selections);

    Ok(NewUnstablePollStartEventContent::plain_text(
        normalized.fallback_text.clone(),
        block,
    ))
}

pub fn apply_poll_start_relations(
    content: &mut NewUnstablePollStartEventContent,
    reply_to: Option<OwnedEventId>,
    thread_root: Option<OwnedEventId>,
) {
    content.relates_to = match (thread_root, reply_to) {
        (Some(root), Some(reply)) => Some(RelationWithoutReplacement::Thread(Thread::reply(
            root, reply,
        ))),
        (Some(root), None) => Some(RelationWithoutReplacement::Thread(
            Thread::without_fallback(root),
        )),
        (None, Some(reply)) => Some(RelationWithoutReplacement::Reply(Reply::with_event_id(
            reply,
        ))),
        (None, None) => None,
    };
}

pub fn poll_response_content(
    poll_event_id: &str,
    answer_ids: &[String],
) -> Result<UnstablePollResponseEventContent, PollSendError> {
    let poll_event_id: OwnedEventId = poll_event_id
        .parse()
        .map_err(|_| PollSendError::PollEventId)?;

    let mut seen = std::collections::BTreeSet::new();
    let mut answers = Vec::new();
    for answer_id in answer_ids {
        let answer_id = answer_id.trim();
        if answer_id.is_empty() || answer_id.len() > 64 || !seen.insert(answer_id.to_owned()) {
            return Err(PollSendError::AnswerIds);
        }
        answers.push(answer_id.to_owned());
    }
    if answers.len() > MAX_POLL_SELECTIONS {
        return Err(PollSendError::AnswerIds);
    }

    Ok(UnstablePollResponseEventContent::new(
        answers,
        poll_event_id,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answer_id_matches_js_ascii_algorithm() {
        assert_eq!(poll_answer_id(0, "Alpha"), "a1_11pyri");
        assert_eq!(poll_answer_id(1, "Beta"), "a2_18avk");
    }

    #[test]
    fn normalize_requires_question_and_two_answers() {
        assert!(normalize_poll("", &["A".into(), "B".into()], 1).is_err());
        assert!(normalize_poll("Q?", &["A".into()], 1).is_err());
        let ok = normalize_poll("Q?", &["A".into(), "B".into()], 1).expect("ok");
        assert_eq!(ok.max_selections, 1);
        assert_eq!(ok.answers.len(), 2);
    }

    #[test]
    fn response_dedupes_and_rejects_blank() {
        assert!(poll_response_content("$x:example.org", &["a1".into(), "a1".into()]).is_err());
        assert!(poll_response_content("$x:example.org", &["".into()]).is_err());
        let content =
            poll_response_content("$x:example.org", &["a1".into(), "a2".into()]).expect("response");
        assert_eq!(content.poll_response.answers, vec!["a1", "a2"]);
    }
}
