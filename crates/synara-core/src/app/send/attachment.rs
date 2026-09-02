//! Live room attachment send. Bytes stay method arguments, never Core JSON.

use matrix_sdk::attachment::AttachmentConfig;
use matrix_sdk::room::reply::{EnforceThread, Reply as AttachmentReply};
use matrix_sdk::ruma::events::room::message::{
    AddMentions, ReplyWithinThread, TextMessageEventContent,
};
use matrix_sdk::Client;
use mime::Mime;

use super::text::{
    parse_reply_event_id, parse_send_room_id, parse_thread_root_event_id, parse_transaction_id,
    validate_outbound_text_payload, validated_mentions,
};
use super::MatrixSendRoomAttachmentResult;

/// Same IPC cap as desktop composer attachments (`MAX_ATTACHMENT_IPC_BYTES`).
pub const MAX_ATTACHMENT_UPLOAD_BYTES: usize = 32 * 1024 * 1024;

/// Complete native attachment-send intent after crossing a shell ABI boundary.
///
/// Keeping the fields together prevents internal forwarding layers from
/// accidentally reordering caption, relation, transaction, or mention data.
#[derive(Debug)]
pub struct SendRoomAttachmentRequest {
    pub room_id: String,
    pub filename: String,
    pub mime_type: String,
    pub payload: Vec<u8>,
    pub caption: Option<String>,
    pub formatted_caption: Option<String>,
    pub reply_to: Option<String>,
    pub thread_root: Option<String>,
    pub transaction_id: Option<String>,
    pub mention_user_ids: Option<Vec<String>>,
    pub mention_room: bool,
}

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
    if caption.is_none() && formatted_caption.is_some() {
        return Err("v-send.1-attachment-formatted-caption-without-caption");
    }
    validate_outbound_text_payload(
        caption.as_deref().unwrap_or_default(),
        formatted_caption.as_deref(),
    )?;
    match (caption, formatted_caption) {
        (Some(body), Some(html)) => Ok(Some(TextMessageEventContent::html(body, html))),
        (Some(body), None) => Ok(Some(TextMessageEventContent::plain(body))),
        (None, Some(_)) => unreachable!("orphan formatted caption rejected above"),
        (None, None) => Ok(None),
    }
}

pub fn attachment_reply(
    reply_to: Option<matrix_sdk::ruma::OwnedEventId>,
    thread_root: Option<matrix_sdk::ruma::OwnedEventId>,
) -> Option<AttachmentReply> {
    match (thread_root, reply_to) {
        // The SDK derives the authoritative thread root from the selected
        // event. ReplyWithinThread::Yes preserves that selected event as
        // m.in_reply_to instead of degrading this into a root-only thread send.
        (Some(_root), Some(reply)) => Some(AttachmentReply {
            event_id: reply,
            enforce_thread: EnforceThread::Threaded(ReplyWithinThread::Yes),
            add_mentions: AddMentions::Yes,
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
    request: SendRoomAttachmentRequest,
) -> Result<MatrixSendRoomAttachmentResult, &'static str> {
    let parsed_room =
        parse_send_room_id(&request.room_id).map_err(|_| "v-send.1-attachment-invalid-room")?;
    let reply_to =
        parse_reply_event_id(request.reply_to).map_err(|_| "v-send.1-attachment-invalid-reply")?;
    let thread_root = parse_thread_root_event_id(request.thread_root)
        .map_err(|_| "v-send.1-attachment-invalid-thread-root")?;
    let transaction_id = parse_transaction_id(request.transaction_id)
        .map_err(|_| "v-send.1-attachment-invalid-transaction-id")?;
    let filename = validate_attachment_filename(&request.filename)?;
    let mime_type = validate_attachment_mime(&request.mime_type)?;
    if request.payload.is_empty() {
        return Err("v-send.1-attachment-empty");
    }
    if request.payload.len() > MAX_ATTACHMENT_UPLOAD_BYTES {
        return Err("v-send.1-attachment-too-large");
    }
    let room = client
        .get_room(&parsed_room)
        .ok_or("v-send.1-attachment-room-not-found")?;
    let config = attachment_config(
        request.caption,
        request.formatted_caption,
        reply_to,
        thread_root,
        transaction_id,
        request.mention_user_ids,
        request.mention_room,
    )?;
    let event_id = room
        .send_attachment(filename, &mime_type, request.payload, config)
        .await
        .map_err(|_| "v-send.1-attachment-sdk-failed")?
        .event_id
        .to_string();
    Ok(MatrixSendRoomAttachmentResult {
        event_id,
        status: "sent",
    })
}

#[cfg(test)]
mod tests {
    use matrix_sdk::room::reply::EnforceThread;
    use matrix_sdk::ruma::{
        event_id,
        events::room::message::{AddMentions, ReplyWithinThread},
    };

    use super::{
        super::MAX_OUTBOUND_TEXT_PAYLOAD_BYTES, attachment_caption, attachment_config,
        attachment_reply,
    };

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
    fn attachment_caption_uses_shared_combined_utf8_payload_budget() {
        let plain_at_limit = "🙂".repeat(MAX_OUTBOUND_TEXT_PAYLOAD_BYTES / "🙂".len());
        assert_eq!(plain_at_limit.len(), MAX_OUTBOUND_TEXT_PAYLOAD_BYTES);
        assert!(attachment_caption(Some(plain_at_limit), None).is_ok());

        let plain_over_limit = format!(
            "{}x",
            "🙂".repeat(MAX_OUTBOUND_TEXT_PAYLOAD_BYTES / "🙂".len())
        );
        assert_eq!(plain_over_limit.len(), MAX_OUTBOUND_TEXT_PAYLOAD_BYTES + 1);
        assert_eq!(
            attachment_caption(Some(plain_over_limit), None)
                .expect_err("oversized plain attachment caption must be rejected"),
            "d0.4-send-text-payload-too-large"
        );

        let body = "caption";
        let formatted_at_limit = "x".repeat(MAX_OUTBOUND_TEXT_PAYLOAD_BYTES - body.len());
        assert!(attachment_caption(Some(body.to_owned()), Some(formatted_at_limit)).is_ok());

        let formatted_over_limit = "x".repeat(MAX_OUTBOUND_TEXT_PAYLOAD_BYTES - body.len() + 1);
        assert_eq!(
            attachment_caption(Some(body.to_owned()), Some(formatted_over_limit))
                .expect_err("oversized combined attachment caption must be rejected"),
            "d0.4-send-text-payload-too-large"
        );
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
        assert_eq!(thread_reply.event_id, reply);
        assert_eq!(
            thread_reply.enforce_thread,
            EnforceThread::Threaded(ReplyWithinThread::Yes)
        );
        assert_eq!(thread_reply.add_mentions, AddMentions::Yes);

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
