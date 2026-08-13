use super::*;

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Stable Tauri IPC fields are intentionally explicit.
pub async fn matrix_send_text(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    body: String,
    msg_type: Option<String>,
    formatted_body: Option<String>,
    mention_user_ids: Option<Vec<String>>,
    mention_room: Option<bool>,
    reply_to: Option<String>,
    // Thread root (`m.thread`). With reply_to → Thread::reply (is_falling_back false).
    thread_root: Option<String>,
    txn_id: Option<String>,
) -> Result<MatrixSendTextResult, MatrixAuthCommandError> {
    crate::bridge::send_text::send_text(
        core.inner().as_ref(),
        room_id,
        body,
        msg_type,
        formatted_body,
        mention_user_ids,
        mention_room,
        reply_to,
        thread_root,
        txn_id,
    )
    .await
}

/// V-SEND.R-EDIT sole native message-edit owner.
///
/// Sends a replacement (`m.replace`) room message via the live matrix-sdk session.
/// The new content is built with `m.new_content` semantics matching Element/Cinny
/// (fallback body `* {plain}`; real body/html/mentions live in `m.new_content`).
/// The JS `mx.sendMessage` edit path is only used when no native session is live;
/// when a native session is live this command is the sole owner and failures are
/// fail-closed (no silent fallthrough to `mx.sendMessage`).
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Stable Tauri IPC fields are intentionally explicit.
pub async fn matrix_edit_message(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    event_id: String,
    body: String,
    msg_type: Option<String>,
    formatted_body: Option<String>,
    mention_user_ids: Option<Vec<String>>,
    mention_room: Option<bool>,
    txn_id: Option<String>,
) -> Result<MatrixSendTextResult, MatrixAuthCommandError> {
    crate::bridge::send_text::edit_message(
        core.inner().as_ref(),
        room_id,
        event_id,
        body,
        msg_type,
        formatted_body,
        mention_user_ids,
        mention_room,
        txn_id,
    )
    .await
}

/// V-SEND.1 sole composer attachment upload+send owner. Bytes cross IPC once;
/// encrypted rooms are encrypted by the managed SDK (no JS dual-encrypt).
/// V-SEND.5 extends the same command with optional `thread_root` so native
/// sessions can start / continue threads without JS relation ownership.
#[tauri::command]
pub async fn matrix_send_attachment(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    filename: String,
    mime_type: String,
    bytes: Vec<u8>,
    reply_to: Option<String>,
    thread_root: Option<String>,
) -> Result<MatrixSendAttachmentResult, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&room_id)?;
    let reply_to = parse_reply_event_id(reply_to)?;
    let thread_root = parse_thread_root_event_id(thread_root)?;
    let filename = validate_attachment_filename(&filename)?;
    let mime_type = validate_attachment_mime(&mime_type)?;
    if bytes.is_empty() {
        return Err(map_attachment_error("v-send.1-attachment-empty"));
    }
    if bytes.len() > MAX_ATTACHMENT_IPC_BYTES {
        return Err(map_attachment_error("v-send.1-attachment-too-large"));
    }
    let size_bytes = bytes.len() as u64;
    let kind = attachment_kind_for_mime(&mime_type);
    let media_handle_id = format!("native-staged:{filename}");

    let (room, session_generation, local_txn_id) = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        let room = active.client.get_room(&room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix room is not available.",
                "v-send.1-attachment-room-not-found",
            )
        })?;
        let session_generation = active.attachments.session_generation();
        let item = active
            .attachments
            .enqueue(AttachmentEnqueue {
                room_id: room_id.to_string(),
                kind,
                media_handle_id,
                file_name: Some(filename.clone()),
                caption: None,
                mime_type: Some(mime_type.to_string()),
                size_bytes: Some(size_bytes),
            })
            .map_err(|error| map_attachment_error(error.diagnostic_id()))?;
        (room, session_generation, item.local_txn_id.clone())
    };

    let send_result =
        send_attachment_to_room(&room, &filename, &mime_type, bytes, reply_to, thread_root).await;

    let mut session = state.session.lock().await;
    if let Some(active) = session.as_mut() {
        if active.attachments.session_generation() == session_generation {
            if send_result.is_ok() {
                let _ = active.attachments.mark_sent(&local_txn_id);
            } else {
                let _ = active
                    .attachments
                    .mark_failed(&local_txn_id, "v-send.1-attachment-sdk-failed");
            }
        }
    }

    let event_id = send_result.map_err(|_| {
        MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix attachment could not be sent.",
            "v-send.1-attachment-sdk-failed",
        )
    })?;
    Ok(MatrixSendAttachmentResult {
        room_id: room_id.to_string(),
        event_id,
        local_txn_id,
        status: "sent",
    })
}

/// V-SEND sticker residual — sole `m.sticker` owner for native sessions.
/// Media is already on the homeserver as an MXC (image-pack sticker); this
/// command does not re-upload bytes. Optional info fields preserve dimensions
/// when the product already knows them; empty info is valid.
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Stable Tauri IPC fields are intentionally explicit.
pub async fn matrix_send_sticker(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    body: String,
    mxc: String,
    width: Option<u64>,
    height: Option<u64>,
    mimetype: Option<String>,
    size: Option<u64>,
    reply_to: Option<String>,
    thread_root: Option<String>,
) -> Result<MatrixSendStickerResult, MatrixAuthCommandError> {
    crate::bridge::send_sticker::send_sticker(
        core.inner().as_ref(),
        room_id,
        body,
        mxc,
        width,
        height,
        mimetype,
        size,
        reply_to,
        thread_root,
    )
    .await
}

/// V-SEND.3 sole poll-start owner (composer board + `/poll` command).
#[tauri::command]
pub async fn matrix_send_poll(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    question: String,
    answers: Vec<String>,
    max_selections: u32,
    // Thread root (`m.thread`). With reply_to → Thread::reply (is_falling_back false).
    thread_root: Option<String>,
    reply_to: Option<String>,
) -> Result<MatrixSendPollResult, MatrixAuthCommandError> {
    crate::bridge::send_poll::send_poll(
        core.inner().as_ref(),
        room_id,
        question,
        answers,
        max_selections,
        thread_root,
        reply_to,
    )
    .await
}

/// V-SEND.3 sole poll-response (vote) owner.
#[tauri::command]
pub async fn matrix_poll_respond(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    poll_event_id: String,
    answer_ids: Vec<String>,
) -> Result<MatrixPollRespondResult, MatrixAuthCommandError> {
    crate::bridge::send_poll::poll_respond(
        core.inner().as_ref(),
        room_id,
        poll_event_id,
        answer_ids,
    )
    .await
}

pub(super) fn map_poll_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    match diagnostic_id {
        "v-send.3-poll-invalid-question"
        | "v-send.3-poll-invalid-answers"
        | "v-send.3-poll-invalid-max-selections"
        | "v-send.3-poll-invalid-event-id"
        | "v-send.3-poll-invalid-answer-ids" => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix poll request is invalid.",
            diagnostic_id,
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix poll operation failed.",
            diagnostic_id,
        ),
    }
}

pub(super) fn parse_thread_root_event_id(
    thread_root: Option<String>,
) -> Result<Option<OwnedEventId>, MatrixAuthCommandError> {
    thread_root
        .map(|event_id| {
            event_id
                .parse()
                .map_err(|_| map_send_error("v-send.5-invalid-thread-root-event-id"))
        })
        .transpose()
}

pub(super) fn parse_edit_event_id(
    event_id: String,
) -> Result<OwnedEventId, MatrixAuthCommandError> {
    synara_core::app::send::parse_edit_event_id(&event_id).map_err(map_send_error)
}

pub(super) fn parse_transaction_id(
    txn_id: Option<String>,
) -> Result<Option<OwnedTransactionId>, MatrixAuthCommandError> {
    txn_id
        .map(|txn_id| {
            if txn_id.is_empty() || txn_id.len() > 255 {
                return Err(map_send_error("d0.4-send-invalid-transaction-id"));
            }
            Ok(OwnedTransactionId::from(txn_id))
        })
        .transpose()
}

pub(super) fn normalize_formatted_body(
    body: &str,
    formatted_body: Option<&str>,
) -> Result<Option<String>, MatrixAuthCommandError> {
    let Some(html) = formatted_body
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    if html.len() > 65_536 {
        return Err(map_send_error("d0.4-send-formatted-body-too-large"));
    }
    if !should_attach_formatted_body(body, Some(html)) {
        return Ok(None);
    }
    Ok(Some(html.to_owned()))
}

/// Build validated `m.sticker` content for the native sticker owner.
///
/// Relation rules match text/attachment (V-SEND.5):
/// - `thread_root` + `reply_to` → `m.thread` with genuine in-thread reply
/// - `thread_root` only → `m.thread` without in-reply fallback
/// - `reply_to` only → classic `m.in_reply_to` reply
#[allow(clippy::too_many_arguments)]
pub(crate) fn sticker_content(
    body: String,
    mxc: String,
    width: Option<u64>,
    height: Option<u64>,
    mimetype: Option<String>,
    size: Option<u64>,
    reply_to: Option<OwnedEventId>,
    thread_root: Option<OwnedEventId>,
) -> Result<StickerEventContent, MatrixAuthCommandError> {
    synara_core::app::send::sticker_content(
        body,
        mxc,
        width,
        height,
        mimetype,
        size,
        reply_to,
        thread_root,
    )
    .map_err(map_send_error)
}

/// Build validated room-message content for the native composer owner.
///
/// Relation rules (V-SEND.4 + V-SEND.5):
/// - `thread_root` + `reply_to` → `m.thread` with genuine in-thread reply
///   (`is_falling_back: false`); root and reply ids may be equal when starting
///   a thread from the root event.
/// - `thread_root` only → `m.thread` without in-reply fallback.
/// - `reply_to` only → classic `m.in_reply_to` reply (no thread).
pub(crate) fn message_content(
    body: String,
    msg_type: Option<String>,
    formatted_body: Option<String>,
    mention_user_ids: Option<Vec<String>>,
    mention_room: bool,
    reply_to: Option<OwnedEventId>,
    thread_root: Option<OwnedEventId>,
) -> Result<RoomMessageEventContent, MatrixAuthCommandError> {
    synara_core::app::send::message_content(
        body,
        msg_type,
        formatted_body,
        mention_user_ids,
        mention_room,
        reply_to,
        thread_root,
    )
    .map_err(|diagnostic| match diagnostic {
        "v-send.4-invalid-message-type" => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix message type is invalid.",
            diagnostic,
        ),
        "v-send.4-invalid-mention-user-id" => MatrixAuthCommandError::new(
            "InvalidRequest",
            "A native Matrix mention user ID is invalid.",
            diagnostic,
        ),
        _ => map_send_error(diagnostic),
    })
}

/// Build validated `m.replace` replacement content for the native edit owner.
///
/// The new content is built via `message_content` (msg_type / formatted_body /
/// mentions), then wrapped with `make_replacement` so the real body/html/mentions
/// live in `m.new_content` and the fallback body is `* {plain}` (Element/Cinny
/// style). `make_replacement` strips any reply/thread relation and sets
/// `m.relates_to.rel_type == m.replace` with the target `event_id`.
pub(crate) fn edit_message_content(
    body: String,
    msg_type: Option<String>,
    formatted_body: Option<String>,
    mention_user_ids: Option<Vec<String>>,
    mention_room: bool,
    event_id: OwnedEventId,
) -> Result<RoomMessageEventContent, MatrixAuthCommandError> {
    synara_core::app::send::edit_message_content(
        body,
        msg_type,
        formatted_body,
        mention_user_ids,
        mention_room,
        event_id,
    )
    .map_err(|diagnostic| match diagnostic {
        "v-send.4-invalid-message-type" => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix message type is invalid.",
            diagnostic,
        ),
        "v-send.4-invalid-mention-user-id" => MatrixAuthCommandError::new(
            "InvalidRequest",
            "A native Matrix mention user ID is invalid.",
            diagnostic,
        ),
        _ => map_send_error(diagnostic),
    })
}

pub(super) async fn send_message_to_room(
    room: &Room,
    content: RoomMessageEventContent,
    txn_id: Option<OwnedTransactionId>,
) -> matrix_sdk::Result<String> {
    let send = room.send(content);
    let result = match txn_id {
        Some(txn_id) => send.with_transaction_id(txn_id).await?,
        None => send.await?,
    };
    Ok(result.response.event_id.to_string())
}

pub(super) fn map_attachment_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    match diagnostic_id {
        "v-send.1-attachment-empty"
        | "v-send.1-attachment-invalid-filename"
        | "v-send.1-attachment-invalid-mime"
        | "p7.4-invalid-room-id"
        | "p7.4-empty-media-handle"
        | "p7.4-file-name-cap"
        | "p7.4-file-too-large"
        | "p7.4-forbidden-handle-scheme"
        | "p7.4-forbidden-handle" => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix attachment request is invalid.",
            diagnostic_id,
        ),
        "v-send.1-attachment-too-large" | "p7.4-active-attachment-cap" => {
            MatrixAuthCommandError::new(
                "InvalidRequest",
                "The native Matrix attachment exceeds the allowed size or concurrency.",
                diagnostic_id,
            )
        }
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix attachment could not be sent.",
            diagnostic_id,
        ),
    }
}

pub(super) fn validate_attachment_filename(
    filename: &str,
) -> Result<String, MatrixAuthCommandError> {
    let filename = filename.trim();
    if filename.is_empty() || filename.chars().count() > 255 {
        return Err(map_attachment_error("v-send.1-attachment-invalid-filename"));
    }
    if filename.contains('/') || filename.contains('\\') || filename.contains('\0') {
        return Err(map_attachment_error("v-send.1-attachment-invalid-filename"));
    }
    Ok(filename.to_owned())
}

pub(super) fn validate_attachment_mime(mime_type: &str) -> Result<Mime, MatrixAuthCommandError> {
    let mime_type = mime_type.trim();
    if mime_type.is_empty() || mime_type.len() > 255 {
        return Err(map_attachment_error("v-send.1-attachment-invalid-mime"));
    }
    mime_type
        .parse::<Mime>()
        .map_err(|_| map_attachment_error("v-send.1-attachment-invalid-mime"))
}

pub(super) fn attachment_kind_for_mime(mime: &Mime) -> AttachmentKind {
    match mime.type_() {
        mime::IMAGE => AttachmentKind::Image,
        mime::VIDEO => AttachmentKind::Video,
        mime::AUDIO => AttachmentKind::Audio,
        _ => AttachmentKind::File,
    }
}

pub(super) async fn send_attachment_to_room(
    room: &Room,
    filename: &str,
    mime_type: &Mime,
    data: Vec<u8>,
    reply_to: Option<OwnedEventId>,
    thread_root: Option<OwnedEventId>,
) -> matrix_sdk::Result<String> {
    let mut config = AttachmentConfig::new();
    if let Some(event_id) = reply_to {
        // Explicit thread root from the product draft forces a thread relation
        // (start thread / reply in thread). Otherwise preserve the prior
        // MaybeThreaded behavior so existing non-thread replies keep working.
        let enforce_thread = if thread_root.is_some() {
            EnforceThread::Threaded(ReplyWithinThread::Yes)
        } else {
            EnforceThread::MaybeThreaded
        };
        config = config.reply(Some(AttachmentReply {
            event_id,
            enforce_thread,
            add_mentions: AddMentions::Yes,
        }));
    }
    let response = room
        .send_attachment(filename, mime_type, data, config)
        .await?;
    Ok(response.event_id.to_string())
}
