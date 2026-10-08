//! Typed SharedCore operations and projections for spaces.

use super::*;
use crate::app::spaces::NativeRestrictedJoinReparentResult;
use crate::app::spaces::NativeSpaceChildMutationResult;
use crate::app::spaces::NativeSpaceChildrenSnapshot;
use crate::app::spaces::NativeSpaceHierarchySnapshot;
use crate::app::spaces::NativeSpaceParentsSnapshot;

/// Privacy-safe space parent row. Child room id plus parent room ids only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SpaceParentEntryDto {
    pub room_id: String,
    pub parent_ids: Vec<String>,
}

/// Privacy-safe space parents snapshot. Entries only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SpaceParentsSnapshotDto {
    pub session_generation: u64,
    pub entries: Vec<SpaceParentEntryDto>,
}

/// Privacy-safe hierarchy room. Metadata only; avatar is an mxc reference.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SpaceHierarchyRoomDto {
    pub room_id: String,
    pub name: Option<String>,
    pub canonical_alias: Option<String>,
    pub topic: Option<String>,
    pub avatar_url: Option<String>,
    pub room_type: Option<String>,
    pub num_joined_members: u64,
    pub join_rule: String,
    pub allowed_room_ids: Vec<String>,
    pub world_readable: bool,
    pub guest_can_join: bool,
}

/// Privacy-safe space hierarchy snapshot. Room metadata only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SpaceHierarchySnapshotDto {
    pub session_generation: u64,
    pub rooms: Vec<SpaceHierarchyRoomDto>,
}

/// Privacy-safe local space-child edge. Room ids, order, suggested, via only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SpaceChildEdgeDto {
    pub parent_id: String,
    pub child_id: String,
    pub order: Option<String>,
    pub suggested: bool,
    pub via: Vec<String>,
    pub origin_server_ts: u64,
}

/// Privacy-safe space children snapshot. Edges only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SpaceChildrenSnapshotDto {
    pub session_generation: u64,
    pub edges: Vec<SpaceChildEdgeDto>,
}

/// Privacy-safe space-child write ack. Room ids and status only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SpaceChildMutationDto {
    pub parent_id: String,
    pub child_id: String,
    pub status: String,
}

/// Privacy-safe restricted-join reparent ack. Room id and status only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RestrictedJoinReparentDto {
    pub room_id: String,
    pub status: String,
}

/// Static fail-closed space error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum SpaceCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SpaceCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SpaceCommandError {}

pub(super) fn space_failed(code: &str, description: &'static str) -> SpaceCommandError {
    SpaceCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_space_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> SpaceCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => space_failed(code, SPACE_NO_SESSION_DESCRIPTION),
        Some(code)
            if code.starts_with("v-rooms.2a-")
                || code.starts_with("v-rooms.2b-")
                || code.starts_with("v-rooms.2c-")
                || code == "v-send.r-room-profile-join-rule-requires-session"
                || code.starts_with("p2-space-")
                || code.starts_with("p2-restricted-join-reparent-") =>
        {
            space_failed(code, SPACE_OWNER_DESCRIPTION)
        }
        _ => space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION),
    }
}

pub(super) fn space_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, SpaceCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION));
    }
    Ok(payload)
}

pub(super) fn closed_space_join_rule(value: &str) -> Option<&'static str> {
    match value {
        "public" => Some("public"),
        "invite" => Some("invite"),
        "knock" => Some("knock"),
        "private" => Some("private"),
        "restricted" => Some("restricted"),
        "knock_restricted" => Some("knock_restricted"),
        _ => None,
    }
}

pub(super) fn closed_space_child_status(value: &str) -> Option<&'static str> {
    match value {
        "updated" => Some("updated"),
        "removed" => Some("removed"),
        "skipped" => Some("skipped"),
        _ => None,
    }
}

pub(super) fn closed_restricted_join_reparent_status(value: &str) -> Option<&'static str> {
    match value {
        "updated" => Some("updated"),
        "skipped" => Some("skipped"),
        _ => None,
    }
}

pub(super) fn required_space_id(value: String) -> Result<String, SpaceCommandError> {
    if value.is_empty() {
        return Err(space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION));
    }
    Ok(value)
}

pub(super) fn space_string_list(values: Vec<String>) -> Result<Vec<String>, SpaceCommandError> {
    values.into_iter().map(required_space_id).collect()
}

pub(super) fn space_parent_entry_dto(
    entry: crate::app::spaces::NativeSpaceParentEntry,
) -> Result<SpaceParentEntryDto, SpaceCommandError> {
    Ok(SpaceParentEntryDto {
        room_id: required_space_id(entry.room_id)?,
        parent_ids: space_string_list(entry.parent_ids)?,
    })
}

pub(super) fn space_parents_snapshot_dto(
    payload: NativeSpaceParentsSnapshot,
) -> Result<SpaceParentsSnapshotDto, SpaceCommandError> {
    let entries = payload
        .entries
        .into_iter()
        .map(space_parent_entry_dto)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SpaceParentsSnapshotDto {
        session_generation: payload.session_generation,
        entries,
    })
}

pub(super) fn space_hierarchy_room_dto(
    room: crate::app::spaces::NativeSpaceHierarchyRoom,
) -> Result<SpaceHierarchyRoomDto, SpaceCommandError> {
    let join_rule = closed_space_join_rule(&room.join_rule)
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    if room.allowed_room_ids.len() > 5_000 {
        return Err(space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION));
    }
    let allowed_room_ids = space_string_list(room.allowed_room_ids)?;
    if !matches!(join_rule, "restricted" | "knock_restricted") && !allowed_room_ids.is_empty() {
        return Err(space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION));
    }
    Ok(SpaceHierarchyRoomDto {
        room_id: required_space_id(room.room_id)?,
        name: room.name,
        canonical_alias: room.canonical_alias,
        topic: room.topic,
        avatar_url: room.avatar_url,
        room_type: room.room_type,
        num_joined_members: room.num_joined_members,
        join_rule: join_rule.to_owned(),
        allowed_room_ids,
        world_readable: room.world_readable,
        guest_can_join: room.guest_can_join,
    })
}

pub(super) fn space_hierarchy_snapshot_dto(
    payload: NativeSpaceHierarchySnapshot,
) -> Result<SpaceHierarchySnapshotDto, SpaceCommandError> {
    let rooms = payload
        .rooms
        .into_iter()
        .map(space_hierarchy_room_dto)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SpaceHierarchySnapshotDto {
        session_generation: payload.session_generation,
        rooms,
    })
}

pub(super) fn space_child_edge_dto(
    edge: crate::app::spaces::NativeSpaceChildEdge,
) -> Result<SpaceChildEdgeDto, SpaceCommandError> {
    Ok(SpaceChildEdgeDto {
        parent_id: required_space_id(edge.parent_id)?,
        child_id: required_space_id(edge.child_id)?,
        order: edge.order,
        suggested: edge.suggested,
        via: space_string_list(edge.via)?,
        origin_server_ts: edge.origin_server_ts,
    })
}

pub(super) fn space_children_snapshot_dto(
    payload: NativeSpaceChildrenSnapshot,
) -> Result<SpaceChildrenSnapshotDto, SpaceCommandError> {
    let edges = payload
        .edges
        .into_iter()
        .map(space_child_edge_dto)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SpaceChildrenSnapshotDto {
        session_generation: payload.session_generation,
        edges,
    })
}

pub(super) fn space_child_mutation_dto(
    payload: NativeSpaceChildMutationResult,
) -> Result<SpaceChildMutationDto, SpaceCommandError> {
    let status = closed_space_child_status(payload.status)
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    Ok(SpaceChildMutationDto {
        parent_id: required_space_id(payload.parent_id)?,
        child_id: required_space_id(payload.child_id)?,
        status: status.to_owned(),
    })
}

pub(super) fn restricted_join_reparent_dto(
    payload: NativeRestrictedJoinReparentResult,
) -> Result<RestrictedJoinReparentDto, SpaceCommandError> {
    let status = closed_restricted_join_reparent_status(payload.status)
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    Ok(RestrictedJoinReparentDto {
        room_id: required_space_id(payload.room_id)?,
        status: status.to_owned(),
    })
}

impl SharedCore {
    pub(super) async fn space_command<T>(
        &self,
        no_session: &'static str,
        request: impl std::future::Future<Output = Result<T, MatrixIpcError>>,
    ) -> Result<T, SpaceCommandError> {
        let response = request
            .await
            .map_err(|error| map_space_core_error(no_session, error))?;
        Ok(response)
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SharedCore {
    pub async fn space_parents_snapshot(
        &self,
    ) -> Result<SpaceParentsSnapshotDto, SpaceCommandError> {
        let payload = self
            .space_command(
                SPACE_PARENTS_SNAPSHOT_NO_SESSION_CODE,
                self.core.space_parents_snapshot(),
            )
            .await?;
        space_parents_snapshot_dto(payload)
    }

    pub async fn space_hierarchy_snapshot(
        &self,
        room_id: String,
    ) -> Result<SpaceHierarchySnapshotDto, SpaceCommandError> {
        space_envelope_payload(serde_json::json!({
            "roomId": room_id,
        }))?;
        let response = self
            .space_command(
                SPACE_HIERARCHY_SNAPSHOT_NO_SESSION_CODE,
                self.core.space_hierarchy_snapshot(
                    crate::core_api::MatrixSpaceHierarchySnapshotRequest { room_id },
                ),
            )
            .await?;
        space_hierarchy_snapshot_dto(response)
    }

    pub async fn space_children_snapshot(
        &self,
    ) -> Result<SpaceChildrenSnapshotDto, SpaceCommandError> {
        let payload = self
            .space_command(
                SPACE_CHILDREN_SNAPSHOT_NO_SESSION_CODE,
                self.core.space_children_snapshot(),
            )
            .await?;
        space_children_snapshot_dto(payload)
    }

    pub async fn space_child_set(
        &self,
        parent_id: String,
        child_id: String,
        via: Vec<String>,
        order: Option<String>,
        suggested: Option<bool>,
    ) -> Result<SpaceChildMutationDto, SpaceCommandError> {
        space_envelope_payload(serde_json::json!({
            "parentId": parent_id,
            "childId": child_id,
            "via": via,
            "order": order,
            "suggested": suggested,
        }))?;
        let response = self
            .space_command(
                SPACE_CHILD_SET_NO_SESSION_CODE,
                self.core
                    .space_child_set(crate::core_api::MatrixSpaceChildSetRequest {
                        parent_id,
                        child_id,
                        via,
                        order,
                        suggested,
                    }),
            )
            .await?;
        space_child_mutation_dto(response)
    }

    pub async fn space_child_remove(
        &self,
        parent_id: String,
        child_id: String,
    ) -> Result<SpaceChildMutationDto, SpaceCommandError> {
        space_envelope_payload(serde_json::json!({
            "parentId": parent_id,
            "childId": child_id,
        }))?;
        let response = self
            .space_command(
                SPACE_CHILD_REMOVE_NO_SESSION_CODE,
                self.core
                    .space_child_remove(crate::core_api::MatrixSpaceChildRemoveRequest {
                        parent_id,
                        child_id,
                    }),
            )
            .await?;
        space_child_mutation_dto(response)
    }

    pub async fn restricted_join_reparent(
        &self,
        room_id: String,
        remove_parent_id: Option<String>,
        add_parent_id: String,
    ) -> Result<RestrictedJoinReparentDto, SpaceCommandError> {
        space_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "removeParentId": remove_parent_id,
            "addParentId": add_parent_id,
        }))?;
        let response = self
            .space_command(
                RESTRICTED_JOIN_REPARENT_NO_SESSION_CODE,
                self.core.restricted_join_reparent(
                    crate::core_api::MatrixRestrictedJoinReparentRequest {
                        room_id,
                        remove_parent_id,
                        add_parent_id,
                    },
                ),
            )
            .await?;
        restricted_join_reparent_dto(response)
    }
}
