//! Live room attachment send. Bytes stay method arguments, never Core JSON.

use matrix_sdk::attachment::AttachmentConfig;
use matrix_sdk::room::reply::{EnforceThread, Reply as AttachmentReply};
use matrix_sdk::ruma::events::room::message::{AddMentions, ReplyWithinThread};
use matrix_sdk::{Client, Room};
use mime::Mime;

use super::text::{parse_reply_event_id, parse_send_room_id, parse_thread_root_event_id};
use super::MatrixSendRoomAttachmentResult;

/// Same IPC cap as desktop composer attachments (`MAX_ATTACHMENT_IPC_BYTES`).
pub const MAX_ATTACHMENT_UPLOAD_BYTES: usize = 32 * 1024 * 1024;

pub fn validate_attachment_filename(filename: &str) -> Result<&str, &'static str> {
    let filename = filename.trim();
    if filename.is_empty() || filename.chars().count() > 255 {
        return Err("v-send.1-attachment-invalid-filename");
    }
    if filename.contains('/') || filename.contains('\\') || filename.contains('\0') {
        return Err("v-send.1-attachment-invalid-filename");
    }
    Ok(filename)
}

pub fn validate_attachment_mime(mime_type: &str) -> Result<Mime, &'static str> {
    let mime_type = mime_type.trim();
    if mime_type.is_empty() || mime_type.len() > 255 {
        return Err("v-send.1-attachment-invalid-mime");
    }
    mime_type
        .parse::<Mime>()
        .map_err(|_| "v-send.1-attachment-invalid-mime")
}

pub async fn send_attachment_to_room(
    room: &Room,
    filename: &str,
    mime_type: &Mime,
    data: Vec<u8>,
    reply_to: Option<matrix_sdk::ruma::OwnedEventId>,
    thread_root: Option<matrix_sdk::ruma::OwnedEventId>,
) -> Result<String, &'static str> {
    let mut config = AttachmentConfig::new();
    if let Some(event_id) = reply_to {
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
        .await
        .map_err(|_| "v-send.1-attachment-sdk-failed")?;
    Ok(response.event_id.to_string())
}

pub async fn send_room_attachment(
    client: &Client,
    room_id: &str,
    filename: &str,
    mime_type: &str,
    payload: Vec<u8>,
    reply_to: Option<String>,
    thread_root: Option<String>,
) -> Result<MatrixSendRoomAttachmentResult, &'static str> {
    let parsed_room =
        parse_send_room_id(room_id).map_err(|_| "v-send.1-attachment-invalid-room")?;
    let reply_to =
        parse_reply_event_id(reply_to).map_err(|_| "v-send.1-attachment-invalid-reply")?;
    let thread_root = parse_thread_root_event_id(thread_root)
        .map_err(|_| "v-send.1-attachment-invalid-thread-root")?;
    let filename = validate_attachment_filename(filename)?;
    let mime_type = validate_attachment_mime(mime_type)?;
    if payload.is_empty() {
        return Err("v-send.1-attachment-empty");
    }
    if payload.len() > MAX_ATTACHMENT_UPLOAD_BYTES {
        return Err("v-send.1-attachment-too-large");
    }
    let room = client
        .get_room(&parsed_room)
        .ok_or("v-send.1-attachment-room-not-found")?;
    let event_id =
        send_attachment_to_room(&room, filename, &mime_type, payload, reply_to, thread_root)
            .await?;
    Ok(MatrixSendRoomAttachmentResult {
        event_id,
        status: "sent",
    })
}
