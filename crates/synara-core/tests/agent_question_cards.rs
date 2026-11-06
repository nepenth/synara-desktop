//! Hermes clarifying questions ("question cards") through the real SDK
//! timeline: structured `org.hermes.clarify` content, its `m.replace`
//! settlement, the plain-text fallback, and both answer routes.

use std::{sync::Arc, time::Duration};

use matrix_sdk::test_utils::mocks::{MatrixMockServer, RoomMessagesResponseTemplate};
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, BOB};
use ruma::{event_id, room_id, serde::Raw, RoomVersionId};
use serde_json::{json, Value};
use synara_core::app::agent_questions::{
    AgentQuestionAnswer, AgentQuestionOwnAnswer, AgentQuestionSource, AgentQuestionState,
};
use synara_core::app::timeline::{
    NativeAgentQuestionAnswerRequest, NativeAgentQuestionAnswerRoute,
    NativeAgentQuestionAnswerStatus, NativeTimelineOpenPosition, NativeTimelineOpenRequest,
    NativeTimelineOwner, TimelineAgentQuestion, TimelineViewRow,
};

const FALLBACK: &str = "❓ Which database should I use?\nReply \"skip\" to skip this question.\n\n1. Postgres (Recommended)\n2. SQLite\n\nReply with the number, the option text, or your own answer.";

fn clarify(requester: &str, state: &str, answer: Value) -> Value {
    json!({
        "v": 1,
        "clarify_id": "clarify-1",
        "question": "Pick a colour",
        "choices": [
            {"index": 1, "label": "Red", "recommended": true},
            {"index": 2, "label": "Blue", "recommended": false}
        ],
        "multi_select": false,
        "allow_other": true,
        "allow_skip": true,
        "requester_user_id": requester,
        "expires_at_ms": null,
        "state": state,
        "answer": answer
    })
}

fn raw_event(value: Value) -> Raw<ruma::events::AnySyncTimelineEvent> {
    Raw::from_json_string(value.to_string()).expect("raw timeline event")
}

fn card_event(event_id: &str, ts: u64, payload: Value) -> Raw<ruma::events::AnySyncTimelineEvent> {
    raw_event(json!({
        "type": "m.room.message",
        "event_id": event_id,
        "sender": BOB.as_str(),
        "origin_server_ts": ts,
        "content": {
            "msgtype": "m.text",
            "body": "❓ Pick a colour\n\n1. Red (Recommended)\n2. Blue",
            "org.hermes.clarify": payload
        }
    }))
}

fn question<'a>(rows: &'a [TimelineViewRow], event_id: &str) -> Option<&'a TimelineAgentQuestion> {
    rows.iter().find_map(|row| match row {
        TimelineViewRow::Message(row) if row.event.event_id.as_deref() == Some(event_id) => {
            row.agent_question.as_ref()
        }
        _ => None,
    })
}

async fn wait_for_rows(
    owner: &NativeTimelineOwner,
    stream_id: &str,
    accept: impl Fn(&[TimelineViewRow]) -> bool,
) -> Vec<TimelineViewRow> {
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            if let Ok(snapshot) = owner.snapshot(stream_id).await {
                if accept(&snapshot.rows) {
                    return snapshot.rows;
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("question card rows must settle")
}

#[tokio::test]
async fn question_cards_project_answer_and_settle_through_the_sdk_timeline() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let room_id = room_id!("!question-cards:example.org");
    let own_user = client.user_id().unwrap().to_owned();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let structured = event_id!("$structured-card");
    let fallback = event_id!("$fallback-card");
    let foreign = event_id!("$foreign-card");
    let f = EventFactory::new().room(room_id);

    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f.create(&own_user, RoomVersionId::V11))
                .add_timeline_event(card_event(
                    structured.as_str(),
                    now - 3_000,
                    clarify(own_user.as_str(), "pending", Value::Null),
                ))
                .add_timeline_event(
                    f.text_msg(FALLBACK)
                        .sender(*BOB)
                        .event_id(fallback)
                        .server_ts(now - 2_000),
                )
                .add_timeline_event(card_event(
                    foreign.as_str(),
                    now - 1_000,
                    clarify("@someone-else:example.org", "pending", Value::Null),
                )),
        )
        .await;
    server.mock_room_state_encryption().plain().mount().await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default())
        .mount()
        .await;
    server
        .mock_room_send()
        .ok(event_id!("$sent-answer"))
        .mount()
        .await;

    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 41);
    let opened = owner
        .open_at(NativeTimelineOpenRequest {
            room_id: room_id.to_string(),
            position: NativeTimelineOpenPosition::LiveBottom,
        })
        .await
        .expect("open native timeline");
    let rows = wait_for_rows(&owner, &opened.stream_id, |rows| {
        [structured, fallback, foreign]
            .iter()
            .all(|id| question(rows, id.as_str()).is_some())
    })
    .await;

    let card = question(&rows, structured.as_str()).unwrap();
    assert_eq!(card.source, AgentQuestionSource::Structured);
    assert_eq!(card.question, "Pick a colour");
    assert_eq!(card.choices.len(), 2);
    assert!(card.choices[0].recommended);
    assert!(card.can_answer);
    assert_eq!(card.state, AgentQuestionState::Pending);
    assert_eq!(card.own_answer, None);
    let text_card = question(&rows, fallback.as_str()).unwrap();
    assert_eq!(text_card.source, AgentQuestionSource::Fallback);
    assert_eq!(text_card.choices[0].label, "Postgres");
    assert!(text_card.can_answer);
    assert!(!question(&rows, foreign.as_str()).unwrap().can_answer);

    // Single choice on a structured card: the keycap reaction.
    let answered = owner
        .answer_agent_question(NativeAgentQuestionAnswerRequest {
            room_id: room_id.to_string(),
            event_id: structured.to_string(),
            answer: AgentQuestionAnswer::Choice { index: 2 },
        })
        .await
        .expect("structured choice");
    assert_eq!(answered.status, NativeAgentQuestionAnswerStatus::Sent);
    assert_eq!(
        answered.route,
        Some(NativeAgentQuestionAnswerRoute::Reaction)
    );
    wait_for_rows(&owner, &opened.stream_id, |rows| {
        question(rows, structured.as_str()).and_then(|card| card.own_answer)
            == Some(AgentQuestionOwnAnswer::Choice { index: 2 })
    })
    .await;
    let again = owner
        .answer_agent_question(NativeAgentQuestionAnswerRequest {
            room_id: room_id.to_string(),
            event_id: structured.to_string(),
            answer: AgentQuestionAnswer::Skip,
        })
        .await
        .expect("second answer reads back");
    assert_eq!(
        again.status,
        NativeAgentQuestionAnswerStatus::AlreadyAnswered
    );
    assert_eq!(again.route, None);

    // Fallback card: free text goes out as a reply to the card.
    let replied = owner
        .answer_agent_question(NativeAgentQuestionAnswerRequest {
            room_id: room_id.to_string(),
            event_id: fallback.to_string(),
            answer: AgentQuestionAnswer::Other {
                text: "Use MySQL".into(),
            },
        })
        .await
        .expect("fallback reply");
    assert_eq!(replied.route, Some(NativeAgentQuestionAnswerRoute::Reply));
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let requests = server
                .server()
                .received_requests()
                .await
                .unwrap_or_default();
            let sent = requests.iter().any(|request| {
                let body: Value = serde_json::from_slice(&request.body).unwrap_or(Value::Null);
                body["body"] == "Use MySQL"
                    && body["msgtype"] == "m.text"
                    && body["m.relates_to"]["m.in_reply_to"]["event_id"] == fallback.as_str()
            });
            if sent {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("reply to the fallback card reaches the homeserver");

    // Someone else's card is read-only.
    let refused = owner
        .answer_agent_question(NativeAgentQuestionAnswerRequest {
            room_id: room_id.to_string(),
            event_id: foreign.to_string(),
            answer: AgentQuestionAnswer::Skip,
        })
        .await
        .expect_err("another requester's card");
    assert_eq!(refused, "agent-question-not-requester");

    // Hermes settles the card with `m.replace`; the latest edit's key wins.
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id).add_timeline_event(raw_event(json!({
                "type": "m.room.message",
                "event_id": "$structured-settled",
                "sender": BOB.as_str(),
                "origin_server_ts": now,
                "content": {
                    "msgtype": "m.text",
                    "body": "* Answered: Blue",
                    "m.new_content": {
                        "msgtype": "m.text",
                        "body": "Answered: Blue",
                        "org.hermes.clarify": clarify(own_user.as_str(), "answered", json!("Blue"))
                    },
                    "m.relates_to": {"rel_type": "m.replace", "event_id": structured.as_str()}
                }
            }))),
        )
        .await;
    let rows = wait_for_rows(&owner, &opened.stream_id, |rows| {
        question(rows, structured.as_str())
            .is_some_and(|card| card.state == AgentQuestionState::Answered)
    })
    .await;
    assert_eq!(
        question(&rows, structured.as_str())
            .unwrap()
            .answer
            .as_deref(),
        Some("Blue")
    );
}
