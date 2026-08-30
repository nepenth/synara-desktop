//! Live room attachment send. Bytes stay method arguments, never Core JSON.

use matrix_sdk::attachment::AttachmentConfig;
use matrix_sdk::room::reply::{EnforceThread, Reply as AttachmentReply};
use matrix_sdk::ruma::events::room::message::{
    AddMentions, ReplyWithinThread, TextMessageEventContent,
};
use matrix_sdk::{Client, Room};
use mime::Mime;

use super::text::{
    parse_reply_event_id, parse_send_room_id, parse_thread_root_event_id, parse_transaction_id,
    validated_mentions,
};
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
    caption: Option<String>,
    formatted_caption: Option<String>,
    reply_to: Option<matrix_sdk::ruma::OwnedEventId>,
    thread_root: Option<matrix_sdk::ruma::OwnedEventId>,
    transaction_id: Option<matrix_sdk::ruma::OwnedTransactionId>,
    mention_user_ids: Option<Vec<String>>,
    mention_room: bool,
) -> Result<String, &'static str> {
    let config = attachment_config(
        caption,
        formatted_caption,
        reply_to,
        thread_root,
        transaction_id,
        mention_user_ids,
        mention_room,
    )?;
    let response = room
        .send_attachment(filename, mime_type, data, config)
        .await
        .map_err(|_| "v-send.1-attachment-sdk-failed")?;
    Ok(response.event_id.to_string())
}

#[allow(clippy::too_many_arguments)]
pub fn attachment_config(
    caption: Option<String>,
    formatted_caption: Option<String>,
    reply_to: Option<matrix_sdk::ruma::OwnedEventId>,
    thread_root: Option<matrix_sdk::ruma::OwnedEventId>,
    transaction_id: Option<matrix_sdk::ruma::OwnedTransactionId>,
    mention_user_ids: Option<Vec<String>>,
    mention_room: bool,
) -> Result<AttachmentConfig, &'static str> {
    let caption = attachment_caption(caption, formatted_caption)?;
    let reply = attachment_reply(reply_to, thread_root);
    let mentions = validated_mentions(mention_user_ids, mention_room)?;
    let config = AttachmentConfig::new()
        .caption(caption)
        .mentions(Some(mentions))
        .reply(reply);
    Ok(match transaction_id {
        Some(transaction_id) => config.txn_id(transaction_id),
        None => config,
    })
}

pub fn attachment_caption(
    caption: Option<String>,
    formatted_caption: Option<String>,
) -> Result<Option<TextMessageEventContent>, &'static str> {
    let caption = caption.filter(|value| !value.trim().is_empty());
    let formatted_caption = formatted_caption.filter(|value| !value.trim().is_empty());
    match (caption, formatted_caption) {
        (Some(body), Some(html)) => Ok(Some(TextMessageEventContent::html(body, html))),
        (Some(body), None) => Ok(Some(TextMessageEventContent::plain(body))),
        (None, Some(_)) => Err("v-send.1-attachment-formatted-caption-without-caption"),
        (None, None) => Ok(None),
    }
}

pub fn attachment_reply(
    reply_to: Option<matrix_sdk::ruma::OwnedEventId>,
    thread_root: Option<matrix_sdk::ruma::OwnedEventId>,
) -> Option<AttachmentReply> {
    match (thread_root, reply_to) {
        (Some(root), Some(_reply)) => Some(AttachmentReply {
            event_id: root,
            enforce_thread: EnforceThread::Threaded(ReplyWithinThread::No),
            add_mentions: AddMentions::No,
        }),
        (Some(root), None) => Some(AttachmentReply {
            event_id: root,
            enforce_thread: EnforceThread::Threaded(ReplyWithinThread::No),
            add_mentions: AddMentions::No,
        }),
        (None, Some(reply)) => Some(AttachmentReply {
            event_id: reply,
            enforce_thread: EnforceThread::MaybeThreaded,
            add_mentions: AddMentions::Yes,
        }),
        (None, None) => None,
    }
}

pub async fn send_room_attachment(
    client: &Client,
    room_id: &str,
    filename: &str,
    mime_type: &str,
    payload: Vec<u8>,
    caption: Option<String>,
    formatted_caption: Option<String>,
    reply_to: Option<String>,
    thread_root: Option<String>,
    transaction_id: Option<String>,
    mention_user_ids: Option<Vec<String>>,
    mention_room: bool,
) -> Result<MatrixSendRoomAttachmentResult, &'static str> {
    let parsed_room =
        parse_send_room_id(room_id).map_err(|_| "v-send.1-attachment-invalid-room")?;
    let reply_to =
        parse_reply_event_id(reply_to).map_err(|_| "v-send.1-attachment-invalid-reply")?;
    let thread_root = parse_thread_root_event_id(thread_root)
        .map_err(|_| "v-send.1-attachment-invalid-thread-root")?;
    let transaction_id = parse_transaction_id(transaction_id)
        .map_err(|_| "v-send.1-attachment-invalid-transaction-id")?;
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
    let event_id = send_attachment_to_room(
        &room,
        filename,
        &mime_type,
        payload,
        caption,
        formatted_caption,
        reply_to,
        thread_root,
        transaction_id,
        mention_user_ids,
        mention_room,
    )
    .await?;
    Ok(MatrixSendRoomAttachmentResult {
        event_id,
        status: "sent",
    })
}

#[cfg(test)]
mod tests {
    use matrix_sdk::room::reply::EnforceThread;
    use matrix_sdk::ruma::{event_id, events::room::message::ReplyWithinThread};

    use super::{attachment_caption, attachment_config, attachment_reply};

    #[test]
    fn attachment_caption_preserves_plain_and_formatted_content() {
        let plain = attachment_caption(Some("plain".to_owned()), None)
            .expect("plain caption")
            .expect("caption content");
        assert_eq!(plain.body, "plain");
        assert!(plain.formatted.is_none());

        let html = attachment_caption(
            Some("plain".to_owned()),
            Some("<strong>plain</strong>".to_owned()),
        )
        .expect("formatted caption")
        .expect("caption content");
        assert_eq!(html.body, "plain");
        assert_eq!(
            html.formatted
                .as_ref()
                .map(|formatted| formatted.body.as_str()),
            Some("<strong>plain</strong>")
        );
    }

    #[test]
    fn attachment_caption_rejects_orphan_formatted_body() {
        assert!(matches!(
            attachment_caption(None, Some("<b>orphan</b>".to_owned())),
            Err("v-send.1-attachment-formatted-caption-without-caption")
        ));
    }

    #[test]
    fn attachment_reply_encodes_reply_and_thread_cases() {
        let root = event_id!("$root:example.org").to_owned();
        let reply = event_id!("$reply:example.org").to_owned();

        let thread = attachment_reply(None, Some(root.clone())).expect("thread relation");
        assert_eq!(thread.event_id, root);
        assert_eq!(
            thread.enforce_thread,
            EnforceThread::Threaded(ReplyWithinThread::No)
        );

        let thread_reply = attachment_reply(
            Some(reply.clone()),
            Some(event_id!("$root:example.org").to_owned()),
        )
        .expect("thread reply relation");
        assert_eq!(thread_reply.event_id, event_id!("$root:example.org"));
        assert_eq!(
            thread_reply.enforce_thread,
            EnforceThread::Threaded(ReplyWithinThread::No)
        );

        let reply = attachment_reply(Some(event_id!("$reply:example.org").to_owned()), None)
            .expect("reply relation");
        assert_eq!(reply.enforce_thread, EnforceThread::MaybeThreaded);
    }

    #[test]
    fn attachment_config_preserves_transaction_and_validated_mentions() {
        let config = attachment_config(
            Some("caption".to_owned()),
            Some("<b>caption</b>".to_owned()),
            None,
            None,
            Some("stable-client-transaction".into()),
            Some(vec!["@alice:example.org".to_owned()]),
            true,
        )
        .expect("valid attachment config");
        assert_eq!(
            config.txn_id.as_deref().map(|value| value.as_str()),
            Some("stable-client-transaction")
        );
        let mentions = config.mentions.expect("mentions");
        assert!(mentions.room);
        assert!(mentions
            .user_ids
            .iter()
            .any(|user_id| user_id.as_str() == "@alice:example.org"));
        assert!(attachment_config(
            None,
            None,
            None,
            None,
            None,
            Some(vec!["invalid".into()]),
            false,
        )
        .is_err());
    }
}
