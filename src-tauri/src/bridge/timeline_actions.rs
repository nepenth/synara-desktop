//! Desktop bridges for timeline edit/redact/report/pin through `Core::command`.

use synara_core::app::timeline::NativeTimelineActionReadback;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const TIMELINE_EDIT_TEXT_COMMAND: &str = "matrix_timeline_edit_text";
const TIMELINE_REDACT_COMMAND: &str = "matrix_timeline_redact";
const TIMELINE_REPORT_COMMAND: &str = "matrix_timeline_report";
const TIMELINE_PIN_COMMAND: &str = "matrix_timeline_pin";
const TIMELINE_UNPIN_COMMAND: &str = "matrix_timeline_unpin";
const TIMELINE_POLL_VOTE_COMMAND: &str = "matrix_timeline_poll_vote";
const TIMELINE_CALL_DECLINE_COMMAND: &str = "matrix_timeline_call_decline";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn timeline_edit_text(
    core: &Core,
    room_id: String,
    event_id: String,
    body: String,
    formatted_body: Option<String>,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let mut payload = serde_json::json!({
        "roomId": room_id,
        "eventId": event_id,
        "body": body,
    });
    if let Some(formatted_body) = formatted_body {
        payload["formattedBody"] = serde_json::Value::String(formatted_body);
    }
    let response = core
        .command(CommandEnvelope {
            command: TIMELINE_EDIT_TEXT_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload,
        })
        .await
        .map_err(map_timeline_action_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| timeline_action_response_error())
}

pub(crate) async fn timeline_redact(
    core: &Core,
    room_id: String,
    event_id: String,
    reason: Option<String>,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let mut payload = serde_json::json!({
        "roomId": room_id,
        "eventId": event_id,
    });
    if let Some(reason) = reason {
        payload["reason"] = serde_json::Value::String(reason);
    }
    let response = core
        .command(CommandEnvelope {
            command: TIMELINE_REDACT_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload,
        })
        .await
        .map_err(map_timeline_action_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| timeline_action_response_error())
}

pub(crate) async fn timeline_report(
    core: &Core,
    room_id: String,
    event_id: String,
    reason: Option<String>,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let mut payload = serde_json::json!({
        "roomId": room_id,
        "eventId": event_id,
    });
    if let Some(reason) = reason {
        payload["reason"] = serde_json::Value::String(reason);
    }
    let response = core
        .command(CommandEnvelope {
            command: TIMELINE_REPORT_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload,
        })
        .await
        .map_err(map_timeline_action_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| timeline_action_response_error())
}

pub(crate) async fn timeline_pin(
    core: &Core,
    room_id: String,
    event_id: String,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    dispatch_pin(core, TIMELINE_PIN_COMMAND, room_id, event_id).await
}

pub(crate) async fn timeline_unpin(
    core: &Core,
    room_id: String,
    event_id: String,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    dispatch_pin(core, TIMELINE_UNPIN_COMMAND, room_id, event_id).await
}

async fn dispatch_pin(
    core: &Core,
    command: &str,
    room_id: String,
    event_id: String,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: command.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
            }),
        })
        .await
        .map_err(map_timeline_action_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| timeline_action_response_error())
}

pub(crate) async fn timeline_poll_vote(
    core: &Core,
    room_id: String,
    event_id: String,
    answer_ids: Vec<String>,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: TIMELINE_POLL_VOTE_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
                "answerIds": answer_ids,
            }),
        })
        .await
        .map_err(map_timeline_action_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| timeline_action_response_error())
}

pub(crate) async fn timeline_call_decline(
    core: &Core,
    room_id: String,
    event_id: String,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: TIMELINE_CALL_DECLINE_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
            }),
        })
        .await
        .map_err(map_timeline_action_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| timeline_action_response_error())
}

fn map_timeline_action_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => {
            let diagnostic = error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-timeline-edit-empty-body");
            let (code, message) = match diagnostic {
                "v-timeline-edit-room-not-found"
                | "v-timeline-redact-room-not-found"
                | "v-timeline-report-room-not-found"
                | "v-timeline-pin-room-not-found"
                | "v-timeline-unpin-room-not-found"
                | "v-timeline-poll-vote-room-not-found"
                | "v-timeline-call-decline-room-not-found" => {
                    ("NotFound", "The native Matrix room is not available.")
                }
                "v-timeline-call-decline-own-call" => (
                    "InvalidRequest",
                    "A call started by this session cannot be declined.",
                ),
                "v-timeline-call-decline-bad-event-type" => (
                    "InvalidRequest",
                    "Only m.rtc.notification events can be declined.",
                ),
                _ => (
                    "InvalidRequest",
                    "The native Matrix timeline action request is invalid.",
                ),
            };
            MatrixAuthCommandError::new(code, message, diagnostic)
        }
        _ => {
            let diagnostic = error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-timeline-edit-send-failed");
            MatrixAuthCommandError::new(
                "InvalidRequest",
                "The native Matrix timeline action request is invalid.",
                diagnostic,
            )
        }
    }
}

fn timeline_action_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix timeline action request is invalid.",
        "v-timeline-edit-send-failed",
    )
}
