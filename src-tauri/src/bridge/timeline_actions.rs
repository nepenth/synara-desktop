//! Desktop bridges for timeline edit/redact/report/pin through `Core::command`.

use synara_core::app::timeline::{NativeTimelineActionReadback, PinnedEventsSnapshot};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn timeline_edit_text(
    core: &Core,
    room_id: String,
    event_id: String,
    body: String,
    formatted_body: Option<String>,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let response = core
        .timeline_edit_text(synara_core::core_api::MatrixTimelineEditTextRequest {
            room_id,
            event_id,
            body,
            formatted_body,
        })
        .await
        .map_err(map_timeline_action_core_error)?;
    Ok(response)
}

pub(crate) async fn timeline_redact(
    core: &Core,
    room_id: String,
    event_id: String,
    reason: Option<String>,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let response = core
        .timeline_redact(synara_core::core_api::MatrixTimelineRedactRequest {
            room_id,
            event_id,
            reason,
        })
        .await
        .map_err(map_timeline_action_core_error)?;
    Ok(response)
}

pub(crate) async fn timeline_report(
    core: &Core,
    room_id: String,
    event_id: String,
    reason: Option<String>,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let response = core
        .timeline_report(synara_core::core_api::MatrixTimelineReportRequest {
            room_id,
            event_id,
            reason,
        })
        .await
        .map_err(map_timeline_action_core_error)?;
    Ok(response)
}

pub(crate) async fn timeline_pin(
    core: &Core,
    room_id: String,
    event_id: String,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    core.timeline_pin(synara_core::core_api::MatrixTimelinePinRequest { room_id, event_id })
        .await
        .map_err(map_timeline_action_core_error)
}

pub(crate) async fn timeline_unpin(
    core: &Core,
    room_id: String,
    event_id: String,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    core.timeline_unpin(synara_core::core_api::MatrixTimelinePinRequest { room_id, event_id })
        .await
        .map_err(map_timeline_action_core_error)
}

pub(crate) async fn pinned_events(
    core: &Core,
    room_id: String,
) -> Result<PinnedEventsSnapshot, MatrixAuthCommandError> {
    let response = core
        .pinned_events(synara_core::core_api::MatrixPinnedEventsRequest { room_id })
        .await
        .map_err(map_timeline_action_core_error)?;
    Ok(response)
}

pub(crate) async fn timeline_poll_vote(
    core: &Core,
    room_id: String,
    event_id: String,
    answer_ids: Vec<String>,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let response = core
        .timeline_poll_vote(synara_core::core_api::MatrixTimelinePollVoteRequest {
            room_id,
            event_id,
            answer_ids,
        })
        .await
        .map_err(map_timeline_action_core_error)?;
    Ok(response)
}

pub(crate) async fn timeline_call_decline(
    core: &Core,
    room_id: String,
    event_id: String,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let response = core
        .timeline_call_decline(synara_core::core_api::MatrixTimelineCallDeclineRequest {
            room_id,
            event_id,
        })
        .await
        .map_err(map_timeline_action_core_error)?;
    Ok(response)
}

pub(crate) async fn timeline_forward_text(
    core: &Core,
    source_room_id: String,
    event_id: String,
    target_room_id: String,
    as_quote: bool,
    confirmed_encryption_downgrade: bool,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let response = core
        .timeline_forward_text(synara_core::core_api::MatrixTimelineForwardTextRequest {
            source_room_id,
            event_id,
            target_room_id,
            as_quote,
            confirmed_encryption_downgrade,
        })
        .await
        .map_err(map_timeline_action_core_error)?;
    Ok(response)
}

pub(crate) async fn timeline_forward_media(
    core: &Core,
    source_room_id: String,
    event_id: String,
    target_room_id: String,
    confirmed_encryption_downgrade: bool,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let response = core
        .timeline_forward_media(synara_core::core_api::MatrixTimelineForwardMediaRequest {
            source_room_id,
            event_id,
            target_room_id,
            confirmed_encryption_downgrade,
        })
        .await
        .map_err(map_timeline_action_core_error)?;
    Ok(response)
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
                | "v-timeline-pinned-room-not-found"
                | "v-timeline-poll-vote-room-not-found"
                | "v-timeline-call-decline-room-not-found"
                | "v-timeline-forward-source-room-not-found"
                | "v-timeline-forward-target-room-not-found"
                | "v-timeline-forward-media-source-room-not-found"
                | "v-timeline-forward-media-target-room-not-found" => {
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
                "v-timeline-forward-source-encryption-unavailable"
                | "v-timeline-forward-target-encryption-unavailable" => (
                    "Unavailable",
                    "Room encryption status is unavailable. Forwarding was not started.",
                ),
                "v-timeline-forward-encryption-downgrade-not-confirmed" => (
                    "ConfirmationRequired",
                    "Confirm before forwarding from an encrypted room to an unencrypted room.",
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
