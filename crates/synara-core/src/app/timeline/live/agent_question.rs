//! Owner route for answering Hermes clarifying questions (question cards).
//!
//! The presenter names the card and the answer; Core resolves that exact
//! event, re-recognises the question from its latest content, enforces the
//! requester and pending state, and sends either a keycap/skip reaction or a
//! reply message. It never scans the room for a similar prompt.

use matrix_sdk_ui::timeline::{
    MsgLikeKind, TimelineItem as SdkTimelineItem, TimelineItemContent as SdkTimelineItemContent,
};
use tokio::time::{timeout, Duration};

use crate::app::agent_questions::{
    own_reaction_answer, plan_agent_question_answer, recognize_agent_question, AgentQuestion,
    AgentQuestionAnswerPlan,
};

use crate::app::timeline::{
    NativeAgentQuestionAnswerRequest, NativeAgentQuestionAnswerResult,
    NativeAgentQuestionAnswerRoute, NativeAgentQuestionAnswerStatus,
};

use super::{agent_approval_now_ms, parse_event_id, parse_room_id, NativeTimelineOwner, Slots};

/// The recognised card plus what the answer route needs from the event.
struct QuestionTarget {
    question: AgentQuestion,
    sender: String,
    thread_root: Option<String>,
    already_answered: bool,
}

fn question_target(item: &SdkTimelineItem, current_user_id: &str) -> Option<QuestionTarget> {
    let event = item.as_event()?;
    let SdkTimelineItemContent::MsgLike(content) = event.content() else {
        return None;
    };
    let MsgLikeKind::Message(message) = &content.kind else {
        return None;
    };
    if !matches!(
        message.msgtype(),
        matrix_sdk::ruma::events::room::message::MessageType::Text(_)
            | matrix_sdk::ruma::events::room::message::MessageType::Notice(_)
    ) {
        return None;
    }
    let question = recognize_agent_question(
        event.latest_json().map(|raw| raw.json().get()),
        message.body(),
    )?;
    let already_answered = own_reaction_answer(
        &question,
        content.reactions.iter().filter_map(|(key, senders)| {
            senders
                .iter()
                .find(|(sender, _)| sender.as_str() == current_user_id)
                .map(|(_, info)| (key.as_str(), Some(u64::from(info.timestamp.get()))))
        }),
    )
    .is_some();
    Some(QuestionTarget {
        question,
        sender: event.sender().to_string(),
        thread_root: content.thread_root.as_ref().map(ToString::to_string),
        already_answered,
    })
}

impl NativeTimelineOwner {
    pub async fn answer_agent_question(
        &self,
        request: NativeAgentQuestionAnswerRequest,
    ) -> Result<NativeAgentQuestionAnswerResult, &'static str> {
        let parsed_room = parse_room_id(&request.room_id)?;
        let room_id = parsed_room.to_string();
        let event_id = parse_event_id(&request.event_id)?;
        let room = self
            .client
            .get_room(&parsed_room)
            .ok_or("v-crypto.6-event-room-not-found")?;
        let current_user_id = self
            .client
            .user_id()
            .ok_or("agent-question-current-user-missing")?
            .to_string();
        // The open room (or a thread or permalink window) usually renders the
        // card already; otherwise its focused window is built once.
        let target = timeout(Duration::from_secs(3), async {
            let (_, item) = Slots::Shared(self)
                .rendered_event(&room, &room_id, &event_id)
                .await
                .ok_or("v-crypto.6-event-open-failed")?;
            question_target(&item, &current_user_id).ok_or("agent-question-invalid")
        })
        .await
        .map_err(|_| "v-crypto.6-event-open-timeout")??;

        if target.already_answered {
            return Ok(NativeAgentQuestionAnswerResult {
                room_id,
                event_id: event_id.to_string(),
                status: NativeAgentQuestionAnswerStatus::AlreadyAnswered,
                route: None,
            });
        }
        let plan = plan_agent_question_answer(
            &target.question,
            &request.answer,
            &target.sender,
            &current_user_id,
            agent_approval_now_ms()?,
        )?;
        let route = match plan {
            AgentQuestionAnswerPlan::Reaction(key) => {
                self.ensure_reaction(&room_id, event_id.as_str(), key)
                    .await?;
                NativeAgentQuestionAnswerRoute::Reaction
            }
            AgentQuestionAnswerPlan::Reply(body) => {
                // Queued like a composer send: the reply survives a relaunch
                // and its row shows Sending / Failed with Retry and Discard.
                self.send_text(
                    room_id.clone(),
                    body,
                    Some("m.text".to_owned()),
                    None,
                    None,
                    None,
                    Some(event_id.to_string()),
                    target.thread_root,
                    None,
                )
                .await?;
                NativeAgentQuestionAnswerRoute::Reply
            }
        };
        Ok(NativeAgentQuestionAnswerResult {
            room_id,
            event_id: event_id.to_string(),
            status: NativeAgentQuestionAnswerStatus::Sent,
            route: Some(route),
        })
    }
}
