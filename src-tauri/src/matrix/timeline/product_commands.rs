use super::*;

#[tauri::command]
pub async fn matrix_timeline_open(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeTimelineOpenRequest,
) -> Result<NativeTimelineOpenReadback, MatrixAuthCommandError> {
    crate::bridge::timeline_open::timeline_open(
        core.inner().as_ref(),
        request.room_id,
        request.position,
    )
    .await
}

#[tauri::command]
pub async fn matrix_timeline_close(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeTimelineCloseRequest,
) -> Result<bool, MatrixAuthCommandError> {
    crate::bridge::timeline_close::timeline_close(core.inner().as_ref(), request.stream_id).await
}

#[tauri::command]
pub async fn matrix_timeline_jump_latest(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeTimelineJumpLatestRequest,
) -> Result<NativeTimelineOpenReadback, MatrixAuthCommandError> {
    crate::bridge::timeline_open::timeline_jump_latest(core.inner().as_ref(), request.stream_id)
        .await
}

#[tauri::command]
pub async fn matrix_timeline_event_readback(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    event_id: String,
) -> Result<NativeTimelineEventReadback, MatrixAuthCommandError> {
    crate::bridge::timeline_event_readback::timeline_event_readback(
        core.inner().as_ref(),
        room_id,
        event_id,
    )
    .await
}

#[tauri::command]
pub async fn matrix_timeline_paginate(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeTimelineViewPaginationRequest,
) -> Result<crate::matrix::timeline::TimelineViewSnapshot, MatrixAuthCommandError> {
    crate::bridge::timeline_paginate::timeline_paginate(
        core.inner().as_ref(),
        request.stream_id,
        request.direction,
    )
    .await
}

#[tauri::command]
pub async fn matrix_timeline_set_read_state(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeTimelineReadStateRequest,
) -> Result<NativeTimelineReadStateReadback, MatrixAuthCommandError> {
    crate::bridge::timeline_set_read_state::timeline_set_read_state(
        core.inner().as_ref(),
        request.stream_id,
        request.action,
    )
    .await
}

#[tauri::command]
pub async fn matrix_timeline_reaction_toggle(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    event_id: String,
    key: String,
) -> Result<NativeReactionMutationResult, MatrixAuthCommandError> {
    crate::bridge::timeline_reactions::reaction_toggle(
        core.inner().as_ref(),
        room_id,
        event_id,
        key,
    )
    .await
}

#[tauri::command]
pub async fn matrix_reaction_ensure(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    event_id: String,
    key: String,
) -> Result<NativeReactionMutationResult, MatrixAuthCommandError> {
    crate::bridge::timeline_reactions::reaction_ensure(
        core.inner().as_ref(),
        room_id,
        event_id,
        key,
    )
    .await
}

#[tauri::command]
pub async fn matrix_reaction_redact(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    target_event_id: String,
    reaction_event_id: String,
    key: String,
) -> Result<NativeReactionMutationResult, MatrixAuthCommandError> {
    crate::bridge::timeline_reactions::reaction_redact(
        core.inner().as_ref(),
        room_id,
        target_event_id,
        reaction_event_id,
        key,
    )
    .await
}

#[tauri::command]
pub async fn matrix_composer_set_reply_draft(
    state: State<'_, MatrixAuthState>,
    request: NativeComposerSetReplyDraftRequest,
) -> Result<NativeComposerReplyDraftReadback, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&request.room_id)?;
    let event_id =
        parse_required_event_id(&request.event_id, "v-timeline-reply-draft-invalid-event-id")?;

    let room = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        active.client.get_room(&room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix room is not available.",
                "v-timeline-reply-draft-room-not-found",
            )
        })?
    };

    let draft = load_reply_draft_preview(&room, &event_id, request.start_thread).await?;
    let room_id_string = room_id.to_string();
    {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        active
            .composer_drafts
            .set(room_id_string.clone(), draft.clone());
    }

    Ok(reply_draft_readback(room_id_string, "set", Some(draft)))
}

#[tauri::command]
pub async fn matrix_composer_clear_reply_draft(
    state: State<'_, MatrixAuthState>,
    request: NativeComposerReplyDraftRoomRequest,
) -> Result<NativeComposerReplyDraftReadback, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&request.room_id)?;
    let room_id_string = room_id.to_string();
    {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        active.composer_drafts.clear(&room_id_string);
    }
    Ok(reply_draft_readback(room_id_string, "cleared", None))
}

#[tauri::command]
pub async fn matrix_composer_get_reply_draft(
    state: State<'_, MatrixAuthState>,
    request: NativeComposerReplyDraftRoomRequest,
) -> Result<NativeComposerReplyDraftReadback, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&request.room_id)?;
    let room_id_string = room_id.to_string();
    let draft = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        active.composer_drafts.get(&room_id_string).cloned()
    };
    Ok(reply_draft_readback(
        room_id_string,
        if draft.is_some() { "set" } else { "empty" },
        draft,
    ))
}

#[tauri::command]
pub async fn matrix_timeline_edit_text(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeTimelineEditTextRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    crate::bridge::timeline_actions::timeline_edit_text(
        core.inner().as_ref(),
        request.room_id,
        request.event_id,
        request.body,
        request.formatted_body,
    )
    .await
}

#[tauri::command]
pub async fn matrix_timeline_redact(
    core: State<'_, Arc<synara_core::Core>>,
    request: NativeTimelineRedactRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    crate::bridge::timeline_actions::timeline_redact(
        core.inner().as_ref(),
        request.room_id,
        request.event_id,
        request.reason,
    )
    .await
}

#[tauri::command]
pub async fn matrix_timeline_forward_text(
    state: State<'_, MatrixAuthState>,
    request: NativeTimelineForwardTextRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let source_room_id = parse_send_room_id(&request.source_room_id)?;
    let target_room_id = parse_send_room_id(&request.target_room_id)?;
    let event_id =
        parse_required_event_id(&request.event_id, "v-timeline-forward-invalid-event-id")?;

    let (source_room, target_room) = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        let source_room = active.client.get_room(&source_room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix source room is not available.",
                "v-timeline-forward-source-room-not-found",
            )
        })?;
        let target_room = active.client.get_room(&target_room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix target room is not available.",
                "v-timeline-forward-target-room-not-found",
            )
        })?;
        (source_room, target_room)
    };

    let (sender_label, body) = load_forwardable_text(&source_room, &event_id).await?;
    let forwarded_body = format_forwarded_plain_body(&sender_label, &body, request.as_quote);
    let content = message_content(forwarded_body, None, None, None, false, None, None)?;
    let event_id = send_message_to_room(&target_room, content, None)
        .await
        .map_err(|_| map_timeline_action_error("v-timeline-forward-send-failed"))?;

    Ok(NativeTimelineActionReadback {
        schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
        action: NativeTimelineActionKind::ForwardText,
        room_id: target_room_id.to_string(),
        event_id,
        status: "sent",
    })
}

#[tauri::command]
pub async fn matrix_timeline_forward_media(
    state: State<'_, MatrixAuthState>,
    request: NativeTimelineForwardMediaRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let source_room_id = parse_send_room_id(&request.source_room_id)?;
    let target_room_id = parse_send_room_id(&request.target_room_id)?;
    let event_id = parse_required_event_id(
        &request.event_id,
        "v-timeline-forward-media-invalid-event-id",
    )?;

    let (source_room, target_room) = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        let source_room = active.client.get_room(&source_room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix source room is not available.",
                "v-timeline-forward-media-source-room-not-found",
            )
        })?;
        let target_room = active.client.get_room(&target_room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix target room is not available.",
                "v-timeline-forward-media-target-room-not-found",
            )
        })?;
        (source_room, target_room)
    };

    let content = load_forwardable_media(&source_room, &event_id).await?;
    let event_id = target_room
        .send(content)
        .await
        .map_err(|_| map_timeline_action_error("v-timeline-forward-media-send-failed"))?
        .response
        .event_id
        .to_string();

    Ok(NativeTimelineActionReadback {
        schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
        action: NativeTimelineActionKind::ForwardMedia,
        room_id: target_room_id.to_string(),
        event_id,
        status: "sent",
    })
}

#[tauri::command]
pub async fn matrix_timeline_report(
    state: State<'_, MatrixAuthState>,
    request: NativeTimelineReportRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&request.room_id)?;
    let event_id =
        parse_required_event_id(&request.event_id, "v-timeline-report-invalid-event-id")?;
    let reason = request
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);

    let room = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        active.client.get_room(&room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix room is not available.",
                "v-timeline-report-room-not-found",
            )
        })?
    };

    room.report_content(event_id.clone(), reason)
        .await
        .map_err(|_| map_timeline_action_error("v-timeline-report-failed"))?;

    Ok(NativeTimelineActionReadback {
        schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
        action: NativeTimelineActionKind::Report,
        room_id: room_id.to_string(),
        event_id: event_id.to_string(),
        status: "reported",
    })
}

#[tauri::command]
pub async fn matrix_timeline_pin(
    state: State<'_, MatrixAuthState>,
    request: NativeTimelinePinRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    pin_or_unpin_event(state, request, true).await
}

#[tauri::command]
pub async fn matrix_timeline_unpin(
    state: State<'_, MatrixAuthState>,
    request: NativeTimelinePinRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    pin_or_unpin_event(state, request, false).await
}

#[tauri::command]
pub async fn matrix_timeline_poll_vote(
    state: State<'_, MatrixAuthState>,
    request: NativeTimelinePollVoteRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&request.room_id)?;
    let event_id =
        parse_required_event_id(&request.event_id, "v-timeline-poll-vote-invalid-event-id")?;
    let answer_ids = request
        .answer_ids
        .into_iter()
        .map(|answer| answer.trim().to_owned())
        .filter(|answer| !answer.is_empty())
        .collect::<Vec<_>>();

    let room = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        active.client.get_room(&room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix room is not available.",
                "v-timeline-poll-vote-room-not-found",
            )
        })?
    };

    let content = UnstablePollResponseEventContent::new(answer_ids, event_id.clone());
    let sent_event_id = room
        .send(content)
        .await
        .map_err(|_| map_timeline_action_error("v-timeline-poll-vote-send-failed"))?
        .response
        .event_id
        .to_string();

    Ok(NativeTimelineActionReadback {
        schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
        action: NativeTimelineActionKind::PollVote,
        room_id: room_id.to_string(),
        event_id: sent_event_id,
        status: "voted",
    })
}

#[tauri::command]
pub async fn matrix_timeline_call_decline(
    state: State<'_, MatrixAuthState>,
    request: NativeTimelineCallDeclineRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&request.room_id)?;
    let event_id = parse_required_event_id(
        &request.event_id,
        "v-timeline-call-decline-invalid-event-id",
    )?;

    let room = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        active.client.get_room(&room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix room is not available.",
                "v-timeline-call-decline-room-not-found",
            )
        })?
    };

    let content = room
        .make_decline_call_event(&event_id)
        .await
        .map_err(|error| match error {
            matrix_sdk::room::calls::CallError::DeclineOwnCall => MatrixAuthCommandError::new(
                "InvalidRequest",
                "A call started by this session cannot be declined.",
                "v-timeline-call-decline-own-call",
            ),
            matrix_sdk::room::calls::CallError::BadEventType => MatrixAuthCommandError::new(
                "InvalidRequest",
                "Only m.rtc.notification events can be declined.",
                "v-timeline-call-decline-bad-event-type",
            ),
            _ => map_timeline_action_error("v-timeline-call-decline-prepare-failed"),
        })?;
    let sent_event_id = room
        .send(content)
        .await
        .map_err(|_| map_timeline_action_error("v-timeline-call-decline-send-failed"))?
        .response
        .event_id
        .to_string();

    Ok(NativeTimelineActionReadback {
        schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
        action: NativeTimelineActionKind::CallDecline,
        room_id: room_id.to_string(),
        event_id: sent_event_id,
        status: "declined",
    })
}

pub(super) async fn pin_or_unpin_event(
    state: State<'_, MatrixAuthState>,
    request: NativeTimelinePinRequest,
    pin: bool,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&request.room_id)?;
    let event_id = parse_required_event_id(
        &request.event_id,
        if pin {
            "v-timeline-pin-invalid-event-id"
        } else {
            "v-timeline-unpin-invalid-event-id"
        },
    )?;

    let room = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        active.client.get_room(&room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix room is not available.",
                if pin {
                    "v-timeline-pin-room-not-found"
                } else {
                    "v-timeline-unpin-room-not-found"
                },
            )
        })?
    };

    let changed = if pin {
        room.pin_event(&event_id)
            .await
            .map_err(|_| map_timeline_action_error("v-timeline-pin-failed"))?
    } else {
        room.unpin_event(&event_id)
            .await
            .map_err(|_| map_timeline_action_error("v-timeline-unpin-failed"))?
    };

    Ok(NativeTimelineActionReadback {
        schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
        action: if pin {
            NativeTimelineActionKind::Pin
        } else {
            NativeTimelineActionKind::Unpin
        },
        room_id: room_id.to_string(),
        event_id: event_id.to_string(),
        status: if changed {
            if pin {
                "pinned"
            } else {
                "unpinned"
            }
        } else if pin {
            "already_pinned"
        } else {
            "already_unpinned"
        },
    })
}

pub(super) fn map_timeline_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "d0.3-timeline-invalid-room-id" => (
            "InvalidRequest",
            "The native Matrix timeline request is invalid.",
        ),
        "d0.3-timeline-room-not-found" | "d0.3-timeline-not-open" => {
            ("NotFound", "The native Matrix timeline is not available.")
        }
        _ => ("Unknown", "The native Matrix timeline is unavailable."),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

pub(super) fn map_reaction_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let code = if diagnostic_id.contains("invalid") {
        "InvalidRequest"
    } else {
        "Unknown"
    };
    MatrixAuthCommandError::new(
        code,
        "The native Matrix reaction operation could not be completed.",
        diagnostic_id,
    )
}

pub(super) fn require_send_session_mut(
    session: Option<&mut ManagedMatrixSession>,
) -> Result<&mut ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        )
    })
}

pub(super) fn map_send_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "InvalidRequest",
        "The native Matrix send request is invalid.",
        diagnostic_id,
    )
}

pub(super) fn parse_send_room_id(room_id: &str) -> Result<OwnedRoomId, MatrixAuthCommandError> {
    room_id
        .parse()
        .map_err(|_| map_send_error("d0.4-send-invalid-room-id"))
}

pub(super) fn parse_reply_event_id(
    reply_to: Option<String>,
) -> Result<Option<OwnedEventId>, MatrixAuthCommandError> {
    reply_to
        .map(|event_id| {
            event_id
                .parse()
                .map_err(|_| map_send_error("d0.4-send-invalid-reply-event-id"))
        })
        .transpose()
}

pub(super) fn parse_required_event_id(
    event_id: &str,
    diagnostic_id: &'static str,
) -> Result<OwnedEventId, MatrixAuthCommandError> {
    event_id
        .trim()
        .parse()
        .map_err(|_| map_timeline_action_error(diagnostic_id))
}

pub(super) fn map_timeline_action_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "InvalidRequest",
        "The native Matrix timeline action request is invalid.",
        diagnostic_id,
    )
}

pub(super) async fn load_forwardable_text(
    room: &Room,
    event_id: &EventId,
) -> Result<(String, String), MatrixAuthCommandError> {
    let timeline_event = room
        .load_or_fetch_event(event_id, None)
        .await
        .map_err(|_| map_timeline_action_error("v-timeline-forward-event-unavailable"))?;
    let sync_event = timeline_event
        .raw()
        .deserialize()
        .map_err(|_| map_timeline_action_error("v-timeline-forward-event-decode-failed"))?;
    match sync_event {
        AnySyncTimelineEvent::MessageLike(AnySyncMessageLikeEvent::RoomMessage(message)) => {
            let original = message
                .as_original()
                .ok_or_else(|| map_timeline_action_error("v-timeline-forward-event-redacted"))?;
            Ok((
                original.sender.to_string(),
                original.content.body().to_owned(),
            ))
        }
        _ => Err(map_timeline_action_error(
            "v-timeline-forward-unsupported-event",
        )),
    }
}

pub(super) async fn load_forwardable_media(
    room: &Room,
    event_id: &EventId,
) -> Result<AnyMessageLikeEventContent, MatrixAuthCommandError> {
    let timeline_event = room
        .load_or_fetch_event(event_id, None)
        .await
        .map_err(|_| map_timeline_action_error("v-timeline-forward-media-event-unavailable"))?;
    let sync_event = timeline_event
        .raw()
        .deserialize()
        .map_err(|_| map_timeline_action_error("v-timeline-forward-media-event-decode-failed"))?;
    match sync_event {
        AnySyncTimelineEvent::MessageLike(AnySyncMessageLikeEvent::RoomMessage(message)) => {
            let original = message.as_original().ok_or_else(|| {
                map_timeline_action_error("v-timeline-forward-media-event-redacted")
            })?;
            let sender = original.sender.to_string();
            let mut msgtype = original.content.msgtype.clone();
            match &mut msgtype {
                MessageType::Image(content) => {
                    content.body = format_forwarded_media_body(&sender, &content.body);
                }
                MessageType::File(content) => {
                    content.body = format_forwarded_media_body(&sender, &content.body);
                }
                MessageType::Audio(content) => {
                    content.body = format_forwarded_media_body(&sender, &content.body);
                }
                MessageType::Video(content) => {
                    content.body = format_forwarded_media_body(&sender, &content.body);
                }
                _ => {
                    return Err(map_timeline_action_error(
                        "v-timeline-forward-media-unsupported-event",
                    ));
                }
            }
            Ok(AnyMessageLikeEventContent::RoomMessage(
                RoomMessageEventContent::new(msgtype),
            ))
        }
        AnySyncTimelineEvent::MessageLike(AnySyncMessageLikeEvent::Sticker(sticker)) => {
            let original = sticker.as_original().ok_or_else(|| {
                map_timeline_action_error("v-timeline-forward-media-event-redacted")
            })?;
            let sender = original.sender.to_string();
            Ok(AnyMessageLikeEventContent::Sticker(
                StickerEventContent::with_source(
                    format_forwarded_media_body(&sender, &original.content.body),
                    original.content.info.clone(),
                    original.content.source.clone(),
                ),
            ))
        }
        _ => Err(map_timeline_action_error(
            "v-timeline-forward-media-unsupported-event",
        )),
    }
}

pub(super) async fn load_reply_draft_preview(
    room: &Room,
    event_id: &EventId,
    start_thread: bool,
) -> Result<NativeComposerReplyDraft, MatrixAuthCommandError> {
    let timeline_event = room
        .load_or_fetch_event(event_id, None)
        .await
        .map_err(|_| map_timeline_action_error("v-timeline-reply-draft-event-unavailable"))?;
    let sync_event = timeline_event
        .raw()
        .deserialize()
        .map_err(|_| map_timeline_action_error("v-timeline-reply-draft-event-decode-failed"))?;
    match sync_event {
        AnySyncTimelineEvent::MessageLike(AnySyncMessageLikeEvent::RoomMessage(message)) => {
            let original = message.as_original().ok_or_else(|| {
                map_timeline_action_error("v-timeline-reply-draft-event-redacted")
            })?;
            let body = original.content.body().to_owned();
            let formatted_body = match original.content.msgtype {
                MessageType::Text(ref content) => content.formatted.as_ref(),
                MessageType::Notice(ref content) => content.formatted.as_ref(),
                MessageType::Emote(ref content) => content.formatted.as_ref(),
                _ => None,
            }
            .filter(|formatted| formatted.format == MessageFormat::Html)
            .map(|formatted| formatted.body.trim().to_owned())
            .filter(|html| !html.is_empty() && html != body.trim());
            let existing_thread_root = match &original.content.relates_to {
                Some(Relation::Thread(thread)) => Some(thread.event_id.to_string()),
                _ => None,
            };
            let thread_root_event_id = if start_thread {
                Some(event_id.to_string())
            } else {
                existing_thread_root
            };
            Ok(NativeComposerReplyDraft {
                event_id: event_id.to_string(),
                sender_id: original.sender.to_string(),
                body,
                formatted_body,
                thread_root_event_id,
            })
        }
        _ => Err(map_timeline_action_error(
            "v-timeline-reply-draft-unsupported-event",
        )),
    }
}
