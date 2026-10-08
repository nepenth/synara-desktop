//! Typed SharedCore operations and projections for media.

use super::*;

/// Privacy-safe generic content upload result. mxc URI only; never bytes.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MediaUploadDto {
    pub mxc: String,
}

/// Static fail-closed content-upload error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum MediaUploadError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for MediaUploadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for MediaUploadError {}

/// Privacy-safe room attachment send ack. Event id and status only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SendRoomAttachmentDto {
    pub event_id: String,
    pub status: SendStatusDto,
}

/// Static fail-closed room-attachment error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum SendRoomAttachmentError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SendRoomAttachmentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SendRoomAttachmentError {}

/// Original-file or thumbnail bytes for a plain `mxc://`. Callers must not log the payload.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MediaBytesDto {
    pub payload: Vec<u8>,
}

/// Static fail-closed plain-media download error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum PlainMediaError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for PlainMediaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for PlainMediaError {}

/// Static fail-closed native media-handle error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum TimelineMediaError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for TimelineMediaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for TimelineMediaError {}

pub(super) fn timeline_media_failed(
    code: &'static str,
    description: &'static str,
) -> TimelineMediaError {
    TimelineMediaError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) type ViewMediaFields = (
    Option<String>,
    Option<String>,
    Option<u32>,
    Option<u32>,
    Option<u64>,
);

pub(super) fn view_media_fields(media: Option<TimelineMediaHandle>) -> ViewMediaFields {
    match media {
        Some(handle) => (
            Some(handle.handle_id),
            handle.mime_type,
            handle.width,
            handle.height,
            handle.duration_ms,
        ),
        None => (None, None, None, None, None),
    }
}

pub(super) fn upload_content_failed(code: &str, description: &'static str) -> MediaUploadError {
    MediaUploadError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_upload_content_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> MediaUploadError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            upload_content_failed(code, UPLOAD_CONTENT_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-send.") => {
            upload_content_failed(code, UPLOAD_CONTENT_OWNER_DESCRIPTION)
        }
        _ => upload_content_failed(
            UPLOAD_CONTENT_FAILED_CODE,
            UPLOAD_CONTENT_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn upload_content_reject_oversize(size: usize) -> Result<(), MediaUploadError> {
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(upload_content_failed(
            UPLOAD_CONTENT_FAILED_CODE,
            UPLOAD_CONTENT_FAILED_DESCRIPTION,
        ));
    }
    Ok(())
}

pub(super) fn send_room_attachment_failed(
    code: &str,
    description: &'static str,
) -> SendRoomAttachmentError {
    SendRoomAttachmentError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_send_room_attachment_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> SendRoomAttachmentError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            send_room_attachment_failed(code, SEND_ROOM_ATTACHMENT_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-send.") => {
            send_room_attachment_failed(code, SEND_ROOM_ATTACHMENT_OWNER_DESCRIPTION)
        }
        _ => send_room_attachment_failed(
            SEND_ROOM_ATTACHMENT_FAILED_CODE,
            SEND_ROOM_ATTACHMENT_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn send_room_attachment_reject_oversize(
    size: usize,
) -> Result<(), SendRoomAttachmentError> {
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(send_room_attachment_failed(
            SEND_ROOM_ATTACHMENT_FAILED_CODE,
            SEND_ROOM_ATTACHMENT_FAILED_DESCRIPTION,
        ));
    }
    Ok(())
}

pub(super) fn plain_media_failed(code: &str, description: &'static str) -> PlainMediaError {
    PlainMediaError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_plain_media_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> PlainMediaError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            plain_media_failed(code, PLAIN_MEDIA_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-send.") => {
            plain_media_failed(code, PLAIN_MEDIA_OWNER_DESCRIPTION)
        }
        _ => plain_media_failed(PLAIN_MEDIA_FAILED_CODE, PLAIN_MEDIA_FAILED_DESCRIPTION),
    }
}

pub(super) fn plain_media_reject_oversize(size: usize) -> Result<(), PlainMediaError> {
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(plain_media_failed(
            PLAIN_MEDIA_FAILED_CODE,
            PLAIN_MEDIA_FAILED_DESCRIPTION,
        ));
    }
    Ok(())
}

#[uniffi::export(async_runtime = "tokio")]
impl SharedCore {
    pub async fn upload_avatar(
        &self,
        payload: Vec<u8>,
        mime_type: String,
    ) -> Result<OwnProfileUploadDto, OwnProfileCommandError> {
        if mime_type.len() > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
            return Err(own_profile_failed(
                OWN_PROFILE_FAILED_CODE,
                OWN_PROFILE_FAILED_DESCRIPTION,
            ));
        }
        let result = self
            .core
            .upload_avatar(payload, &mime_type)
            .await
            .map_err(|error| map_own_profile_core_error(UPLOAD_AVATAR_NO_SESSION_CODE, error))?;
        Ok(OwnProfileUploadDto { mxc: result.mxc })
    }

    pub async fn upload_content(
        &self,
        payload: Vec<u8>,
        mime_type: String,
        filename: Option<String>,
    ) -> Result<MediaUploadDto, MediaUploadError> {
        upload_content_reject_oversize(mime_type.len())?;
        if let Some(filename) = filename.as_ref() {
            upload_content_reject_oversize(filename.len())?;
        }
        let result = self
            .core
            .upload_content(payload, &mime_type, filename.as_deref())
            .await
            .map_err(|error| {
                map_upload_content_core_error(UPLOAD_CONTENT_NO_SESSION_CODE, error)
            })?;
        Ok(MediaUploadDto { mxc: result.mxc })
    }

    #[allow(clippy::too_many_arguments)] // Stable UniFFI fields remain explicit for compatibility.
    pub async fn send_room_attachment(
        &self,
        room_id: String,
        filename: String,
        mime_type: String,
        payload: Vec<u8>,
        caption: Option<String>,
        formatted_caption: Option<String>,
        reply_to: Option<String>,
        thread_root: Option<String>,
        transaction_id: Option<String>,
        mention_user_ids: Option<Vec<String>>,
        mention_room: Option<bool>,
    ) -> Result<SendRoomAttachmentDto, SendRoomAttachmentError> {
        send_room_attachment_reject_oversize(room_id.len())?;
        send_room_attachment_reject_oversize(filename.len())?;
        send_room_attachment_reject_oversize(mime_type.len())?;
        if let Some(caption) = caption.as_ref() {
            send_room_attachment_reject_oversize(caption.len())?;
        }
        if let Some(formatted_caption) = formatted_caption.as_ref() {
            send_room_attachment_reject_oversize(formatted_caption.len())?;
        }
        if let Some(reply_to) = reply_to.as_ref() {
            send_room_attachment_reject_oversize(reply_to.len())?;
        }
        if let Some(thread_root) = thread_root.as_ref() {
            send_room_attachment_reject_oversize(thread_root.len())?;
        }
        if let Some(transaction_id) = transaction_id.as_ref() {
            send_room_attachment_reject_oversize(transaction_id.len())?;
        }
        if let Some(mention_user_ids) = mention_user_ids.as_ref() {
            for user_id in mention_user_ids {
                send_room_attachment_reject_oversize(user_id.len())?;
            }
        }
        let result = self
            .core
            .send_room_attachment(crate::app::send::SendRoomAttachmentRequest {
                room_id,
                filename,
                mime_type,
                payload,
                caption,
                formatted_caption,
                reply_to,
                thread_root,
                transaction_id,
                mention_user_ids,
                mention_room: mention_room.unwrap_or(false),
            })
            .await
            .map_err(|error| {
                map_send_room_attachment_core_error(SEND_ROOM_ATTACHMENT_NO_SESSION_CODE, error)
            })?;
        Ok(SendRoomAttachmentDto {
            event_id: result.event_id,
            status: result.status.into(),
        })
    }

    pub async fn download_plain_media(
        &self,
        content_uri: String,
    ) -> Result<MediaBytesDto, PlainMediaError> {
        plain_media_reject_oversize(content_uri.len())?;
        let payload = self
            .core
            .download_plain_media(&content_uri)
            .await
            .map_err(|error| {
                map_plain_media_core_error(DOWNLOAD_PLAIN_MEDIA_NO_SESSION_CODE, error)
            })?;
        Ok(MediaBytesDto { payload })
    }

    pub async fn thumbnail_plain_media(
        &self,
        content_uri: String,
        width: u64,
        height: u64,
    ) -> Result<MediaBytesDto, PlainMediaError> {
        plain_media_reject_oversize(content_uri.len())?;
        let payload = self
            .core
            .thumbnail_plain_media(&content_uri, width, height)
            .await
            .map_err(|error| {
                map_plain_media_core_error(THUMBNAIL_PLAIN_MEDIA_NO_SESSION_CODE, error)
            })?;
        Ok(MediaBytesDto { payload })
    }

    pub async fn timeline_forward_media(
        &self,
        source_room_id: String,
        event_id: String,
        target_room_id: String,
        confirmed_encryption_downgrade: bool,
    ) -> Result<TimelineForwardDto, TimelineForwardError> {
        let request = crate::core_api::MatrixTimelineForwardMediaRequest {
            source_room_id,
            event_id,
            target_room_id,
            confirmed_encryption_downgrade,
        };
        if !within_envelope_cap(&request) {
            return Err(timeline_forward_failed(
                TIMELINE_FORWARD_FAILED_CODE,
                TIMELINE_FORWARD_FAILED_DESCRIPTION,
            ));
        }
        self.timeline_forward_command(
            TIMELINE_FORWARD_MEDIA_NO_SESSION_CODE,
            self.core.timeline_forward_media(request),
        )
        .await
    }

    /// Download bytes for an opaque timeline media handle.
    ///
    /// Dedicated UniFFI bytes, not a `Core.command` envelope. NSE cannot
    /// download. Unknown handles and missing owners fail closed without
    /// echoing the handle, mxc, or tokens.
    pub async fn timeline_media_bytes(
        &self,
        handle_id: String,
    ) -> Result<LeftoverBytesDto, TimelineMediaError> {
        let Some(owner) = self.core.attached_timeline_owner() else {
            return Err(timeline_media_failed(
                TIMELINE_MEDIA_NO_SESSION_CODE,
                TIMELINE_MEDIA_NO_SESSION_DESCRIPTION,
            ));
        };
        match owner.media_bytes(&handle_id).await {
            Ok(payload) => Ok(LeftoverBytesDto { payload }),
            Err("p4-s33-media-unknown-handle") => Err(timeline_media_failed(
                TIMELINE_MEDIA_UNKNOWN_HANDLE_CODE,
                TIMELINE_MEDIA_UNKNOWN_HANDLE_DESCRIPTION,
            )),
            Err("p4-s33-media-too-large") => Err(timeline_media_failed(
                TIMELINE_MEDIA_TOO_LARGE_CODE,
                TIMELINE_MEDIA_TOO_LARGE_DESCRIPTION,
            )),
            Err(_) => Err(timeline_media_failed(
                TIMELINE_MEDIA_FAILED_CODE,
                TIMELINE_MEDIA_FAILED_DESCRIPTION,
            )),
        }
    }

    pub async fn media_download(
        &self,
        mxc: String,
    ) -> Result<LeftoverBytesDto, LeftoverCommandError> {
        leftover_reject_oversize(mxc.len())?;
        if !self.has_retained_client() {
            return Err(leftover_failed(
                LEFTOVER_NO_SESSION_CODE,
                LEFTOVER_NO_SESSION_DESCRIPTION,
            ));
        }
        Err(leftover_failed(
            LEFTOVER_UNAVAILABLE_CODE,
            LEFTOVER_UNAVAILABLE_DESCRIPTION,
        ))
    }

    pub async fn media_thumbnail(
        &self,
        mxc: String,
        width: u64,
        height: u64,
    ) -> Result<LeftoverBytesDto, LeftoverCommandError> {
        leftover_reject_oversize(mxc.len())?;
        let _ = (width, height);
        if !self.has_retained_client() {
            return Err(leftover_failed(
                LEFTOVER_NO_SESSION_CODE,
                LEFTOVER_NO_SESSION_DESCRIPTION,
            ));
        }
        Err(leftover_failed(
            LEFTOVER_UNAVAILABLE_CODE,
            LEFTOVER_UNAVAILABLE_DESCRIPTION,
        ))
    }

    pub async fn media_upload(
        &self,
        payload: Vec<u8>,
        mime_type: String,
        filename: String,
    ) -> Result<LeftoverAckDto, LeftoverCommandError> {
        leftover_reject_oversize(payload.len() + mime_type.len() + filename.len())?;
        if !self.has_retained_client() {
            return Err(leftover_failed(
                LEFTOVER_NO_SESSION_CODE,
                LEFTOVER_NO_SESSION_DESCRIPTION,
            ));
        }
        Err(leftover_failed(
            LEFTOVER_UNAVAILABLE_CODE,
            LEFTOVER_UNAVAILABLE_DESCRIPTION,
        ))
    }

    pub async fn room_avatar_bytes(
        &self,
        room_id: String,
    ) -> Result<LeftoverBytesDto, LeftoverCommandError> {
        leftover_reject_oversize(room_id.len())?;
        if !self.has_retained_client() {
            return Err(leftover_failed(
                LEFTOVER_NO_SESSION_CODE,
                LEFTOVER_NO_SESSION_DESCRIPTION,
            ));
        }
        Err(leftover_failed(
            LEFTOVER_UNAVAILABLE_CODE,
            LEFTOVER_UNAVAILABLE_DESCRIPTION,
        ))
    }
}
