//! Typed SharedCore operations and projections for spaces.

use super::*;

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

pub(super) fn required_space_id(
    value: Option<&serde_json::Value>,
) -> Result<String, SpaceCommandError> {
    value
        .and_then(|item| item.as_str())
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))
}

pub(super) fn optional_space_string(value: Option<&serde_json::Value>) -> Option<String> {
    value.and_then(|item| item.as_str()).map(ToOwned::to_owned)
}

pub(super) fn space_string_list(
    value: Option<&serde_json::Value>,
) -> Result<Vec<String>, SpaceCommandError> {
    value
        .and_then(|item| item.as_array())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?
        .iter()
        .map(|item| required_space_id(Some(item)))
        .collect()
}

pub(super) fn space_parent_entry_dto(
    value: &serde_json::Value,
) -> Result<SpaceParentEntryDto, SpaceCommandError> {
    Ok(SpaceParentEntryDto {
        room_id: required_space_id(value.get("roomId"))?,
        parent_ids: space_string_list(value.get("parentIds"))?,
    })
}

pub(super) fn space_parents_snapshot_dto(
    payload: serde_json::Value,
) -> Result<SpaceParentsSnapshotDto, SpaceCommandError> {
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    let entries = payload
        .get("entries")
        .and_then(|value| value.as_array())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?
        .iter()
        .map(space_parent_entry_dto)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SpaceParentsSnapshotDto {
        session_generation,
        entries,
    })
}

pub(super) fn space_hierarchy_room_dto(
    value: &serde_json::Value,
) -> Result<SpaceHierarchyRoomDto, SpaceCommandError> {
    let join_rule = value
        .get("joinRule")
        .and_then(|item| item.as_str())
        .and_then(closed_space_join_rule)
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    let num_joined_members = value
        .get("numJoinedMembers")
        .and_then(|item| item.as_u64())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    let world_readable = value
        .get("worldReadable")
        .and_then(|item| item.as_bool())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    let guest_can_join = value
        .get("guestCanJoin")
        .and_then(|item| item.as_bool())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    let allowed_room_ids = value
        .get("allowedRoomIds")
        .and_then(|item| item.as_array())
        .filter(|items| items.len() <= 5_000)
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?
        .iter()
        .map(|item| required_space_id(Some(item)))
        .collect::<Result<Vec<_>, _>>()?;
    if !matches!(join_rule, "restricted" | "knock_restricted") && !allowed_room_ids.is_empty() {
        return Err(space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION));
    }
    Ok(SpaceHierarchyRoomDto {
        room_id: required_space_id(value.get("roomId"))?,
        name: optional_space_string(value.get("name")),
        canonical_alias: optional_space_string(value.get("canonicalAlias")),
        topic: optional_space_string(value.get("topic")),
        avatar_url: optional_space_string(value.get("avatarUrl")),
        room_type: optional_space_string(value.get("roomType")),
        num_joined_members,
        join_rule: join_rule.to_owned(),
        allowed_room_ids,
        world_readable,
        guest_can_join,
    })
}

pub(super) fn space_hierarchy_snapshot_dto(
    payload: serde_json::Value,
) -> Result<SpaceHierarchySnapshotDto, SpaceCommandError> {
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    let rooms = payload
        .get("rooms")
        .and_then(|value| value.as_array())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?
        .iter()
        .map(space_hierarchy_room_dto)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SpaceHierarchySnapshotDto {
        session_generation,
        rooms,
    })
}

pub(super) fn space_child_edge_dto(
    value: &serde_json::Value,
) -> Result<SpaceChildEdgeDto, SpaceCommandError> {
    let suggested = value
        .get("suggested")
        .and_then(|item| item.as_bool())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    let origin_server_ts = value
        .get("originServerTs")
        .and_then(|item| item.as_u64())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    Ok(SpaceChildEdgeDto {
        parent_id: required_space_id(value.get("parentId"))?,
        child_id: required_space_id(value.get("childId"))?,
        order: optional_space_string(value.get("order")),
        suggested,
        via: space_string_list(value.get("via"))?,
        origin_server_ts,
    })
}

pub(super) fn space_children_snapshot_dto(
    payload: serde_json::Value,
) -> Result<SpaceChildrenSnapshotDto, SpaceCommandError> {
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    let edges = payload
        .get("edges")
        .and_then(|value| value.as_array())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?
        .iter()
        .map(space_child_edge_dto)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SpaceChildrenSnapshotDto {
        session_generation,
        edges,
    })
}

pub(super) fn space_child_mutation_dto(
    payload: serde_json::Value,
) -> Result<SpaceChildMutationDto, SpaceCommandError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .and_then(closed_space_child_status)
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    Ok(SpaceChildMutationDto {
        parent_id: required_space_id(payload.get("parentId"))?,
        child_id: required_space_id(payload.get("childId"))?,
        status: status.to_owned(),
    })
}

pub(super) fn restricted_join_reparent_dto(
    payload: serde_json::Value,
) -> Result<RestrictedJoinReparentDto, SpaceCommandError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .and_then(closed_restricted_join_reparent_status)
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    Ok(RestrictedJoinReparentDto {
        room_id: required_space_id(payload.get("roomId"))?,
        status: status.to_owned(),
    })
}

impl SharedCore {
    pub(super) async fn space_null_command(
        &self,
        command: &'static str,
        no_session: &'static str,
    ) -> Result<serde_json::Value, SpaceCommandError> {
        self.space_command(command, no_session, serde_json::Value::Null)
            .await
    }

    pub(super) async fn space_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, SpaceCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: SPACE_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_space_core_error(no_session, error))?;
        Ok(response.payload)
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SharedCore {
    pub async fn space_parents_snapshot(
        &self,
    ) -> Result<SpaceParentsSnapshotDto, SpaceCommandError> {
        let payload = self
            .space_null_command(
                SPACE_PARENTS_SNAPSHOT_COMMAND,
                SPACE_PARENTS_SNAPSHOT_NO_SESSION_CODE,
            )
            .await?;
        space_parents_snapshot_dto(payload)
    }

    pub async fn space_hierarchy_snapshot(
        &self,
        room_id: String,
    ) -> Result<SpaceHierarchySnapshotDto, SpaceCommandError> {
        let payload = space_envelope_payload(serde_json::json!({
            "roomId": room_id,
        }))?;
        let response = self
            .space_command(
                SPACE_HIERARCHY_SNAPSHOT_COMMAND,
                SPACE_HIERARCHY_SNAPSHOT_NO_SESSION_CODE,
                payload,
            )
            .await?;
        space_hierarchy_snapshot_dto(response)
    }

    pub async fn space_children_snapshot(
        &self,
    ) -> Result<SpaceChildrenSnapshotDto, SpaceCommandError> {
        let payload = self
            .space_null_command(
                SPACE_CHILDREN_SNAPSHOT_COMMAND,
                SPACE_CHILDREN_SNAPSHOT_NO_SESSION_CODE,
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
        let payload = space_envelope_payload(serde_json::json!({
            "parentId": parent_id,
            "childId": child_id,
            "via": via,
            "order": order,
            "suggested": suggested,
        }))?;
        let response = self
            .space_command(
                SPACE_CHILD_SET_COMMAND,
                SPACE_CHILD_SET_NO_SESSION_CODE,
                payload,
            )
            .await?;
        space_child_mutation_dto(response)
    }

    pub async fn space_child_remove(
        &self,
        parent_id: String,
        child_id: String,
    ) -> Result<SpaceChildMutationDto, SpaceCommandError> {
        let payload = space_envelope_payload(serde_json::json!({
            "parentId": parent_id,
            "childId": child_id,
        }))?;
        let response = self
            .space_command(
                SPACE_CHILD_REMOVE_COMMAND,
                SPACE_CHILD_REMOVE_NO_SESSION_CODE,
                payload,
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
        let payload = space_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "removeParentId": remove_parent_id,
            "addParentId": add_parent_id,
        }))?;
        let response = self
            .space_command(
                RESTRICTED_JOIN_REPARENT_COMMAND,
                RESTRICTED_JOIN_REPARENT_NO_SESSION_CODE,
                payload,
            )
            .await?;
        restricted_join_reparent_dto(response)
    }
}
