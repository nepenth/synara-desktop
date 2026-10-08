//! Typed SharedCore operations and projections for account data.

use super::*;

pub(super) fn account_data_owner_update_family(
    kind: NativeAccountDataWakeupKind,
) -> Option<&'static str> {
    match kind {
        NativeAccountDataWakeupKind::AgentNotificationPreferences => {
            Some("agent_notification_preferences")
        }
        NativeAccountDataWakeupKind::ImagePacks => Some("image_packs"),
        // Older iOS waiters ignore unknown families, so this cannot mis-route
        // into the image-pack UI. New iOS refetches history on this family.
        NativeAccountDataWakeupKind::AgentApprovalHistory => Some("agent_approval_history"),
    }
}

/// Privacy-safe image-pack row. Metadata/IDs/mxc URLs/JSON only; never image bytes.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ImagePackDto {
    pub id: String,
    pub room_id: Option<String>,
    pub state_key: Option<String>,
    pub content_json: String,
}

/// Privacy-safe user pack snapshot. No tokens or image bytes.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct UserImagePackSnapshotDto {
    pub session_generation: u64,
    pub pack: Option<ImagePackDto>,
}

/// Privacy-safe room pack snapshot. No tokens or image bytes.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomImagePacksSnapshotDto {
    pub session_generation: u64,
    pub room_id: String,
    pub packs: Vec<ImagePackDto>,
}

/// Privacy-safe global pack snapshot. No tokens or image bytes.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GlobalImagePacksSnapshotDto {
    pub session_generation: u64,
    pub packs: Vec<ImagePackDto>,
}

/// Privacy-safe pack write ack. Status only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ImagePackWriteDto {
    pub status: String,
}

/// Static fail-closed image-pack-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum ImagePackCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for ImagePackCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for ImagePackCommandError {}

pub(super) fn image_pack_failed(code: &str, description: &'static str) -> ImagePackCommandError {
    ImagePackCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_image_pack_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> ImagePackCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            image_pack_failed(code, IMAGE_PACK_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-send.r-pack-") => {
            image_pack_failed(code, IMAGE_PACK_OWNER_DESCRIPTION)
        }
        _ => image_pack_failed(IMAGE_PACK_FAILED_CODE, IMAGE_PACK_FAILED_DESCRIPTION),
    }
}

pub(super) fn parse_image_pack_content_json(
    content_json: &str,
) -> Result<serde_json::Value, ImagePackCommandError> {
    if content_json.len() > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(image_pack_failed(
            IMAGE_PACK_FAILED_CODE,
            IMAGE_PACK_FAILED_DESCRIPTION,
        ));
    }
    serde_json::from_str(content_json).map_err(|_| {
        image_pack_failed(
            IMAGE_PACK_INVALID_JSON_CODE,
            IMAGE_PACK_INVALID_JSON_DESCRIPTION,
        )
    })
}

pub(super) fn image_pack_dto(pack: NativeImagePack) -> Result<ImagePackDto, ImagePackCommandError> {
    let content_json = serde_json::to_string(&pack.content)
        .map_err(|_| image_pack_failed(IMAGE_PACK_FAILED_CODE, IMAGE_PACK_FAILED_DESCRIPTION))?;
    Ok(ImagePackDto {
        id: pack.id,
        room_id: pack.room_id,
        state_key: pack.state_key,
        content_json,
    })
}

pub(super) fn image_pack_write_dto(
    payload: serde_json::Value,
) -> Result<ImagePackWriteDto, ImagePackCommandError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .ok_or_else(|| image_pack_failed(IMAGE_PACK_FAILED_CODE, IMAGE_PACK_FAILED_DESCRIPTION))?;
    Ok(ImagePackWriteDto {
        status: status.to_owned(),
    })
}

/// Privacy-safe later item. Room/event ids and timestamps only; no tokens.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct LaterItemDto {
    pub id: String,
    pub kind: String,
    pub room_id: String,
    pub event_id: String,
    pub created_at: f64,
    pub due_ts: Option<f64>,
    pub reminded_at: Option<f64>,
    pub completed_at: Option<f64>,
}

/// Privacy-safe later snapshot. No tokens or secret material.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct LaterSnapshotDto {
    pub session_generation: u64,
    pub version: u32,
    pub items: Vec<LaterItemDto>,
}

/// Static fail-closed later-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum LaterCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for LaterCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for LaterCommandError {}

pub(super) fn later_failed(code: &str, description: &'static str) -> LaterCommandError {
    LaterCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_later_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> LaterCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => later_failed(code, LATER_NO_SESSION_DESCRIPTION),
        Some(code) if code.starts_with("v-timeline-later-") => {
            later_failed(code, LATER_OWNER_DESCRIPTION)
        }
        _ => later_failed(LATER_FAILED_CODE, LATER_FAILED_DESCRIPTION),
    }
}

pub(super) fn later_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, LaterCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(later_failed(LATER_FAILED_CODE, LATER_FAILED_DESCRIPTION));
    }
    Ok(payload)
}

pub(super) fn later_item_from_dto(
    item: LaterItemDto,
) -> Result<SynaraLaterItem, LaterCommandError> {
    let kind = match item.kind.as_str() {
        "saved" => SynaraLaterItemKind::Saved,
        "reminder" => SynaraLaterItemKind::Reminder,
        _ => {
            return Err(later_failed(
                LATER_INVALID_ITEM_CODE,
                LATER_INVALID_ITEM_DESCRIPTION,
            ))
        }
    };
    if item.id.is_empty()
        || item.room_id.is_empty()
        || item.event_id.is_empty()
        || !item.created_at.is_finite()
    {
        return Err(later_failed(
            LATER_INVALID_ITEM_CODE,
            LATER_INVALID_ITEM_DESCRIPTION,
        ));
    }
    Ok(SynaraLaterItem {
        id: item.id,
        kind,
        room_id: item.room_id,
        event_id: item.event_id,
        created_at: item.created_at,
        due_ts: item.due_ts.filter(|value| value.is_finite()),
        reminded_at: item.reminded_at.filter(|value| value.is_finite()),
        completed_at: item.completed_at.filter(|value| value.is_finite()),
    })
}

pub(super) fn later_item_dto(item: SynaraLaterItem) -> LaterItemDto {
    LaterItemDto {
        id: item.id,
        kind: match item.kind {
            SynaraLaterItemKind::Saved => "saved".to_owned(),
            SynaraLaterItemKind::Reminder => "reminder".to_owned(),
        },
        room_id: item.room_id,
        event_id: item.event_id,
        created_at: item.created_at,
        due_ts: item.due_ts,
        reminded_at: item.reminded_at,
        completed_at: item.completed_at,
    }
}

pub(super) fn later_snapshot_dto(
    payload: serde_json::Value,
) -> Result<LaterSnapshotDto, LaterCommandError> {
    let snapshot: NativeLaterSnapshot = serde_json::from_value(payload)
        .map_err(|_| later_failed(LATER_FAILED_CODE, LATER_FAILED_DESCRIPTION))?;
    Ok(LaterSnapshotDto {
        session_generation: snapshot.session_generation,
        version: snapshot.content.version,
        items: snapshot
            .content
            .items
            .into_values()
            .map(later_item_dto)
            .collect(),
    })
}

/// Privacy-safe m.direct snapshot. User/room ids are the product map; no tokens.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MDirectSnapshotDto {
    pub session_generation: u64,
    pub room_ids: Vec<String>,
    pub user_ids: Vec<String>,
}

/// Privacy-safe m.direct write ack. Status and the mutated room id only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MDirectMutationDto {
    pub room_id: String,
    pub status: String,
}

/// Static fail-closed m.direct-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum MDirectCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for MDirectCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for MDirectCommandError {}

pub(super) fn mdirect_failed(code: &str, description: &'static str) -> MDirectCommandError {
    MDirectCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_mdirect_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> MDirectCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => mdirect_failed(code, MDIRECT_NO_SESSION_DESCRIPTION),
        Some(code) if code.starts_with("v-rooms.5-mdirect-") => {
            mdirect_failed(code, MDIRECT_OWNER_DESCRIPTION)
        }
        _ => mdirect_failed(MDIRECT_FAILED_CODE, MDIRECT_FAILED_DESCRIPTION),
    }
}

pub(super) fn mdirect_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, MDirectCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(mdirect_failed(
            MDIRECT_FAILED_CODE,
            MDIRECT_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

pub(super) fn mdirect_snapshot_dto(
    payload: serde_json::Value,
) -> Result<MDirectSnapshotDto, MDirectCommandError> {
    let snapshot: NativeMDirectSnapshot = serde_json::from_value(payload)
        .map_err(|_| mdirect_failed(MDIRECT_FAILED_CODE, MDIRECT_FAILED_DESCRIPTION))?;
    Ok(MDirectSnapshotDto {
        session_generation: snapshot.session_generation,
        room_ids: snapshot.room_ids,
        user_ids: snapshot.user_ids,
    })
}

pub(super) fn mdirect_mutation_dto(
    payload: serde_json::Value,
) -> Result<MDirectMutationDto, MDirectCommandError> {
    let room_id = payload
        .get("roomId")
        .and_then(|value| value.as_str())
        .ok_or_else(|| mdirect_failed(MDIRECT_FAILED_CODE, MDIRECT_FAILED_DESCRIPTION))?;
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .ok_or_else(|| mdirect_failed(MDIRECT_FAILED_CODE, MDIRECT_FAILED_DESCRIPTION))?;
    if status != "updated" {
        return Err(mdirect_failed(
            MDIRECT_FAILED_CODE,
            MDIRECT_FAILED_DESCRIPTION,
        ));
    }
    Ok(MDirectMutationDto {
        room_id: room_id.to_owned(),
        status: status.to_owned(),
    })
}

/// Privacy-safe room-notes snapshot. Flattened items; no tokens.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct RoomNotesSnapshotDto {
    pub session_generation: u64,
    pub version: u32,
    pub items: Vec<RoomNoteItemDto>,
}

/// Static fail-closed room-notes-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum RoomNotesCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomNotesCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomNotesCommandError {}

pub(super) fn room_notes_failed(code: &str, description: &'static str) -> RoomNotesCommandError {
    RoomNotesCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_room_notes_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> RoomNotesCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            room_notes_failed(code, ROOM_NOTES_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-timeline-room-notes-") => {
            room_notes_failed(code, ROOM_NOTES_OWNER_DESCRIPTION)
        }
        _ => room_notes_failed(ROOM_NOTES_FAILED_CODE, ROOM_NOTES_FAILED_DESCRIPTION),
    }
}

pub(super) fn room_notes_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, RoomNotesCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(room_notes_failed(
            ROOM_NOTES_FAILED_CODE,
            ROOM_NOTES_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

pub(super) fn room_notes_snapshot_dto(
    payload: serde_json::Value,
) -> Result<RoomNotesSnapshotDto, RoomNotesCommandError> {
    let snapshot: NativeRoomNotesSnapshot = serde_json::from_value(payload)
        .map_err(|_| room_notes_failed(ROOM_NOTES_FAILED_CODE, ROOM_NOTES_FAILED_DESCRIPTION))?;
    Ok(RoomNotesSnapshotDto {
        session_generation: snapshot.session_generation,
        version: snapshot.content.version,
        items: snapshot
            .content
            .rooms
            .into_values()
            .flat_map(|room| room.items.into_values())
            .map(room_note_item_dto)
            .collect(),
    })
}

/// Privacy-safe directory-visibility read. Visibility is public/private only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomDirectoryVisibilityDto {
    pub status: String,
    pub room_id: String,
    pub session_generation: u64,
    pub visibility: String,
}

/// Privacy-safe directory-visibility write ack. Visibility is public/private only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomDirectoryVisibilityWriteDto {
    pub status: String,
    pub room_id: String,
    pub session_generation: u64,
    pub requested_visibility: String,
}

pub(super) fn room_directory_visibility_dto(
    payload: serde_json::Value,
) -> Result<RoomDirectoryVisibilityDto, DirectoryVisibilityCommandError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            directory_visibility_failed(
                DIRECTORY_VISIBILITY_FAILED_CODE,
                DIRECTORY_VISIBILITY_FAILED_DESCRIPTION,
            )
        })?;
    let room_id = payload
        .get("roomId")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            directory_visibility_failed(
                DIRECTORY_VISIBILITY_FAILED_CODE,
                DIRECTORY_VISIBILITY_FAILED_DESCRIPTION,
            )
        })?;
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            directory_visibility_failed(
                DIRECTORY_VISIBILITY_FAILED_CODE,
                DIRECTORY_VISIBILITY_FAILED_DESCRIPTION,
            )
        })?;
    let visibility = payload
        .get("visibility")
        .and_then(|value| value.as_str())
        .and_then(closed_directory_visibility)
        .ok_or_else(|| {
            directory_visibility_failed(
                DIRECTORY_VISIBILITY_FAILED_CODE,
                DIRECTORY_VISIBILITY_FAILED_DESCRIPTION,
            )
        })?;
    Ok(RoomDirectoryVisibilityDto {
        status: status.to_owned(),
        room_id: room_id.to_owned(),
        session_generation,
        visibility: visibility.to_owned(),
    })
}

pub(super) fn room_directory_visibility_write_dto(
    payload: serde_json::Value,
) -> Result<RoomDirectoryVisibilityWriteDto, DirectoryVisibilityCommandError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            directory_visibility_failed(
                DIRECTORY_VISIBILITY_FAILED_CODE,
                DIRECTORY_VISIBILITY_FAILED_DESCRIPTION,
            )
        })?;
    let room_id = payload
        .get("roomId")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            directory_visibility_failed(
                DIRECTORY_VISIBILITY_FAILED_CODE,
                DIRECTORY_VISIBILITY_FAILED_DESCRIPTION,
            )
        })?;
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            directory_visibility_failed(
                DIRECTORY_VISIBILITY_FAILED_CODE,
                DIRECTORY_VISIBILITY_FAILED_DESCRIPTION,
            )
        })?;
    let requested_visibility = payload
        .get("requestedVisibility")
        .and_then(|value| value.as_str())
        .and_then(closed_directory_visibility)
        .ok_or_else(|| {
            directory_visibility_failed(
                DIRECTORY_VISIBILITY_FAILED_CODE,
                DIRECTORY_VISIBILITY_FAILED_DESCRIPTION,
            )
        })?;
    Ok(RoomDirectoryVisibilityWriteDto {
        status: status.to_owned(),
        room_id: room_id.to_owned(),
        session_generation,
        requested_visibility: requested_visibility.to_owned(),
    })
}

/// Privacy-safe third-party directory protocol instance. Ids and description only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomDirectoryProtocolInstanceDto {
    pub protocol_id: String,
    pub instance_id: String,
    pub description: String,
}

/// Privacy-safe protocol list. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomDirectoryProtocolsDto {
    pub session_generation: u64,
    pub instances: Vec<RoomDirectoryProtocolInstanceDto>,
}

/// Privacy-safe public-directory room hit. Metadata only; avatar_url is mxc, never bytes.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomDirectoryHitDto {
    pub room_id: String,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub canonical_alias: Option<String>,
    pub avatar_url: Option<String>,
    pub member_count: u32,
    pub world_readable: bool,
    pub guest_can_join: bool,
    pub room_type: String,
}

/// Privacy-safe search page. Room metadata only; no avatar bytes.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomDirectoryPageDto {
    pub session_generation: u64,
    pub request_id: u64,
    pub chunk: Vec<RoomDirectoryHitDto>,
    pub prev_batch: Option<String>,
    pub next_batch: Option<String>,
}

/// Privacy-safe search/cancel result. Status is ready/stale/cancelled.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomDirectorySearchDto {
    pub session_generation: u64,
    pub request_id: u64,
    pub status: String,
    pub page: Option<RoomDirectoryPageDto>,
}

pub(super) fn room_directory_protocols_dto(
    payload: serde_json::Value,
) -> Result<RoomDirectoryProtocolsDto, DirectorySearchCommandError> {
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let instances = payload
        .get("instances")
        .and_then(|value| value.as_array())
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let mut mapped = Vec::with_capacity(instances.len());
    for instance in instances {
        let protocol_id = instance
            .get("protocolId")
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                directory_search_failed(
                    DIRECTORY_SEARCH_FAILED_CODE,
                    DIRECTORY_SEARCH_FAILED_DESCRIPTION,
                )
            })?;
        let instance_id = instance
            .get("instanceId")
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                directory_search_failed(
                    DIRECTORY_SEARCH_FAILED_CODE,
                    DIRECTORY_SEARCH_FAILED_DESCRIPTION,
                )
            })?;
        let description = instance
            .get("description")
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                directory_search_failed(
                    DIRECTORY_SEARCH_FAILED_CODE,
                    DIRECTORY_SEARCH_FAILED_DESCRIPTION,
                )
            })?;
        mapped.push(RoomDirectoryProtocolInstanceDto {
            protocol_id: protocol_id.to_owned(),
            instance_id: instance_id.to_owned(),
            description: description.to_owned(),
        });
    }
    Ok(RoomDirectoryProtocolsDto {
        session_generation,
        instances: mapped,
    })
}

pub(super) fn room_directory_hit_dto(
    payload: &serde_json::Value,
) -> Result<RoomDirectoryHitDto, DirectorySearchCommandError> {
    let room_id = payload
        .get("roomId")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let member_count = payload
        .get("memberCount")
        .and_then(|value| value.as_u64())
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let world_readable = payload
        .get("worldReadable")
        .and_then(|value| value.as_bool())
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let guest_can_join = payload
        .get("guestCanJoin")
        .and_then(|value| value.as_bool())
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let room_type = payload
        .get("roomType")
        .and_then(|value| value.as_str())
        .and_then(closed_directory_room_type)
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    Ok(RoomDirectoryHitDto {
        room_id: room_id.to_owned(),
        name: json_optional_string(payload.get("name")),
        topic: json_optional_string(payload.get("topic")),
        canonical_alias: json_optional_string(payload.get("canonicalAlias")),
        avatar_url: json_optional_string(payload.get("avatarUrl")),
        member_count,
        world_readable,
        guest_can_join,
        room_type: room_type.to_owned(),
    })
}

pub(super) fn room_directory_page_dto(
    payload: &serde_json::Value,
) -> Result<RoomDirectoryPageDto, DirectorySearchCommandError> {
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let request_id = payload
        .get("requestId")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let chunk = payload
        .get("chunk")
        .and_then(|value| value.as_array())
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let mut mapped = Vec::with_capacity(chunk.len());
    for hit in chunk {
        mapped.push(room_directory_hit_dto(hit)?);
    }
    Ok(RoomDirectoryPageDto {
        session_generation,
        request_id,
        chunk: mapped,
        prev_batch: json_optional_string(payload.get("prevBatch")),
        next_batch: json_optional_string(payload.get("nextBatch")),
    })
}

pub(super) fn room_directory_search_dto(
    payload: serde_json::Value,
) -> Result<RoomDirectorySearchDto, DirectorySearchCommandError> {
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let request_id = payload
        .get("requestId")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .and_then(closed_directory_search_status)
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let page = match payload.get("page") {
        None | Some(serde_json::Value::Null) => None,
        Some(page) => Some(room_directory_page_dto(page)?),
    };
    Ok(RoomDirectorySearchDto {
        session_generation,
        request_id,
        status: status.to_owned(),
        page,
    })
}

impl SharedCore {
    pub(super) async fn later_null_command(
        &self,
        command: &'static str,
        no_session: &'static str,
    ) -> Result<LaterSnapshotDto, LaterCommandError> {
        self.later_command(command, no_session, serde_json::Value::Null)
            .await
    }

    pub(super) async fn later_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<LaterSnapshotDto, LaterCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: LATER_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_later_core_error(no_session, error))?;
        later_snapshot_dto(response.payload)
    }

    pub(super) async fn room_notes_null_command(
        &self,
        command: &'static str,
        no_session: &'static str,
    ) -> Result<RoomNotesSnapshotDto, RoomNotesCommandError> {
        self.room_notes_command(command, no_session, serde_json::Value::Null)
            .await
    }

    pub(super) async fn room_notes_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<RoomNotesSnapshotDto, RoomNotesCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: ROOM_NOTES_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_room_notes_core_error(no_session, error))?;
        room_notes_snapshot_dto(response.payload)
    }

    pub(super) async fn image_pack_null_command(
        &self,
        command: &'static str,
        no_session: &'static str,
    ) -> Result<serde_json::Value, ImagePackCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: IMAGE_PACK_COMMAND_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .map_err(|error| map_image_pack_core_error(no_session, error))?;
        Ok(response.payload)
    }

    pub(super) async fn image_pack_set_content(
        &self,
        command: &'static str,
        no_session: &'static str,
        content_json: String,
    ) -> Result<ImagePackWriteDto, ImagePackCommandError> {
        let content = parse_image_pack_content_json(&content_json)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: IMAGE_PACK_COMMAND_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "content": content }),
            })
            .await
            .map_err(|error| map_image_pack_core_error(no_session, error))?;
        image_pack_write_dto(response.payload)
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SharedCore {
    pub async fn get_global_image_packs(
        &self,
    ) -> Result<GlobalImagePacksSnapshotDto, ImagePackCommandError> {
        let payload = self
            .image_pack_null_command(
                GET_GLOBAL_IMAGE_PACKS_COMMAND,
                GET_GLOBAL_IMAGE_PACKS_NO_SESSION_CODE,
            )
            .await?;
        let snapshot: NativeGlobalImagePacksSnapshot =
            serde_json::from_value(payload).map_err(|_| {
                image_pack_failed(IMAGE_PACK_FAILED_CODE, IMAGE_PACK_FAILED_DESCRIPTION)
            })?;
        Ok(GlobalImagePacksSnapshotDto {
            session_generation: snapshot.session_generation,
            packs: snapshot
                .packs
                .into_iter()
                .map(image_pack_dto)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    pub async fn get_user_image_pack(
        &self,
    ) -> Result<UserImagePackSnapshotDto, ImagePackCommandError> {
        let payload = self
            .image_pack_null_command(
                GET_USER_IMAGE_PACK_COMMAND,
                GET_USER_IMAGE_PACK_NO_SESSION_CODE,
            )
            .await?;
        let snapshot: NativeUserImagePackSnapshot =
            serde_json::from_value(payload).map_err(|_| {
                image_pack_failed(IMAGE_PACK_FAILED_CODE, IMAGE_PACK_FAILED_DESCRIPTION)
            })?;
        Ok(UserImagePackSnapshotDto {
            session_generation: snapshot.session_generation,
            pack: snapshot.pack.map(image_pack_dto).transpose()?,
        })
    }

    pub async fn get_room_image_packs(
        &self,
        room_id: String,
    ) -> Result<RoomImagePacksSnapshotDto, ImagePackCommandError> {
        let payload = self
            .core
            .command(CommandEnvelope {
                command: GET_ROOM_IMAGE_PACKS_COMMAND.to_owned(),
                session_generation: IMAGE_PACK_COMMAND_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "roomId": room_id }),
            })
            .await
            .map_err(|error| {
                map_image_pack_core_error(GET_ROOM_IMAGE_PACKS_NO_SESSION_CODE, error)
            })?;
        let snapshot: NativeRoomImagePacksSnapshot = serde_json::from_value(payload.payload)
            .map_err(|_| {
                image_pack_failed(IMAGE_PACK_FAILED_CODE, IMAGE_PACK_FAILED_DESCRIPTION)
            })?;
        Ok(RoomImagePacksSnapshotDto {
            session_generation: snapshot.session_generation,
            room_id: snapshot.room_id,
            packs: snapshot
                .packs
                .into_iter()
                .map(image_pack_dto)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    pub async fn set_user_image_pack(
        &self,
        content_json: String,
    ) -> Result<ImagePackWriteDto, ImagePackCommandError> {
        self.image_pack_set_content(
            SET_USER_IMAGE_PACK_COMMAND,
            SET_USER_IMAGE_PACK_NO_SESSION_CODE,
            content_json,
        )
        .await
    }

    pub async fn set_global_image_packs(
        &self,
        content_json: String,
    ) -> Result<ImagePackWriteDto, ImagePackCommandError> {
        self.image_pack_set_content(
            SET_GLOBAL_IMAGE_PACKS_COMMAND,
            SET_GLOBAL_IMAGE_PACKS_NO_SESSION_CODE,
            content_json,
        )
        .await
    }

    pub async fn set_room_image_pack(
        &self,
        room_id: String,
        state_key: String,
        content_json: String,
    ) -> Result<ImagePackWriteDto, ImagePackCommandError> {
        let content = parse_image_pack_content_json(&content_json)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: SET_ROOM_IMAGE_PACK_COMMAND.to_owned(),
                session_generation: IMAGE_PACK_COMMAND_GENERATION,
                request_id: None,
                payload: serde_json::json!({
                    "roomId": room_id,
                    "stateKey": state_key,
                    "content": content,
                }),
            })
            .await
            .map_err(|error| {
                map_image_pack_core_error(SET_ROOM_IMAGE_PACK_NO_SESSION_CODE, error)
            })?;
        image_pack_write_dto(response.payload)
    }

    pub async fn later_snapshot(&self) -> Result<LaterSnapshotDto, LaterCommandError> {
        self.later_null_command(LATER_SNAPSHOT_COMMAND, LATER_SNAPSHOT_NO_SESSION_CODE)
            .await
    }

    pub async fn later_upsert(
        &self,
        item: LaterItemDto,
    ) -> Result<LaterSnapshotDto, LaterCommandError> {
        let item = later_item_from_dto(item)?;
        let payload = later_envelope_payload(serde_json::json!({ "item": item }))?;
        self.later_command(LATER_UPSERT_COMMAND, LATER_UPSERT_NO_SESSION_CODE, payload)
            .await
    }

    pub async fn later_complete(
        &self,
        item_id: String,
        completed_at: Option<f64>,
    ) -> Result<LaterSnapshotDto, LaterCommandError> {
        let payload = later_envelope_payload(serde_json::json!({
            "itemId": item_id,
            "completedAt": completed_at,
        }))?;
        self.later_command(
            LATER_COMPLETE_COMMAND,
            LATER_COMPLETE_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn later_snooze(
        &self,
        item_id: String,
        due_ts: f64,
    ) -> Result<LaterSnapshotDto, LaterCommandError> {
        let payload = later_envelope_payload(serde_json::json!({
            "itemId": item_id,
            "dueTs": due_ts,
        }))?;
        self.later_command(LATER_SNOOZE_COMMAND, LATER_SNOOZE_NO_SESSION_CODE, payload)
            .await
    }

    pub async fn later_clear_completed(&self) -> Result<LaterSnapshotDto, LaterCommandError> {
        self.later_null_command(
            LATER_CLEAR_COMPLETED_COMMAND,
            LATER_CLEAR_COMPLETED_NO_SESSION_CODE,
        )
        .await
    }

    pub async fn later_mark_reminded(
        &self,
        item_id: String,
        reminded_at: Option<f64>,
    ) -> Result<LaterSnapshotDto, LaterCommandError> {
        let payload = later_envelope_payload(serde_json::json!({
            "itemId": item_id,
            "remindedAt": reminded_at,
        }))?;
        self.later_command(
            LATER_MARK_REMINDED_COMMAND,
            LATER_MARK_REMINDED_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn mdirect_snapshot(&self) -> Result<MDirectSnapshotDto, MDirectCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: MDIRECT_SNAPSHOT_COMMAND.to_owned(),
                session_generation: MDIRECT_COMMAND_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .map_err(|error| map_mdirect_core_error(MDIRECT_SNAPSHOT_NO_SESSION_CODE, error))?;
        mdirect_snapshot_dto(response.payload)
    }

    pub async fn mdirect_add(
        &self,
        room_id: String,
        user_id: String,
    ) -> Result<MDirectMutationDto, MDirectCommandError> {
        let payload = mdirect_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "userId": user_id,
        }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: MDIRECT_ADD_COMMAND.to_owned(),
                session_generation: MDIRECT_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_mdirect_core_error(MDIRECT_ADD_NO_SESSION_CODE, error))?;
        mdirect_mutation_dto(response.payload)
    }

    pub async fn mdirect_remove(
        &self,
        room_id: String,
    ) -> Result<MDirectMutationDto, MDirectCommandError> {
        let payload = mdirect_envelope_payload(serde_json::json!({ "roomId": room_id }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: MDIRECT_REMOVE_COMMAND.to_owned(),
                session_generation: MDIRECT_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_mdirect_core_error(MDIRECT_REMOVE_NO_SESSION_CODE, error))?;
        mdirect_mutation_dto(response.payload)
    }

    pub async fn room_notes_snapshot(&self) -> Result<RoomNotesSnapshotDto, RoomNotesCommandError> {
        self.room_notes_null_command(
            ROOM_NOTES_SNAPSHOT_COMMAND,
            ROOM_NOTES_SNAPSHOT_NO_SESSION_CODE,
        )
        .await
    }

    pub async fn room_notes_upsert(
        &self,
        item: RoomNoteItemDto,
    ) -> Result<RoomNotesSnapshotDto, RoomNotesCommandError> {
        let item = room_note_item_from_dto(item)?;
        let payload = room_notes_envelope_payload(serde_json::json!({ "item": item }))?;
        self.room_notes_command(
            ROOM_NOTES_UPSERT_COMMAND,
            ROOM_NOTES_UPSERT_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn room_notes_delete(
        &self,
        room_id: String,
        item_id: String,
    ) -> Result<RoomNotesSnapshotDto, RoomNotesCommandError> {
        let payload = room_notes_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "itemId": item_id,
        }))?;
        self.room_notes_command(
            ROOM_NOTES_DELETE_COMMAND,
            ROOM_NOTES_DELETE_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn room_notes_complete_todo(
        &self,
        room_id: String,
        item_id: String,
        completed: bool,
    ) -> Result<RoomNotesSnapshotDto, RoomNotesCommandError> {
        let payload = room_notes_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "itemId": item_id,
            "completed": completed,
        }))?;
        self.room_notes_command(
            ROOM_NOTES_COMPLETE_TODO_COMMAND,
            ROOM_NOTES_COMPLETE_TODO_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn room_notes_move_todo(
        &self,
        room_id: String,
        item_id: String,
        direction: String,
    ) -> Result<RoomNotesSnapshotDto, RoomNotesCommandError> {
        let direction = room_note_move_direction_from_dto(&direction)?;
        let payload = room_notes_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "itemId": item_id,
            "direction": direction,
        }))?;
        self.room_notes_command(
            ROOM_NOTES_MOVE_TODO_COMMAND,
            ROOM_NOTES_MOVE_TODO_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn get_room_directory_visibility(
        &self,
        room_id: String,
        session_generation: u64,
    ) -> Result<RoomDirectoryVisibilityDto, DirectoryVisibilityCommandError> {
        let payload = directory_visibility_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "sessionGeneration": session_generation,
        }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: GET_ROOM_DIRECTORY_VISIBILITY_COMMAND.to_owned(),
                session_generation,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| {
                map_directory_visibility_core_error(
                    GET_ROOM_DIRECTORY_VISIBILITY_NO_SESSION_CODE,
                    error,
                )
            })?;
        room_directory_visibility_dto(response.payload)
    }

    pub async fn set_room_directory_visibility(
        &self,
        room_id: String,
        session_generation: u64,
        visibility: String,
    ) -> Result<RoomDirectoryVisibilityWriteDto, DirectoryVisibilityCommandError> {
        let payload = directory_visibility_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "sessionGeneration": session_generation,
            "visibility": visibility,
        }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: SET_ROOM_DIRECTORY_VISIBILITY_COMMAND.to_owned(),
                session_generation,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| {
                map_directory_visibility_core_error(
                    SET_ROOM_DIRECTORY_VISIBILITY_NO_SESSION_CODE,
                    error,
                )
            })?;
        room_directory_visibility_write_dto(response.payload)
    }

    pub async fn room_directory_protocols(
        &self,
    ) -> Result<RoomDirectoryProtocolsDto, DirectorySearchCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: ROOM_DIRECTORY_PROTOCOLS_COMMAND.to_owned(),
                session_generation: DIRECTORY_SEARCH_ENVELOPE_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .map_err(|error| {
                map_directory_search_core_error(ROOM_DIRECTORY_PROTOCOLS_NO_SESSION_CODE, error)
            })?;
        room_directory_protocols_dto(response.payload)
    }

    #[allow(clippy::too_many_arguments)] // UniFFI preserves the typed Matrix directory query fields.
    pub async fn room_directory_search(
        &self,
        session_generation: u64,
        request_id: u64,
        server_name: Option<String>,
        term: Option<String>,
        room_type: Option<String>,
        third_party_instance_id: Option<String>,
        limit: u64,
        since: Option<String>,
    ) -> Result<RoomDirectorySearchDto, DirectorySearchCommandError> {
        let payload = directory_search_envelope_payload(serde_json::json!({
            "sessionGeneration": session_generation,
            "requestId": request_id,
            "serverName": server_name,
            "term": term,
            "roomType": room_type,
            "thirdPartyInstanceId": third_party_instance_id,
            "limit": limit,
            "since": since,
        }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: ROOM_DIRECTORY_SEARCH_COMMAND.to_owned(),
                session_generation: DIRECTORY_SEARCH_ENVELOPE_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| {
                map_directory_search_core_error(ROOM_DIRECTORY_SEARCH_NO_SESSION_CODE, error)
            })?;
        room_directory_search_dto(response.payload)
    }

    pub async fn room_directory_cancel(
        &self,
        session_generation: u64,
        request_id: u64,
    ) -> Result<RoomDirectorySearchDto, DirectorySearchCommandError> {
        let payload = directory_search_envelope_payload(serde_json::json!({
            "sessionGeneration": session_generation,
            "requestId": request_id,
        }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: ROOM_DIRECTORY_CANCEL_COMMAND.to_owned(),
                session_generation: DIRECTORY_SEARCH_ENVELOPE_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| {
                map_directory_search_core_error(ROOM_DIRECTORY_CANCEL_NO_SESSION_CODE, error)
            })?;
        room_directory_search_dto(response.payload)
    }
}
