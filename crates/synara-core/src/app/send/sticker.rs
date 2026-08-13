//! Native `m.sticker` content builder.
//!
//! Live `Room::send` stays on the attached timeline owner.

use matrix_sdk::ruma::{
    events::{
        relation::{Reply, Thread},
        room::{message::Relation, ImageInfo},
        sticker::StickerEventContent,
    },
    MxcUri, OwnedEventId, OwnedMxcUri, UInt,
};

pub fn sticker_content(
    body: String,
    mxc: String,
    width: Option<u64>,
    height: Option<u64>,
    mimetype: Option<String>,
    size: Option<u64>,
    reply_to: Option<OwnedEventId>,
    thread_root: Option<OwnedEventId>,
) -> Result<StickerEventContent, &'static str> {
    let body = body.trim();
    if body.is_empty() || body.len() > 1024 {
        return Err("v-send-sticker-invalid-body");
    }
    let mxc = mxc.trim();
    if mxc.is_empty() || mxc.len() > 1024 {
        return Err("v-send-sticker-invalid-mxc");
    }
    let mxc_ref: &MxcUri = mxc.into();
    if !mxc_ref.is_valid() {
        return Err("v-send-sticker-invalid-mxc");
    }
    let url: OwnedMxcUri = mxc_ref.to_owned();

    let mut info = ImageInfo::new();
    info.width = width.and_then(UInt::new);
    info.height = height.and_then(UInt::new);
    info.size = size.and_then(UInt::new);
    if let Some(mimetype) = mimetype {
        let mime = mimetype.trim();
        if !mime.is_empty() {
            if mime.len() > 255 || !mime.chars().all(|c| c.is_ascii_graphic()) {
                return Err("v-send-sticker-invalid-mimetype");
            }
            info.mimetype = Some(mime.to_owned());
        }
    }

    let mut content = StickerEventContent::new(body.to_owned(), info, url);
    content.relates_to = match (thread_root, reply_to) {
        (Some(root), Some(reply)) => Some(Relation::Thread(Thread::reply(root, reply))),
        (Some(root), None) => Some(Relation::Thread(Thread::without_fallback(root))),
        (None, Some(reply)) => Some(Relation::Reply(Reply::with_event_id(reply))),
        (None, None) => None,
    };
    Ok(content)
}
