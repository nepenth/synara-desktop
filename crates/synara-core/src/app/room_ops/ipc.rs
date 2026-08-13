//! Credential-free room-create IPC request DTOs.
//!
//! Live Client create-room I/O stays in the desktop shell.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// JSON-friendly create-room request owned by the native Matrix route.
/// `parent_room_id` is used for restricted join rules; the post-create space
/// child edge remains an explicit `matrix_space_child_set` operation in TS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomCreateRequest {
    pub name: Option<String>,
    pub topic: Option<String>,
    pub room_version: Option<String>,
    pub room_alias_name: Option<String>,
    #[serde(default)]
    pub is_direct: bool,
    #[serde(default)]
    pub invite: Vec<String>,
    pub visibility: Option<MatrixRoomCreateVisibility>,
    pub preset: Option<MatrixRoomCreatePreset>,
    pub creation_content: Option<MatrixRoomCreateContent>,
    #[serde(default)]
    pub encryption: bool,
    pub join_rule: Option<String>,
    #[serde(default)]
    pub knock: bool,
    pub parent_room_id: Option<String>,
    pub power_level_content_override: Option<MatrixRoomCreatePowerLevels>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatrixRoomCreateVisibility {
    Private,
    Public,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatrixRoomCreatePreset {
    #[serde(rename = "private_chat")]
    Private,
    #[serde(rename = "public_chat")]
    Public,
    #[serde(rename = "trusted_private_chat")]
    TrustedPrivate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomCreateContent {
    #[serde(rename = "type")]
    pub room_type: Option<String>,
    #[serde(rename = "m.federate", alias = "federate")]
    pub federate: Option<bool>,
    pub additional_creators: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomCreatePowerLevels {
    pub events_default: Option<i64>,
    #[serde(default)]
    pub events: BTreeMap<String, i64>,
}
