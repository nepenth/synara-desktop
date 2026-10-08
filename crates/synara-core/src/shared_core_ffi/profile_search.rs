//! Typed SharedCore operations and projections for profile search.

use super::*;

/// Privacy-safe media upload-size config from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MediaConfigDto {
    pub upload_size: u64,
}

/// Privacy-safe own-profile write ack. Status only; no display name or mxc.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct OwnProfileWriteDto {
    pub status: String,
}

/// Privacy-safe own-profile read. Avatar is an `mxc://` URI only; never bytes.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct OwnProfileDto {
    pub user_id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

/// Static fail-closed own-profile-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum OwnProfileCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for OwnProfileCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for OwnProfileCommandError {}

pub(super) fn own_profile_failed(code: &str, description: &'static str) -> OwnProfileCommandError {
    OwnProfileCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_own_profile_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> OwnProfileCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            own_profile_failed(code, OWN_PROFILE_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-send.r-avatar-") => {
            own_profile_failed(code, OWN_PROFILE_OWNER_DESCRIPTION)
        }
        _ => own_profile_failed(OWN_PROFILE_FAILED_CODE, OWN_PROFILE_FAILED_DESCRIPTION),
    }
}

pub(super) fn own_profile_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, OwnProfileCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(own_profile_failed(
            OWN_PROFILE_FAILED_CODE,
            OWN_PROFILE_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

pub(super) fn own_profile_write_dto(
    payload: serde_json::Value,
) -> Result<OwnProfileWriteDto, OwnProfileCommandError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            own_profile_failed(OWN_PROFILE_FAILED_CODE, OWN_PROFILE_FAILED_DESCRIPTION)
        })?;
    Ok(OwnProfileWriteDto {
        status: status.to_owned(),
    })
}

pub(super) fn own_profile_dto(
    payload: serde_json::Value,
) -> Result<OwnProfileDto, OwnProfileCommandError> {
    let profile: crate::app::user_profile::MatrixOwnProfile = serde_json::from_value(payload)
        .map_err(|_| own_profile_failed(OWN_PROFILE_FAILED_CODE, OWN_PROFILE_FAILED_DESCRIPTION))?;
    let avatar_url = profile.avatar_url.filter(|mxc| mxc.starts_with("mxc://"));
    Ok(OwnProfileDto {
        user_id: profile.user_id,
        display_name: profile.display_name,
        avatar_url,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct OwnProfileUploadDto {
    pub mxc: String,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct IgnoredUsersSnapshotDto {
    pub user_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct IgnoredUsersWriteDto {
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum IgnoredUsersCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for IgnoredUsersCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for IgnoredUsersCommandError {}

pub(super) fn ignored_users_failed(
    code: &str,
    description: &'static str,
) -> IgnoredUsersCommandError {
    IgnoredUsersCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_ignored_users_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> IgnoredUsersCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            ignored_users_failed(code, IGNORED_USERS_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-profile.ignore-") => {
            ignored_users_failed(code, IGNORED_USERS_OWNER_DESCRIPTION)
        }
        _ => ignored_users_failed(IGNORED_USERS_FAILED_CODE, IGNORED_USERS_FAILED_DESCRIPTION),
    }
}

pub(super) fn ignored_users_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, IgnoredUsersCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(ignored_users_failed(
            IGNORED_USERS_FAILED_CODE,
            IGNORED_USERS_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

pub(super) fn ignored_users_snapshot_dto(
    payload: serde_json::Value,
) -> Result<IgnoredUsersSnapshotDto, IgnoredUsersCommandError> {
    let snapshot: crate::app::user_profile::MatrixIgnoredUsersSnapshot =
        serde_json::from_value(payload).map_err(|_| {
            ignored_users_failed(IGNORED_USERS_FAILED_CODE, IGNORED_USERS_FAILED_DESCRIPTION)
        })?;
    Ok(IgnoredUsersSnapshotDto {
        user_ids: snapshot.user_ids,
    })
}

pub(super) fn ignored_users_write_dto(
    payload: serde_json::Value,
) -> Result<IgnoredUsersWriteDto, IgnoredUsersCommandError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            ignored_users_failed(IGNORED_USERS_FAILED_CODE, IGNORED_USERS_FAILED_DESCRIPTION)
        })?;
    Ok(IgnoredUsersWriteDto {
        status: status.to_owned(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct UserDirectoryHitDto {
    pub user_id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct UserDirectorySearchDto {
    pub limited: bool,
    pub results: Vec<UserDirectoryHitDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum UserDirectorySearchError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for UserDirectorySearchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for UserDirectorySearchError {}

pub(super) fn user_directory_search_failed(
    code: &str,
    description: &'static str,
) -> UserDirectorySearchError {
    UserDirectorySearchError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_user_directory_search_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> UserDirectorySearchError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            user_directory_search_failed(code, USER_DIRECTORY_SEARCH_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-search.") || code.starts_with("v-directory.") => {
            user_directory_search_failed(code, USER_DIRECTORY_SEARCH_OWNER_DESCRIPTION)
        }
        _ => user_directory_search_failed(
            USER_DIRECTORY_SEARCH_FAILED_CODE,
            USER_DIRECTORY_SEARCH_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn user_directory_search_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, UserDirectorySearchError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(user_directory_search_failed(
            USER_DIRECTORY_SEARCH_FAILED_CODE,
            USER_DIRECTORY_SEARCH_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

pub(super) fn user_directory_search_dto(
    payload: serde_json::Value,
) -> Result<UserDirectorySearchDto, UserDirectorySearchError> {
    let result: crate::app::user_profile::MatrixUserDirectorySearchResult =
        serde_json::from_value(payload).map_err(|_| {
            user_directory_search_failed(
                USER_DIRECTORY_SEARCH_FAILED_CODE,
                USER_DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let mut results = Vec::with_capacity(result.results.len());
    for hit in result.results {
        if matrix_sdk::ruma::UserId::parse(hit.user_id.as_str()).is_err() {
            continue;
        }
        let avatar_url = hit.avatar_url.filter(|mxc| mxc.starts_with("mxc://"));
        results.push(UserDirectoryHitDto {
            user_id: hit.user_id,
            display_name: hit.display_name,
            avatar_url,
        });
    }
    Ok(UserDirectorySearchDto {
        limited: result.limited,
        results,
    })
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct MessageSearchItemDto {
    pub rank: f64,
    pub event_id: String,
    pub sender: String,
    pub origin_server_ts: u64,
    pub body: String,
    pub room_id: String,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct MessageSearchGroupDto {
    pub room_id: String,
    pub items: Vec<MessageSearchItemDto>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct MessageSearchDto {
    pub next_token: Option<String>,
    pub highlights: Vec<String>,
    pub groups: Vec<MessageSearchGroupDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum MessageSearchError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for MessageSearchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for MessageSearchError {}

pub(super) fn message_search_failed(code: &str, description: &'static str) -> MessageSearchError {
    MessageSearchError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_message_search_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> MessageSearchError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            message_search_failed(code, MESSAGE_SEARCH_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-search.") => {
            message_search_failed(code, MESSAGE_SEARCH_OWNER_DESCRIPTION)
        }
        _ => message_search_failed(
            MESSAGE_SEARCH_FAILED_CODE,
            MESSAGE_SEARCH_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn message_search_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, MessageSearchError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(message_search_failed(
            MESSAGE_SEARCH_FAILED_CODE,
            MESSAGE_SEARCH_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

pub(super) fn message_search_dto(
    payload: serde_json::Value,
) -> Result<MessageSearchDto, MessageSearchError> {
    let result: crate::app::search::MatrixMessageSearchResult = serde_json::from_value(payload)
        .map_err(|_| {
            message_search_failed(
                MESSAGE_SEARCH_FAILED_CODE,
                MESSAGE_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let mut highlights = Vec::new();
    for highlight in result.highlights {
        let trimmed = highlight.trim();
        if trimmed.is_empty() {
            continue;
        }
        if highlights.len() >= crate::app::search::MAX_MESSAGE_SEARCH_HIGHLIGHTS {
            break;
        }
        highlights.push(
            trimmed
                .chars()
                .take(crate::app::search::MAX_MESSAGE_SEARCH_HIGHLIGHT_CHARS)
                .collect(),
        );
    }
    let next_token = result.next_token.filter(|token| {
        !token.is_empty()
            && token.chars().count() <= crate::app::search::MAX_MESSAGE_SEARCH_NEXT_TOKEN_CHARS
            && !token.contains("syt_")
            && !token.contains("access_token")
    });
    let mut groups = Vec::new();
    let mut items_seen = 0usize;
    for group in result.groups {
        if groups.len() >= crate::app::search::MAX_MESSAGE_SEARCH_GROUPS {
            break;
        }
        if !group.room_id.starts_with('!') {
            continue;
        }
        let mut items = Vec::new();
        for item in group.items {
            if items_seen >= crate::app::search::MAX_MESSAGE_SEARCH_ITEMS {
                break;
            }
            if !item.event_id.starts_with('$') || !item.sender.starts_with('@') {
                continue;
            }
            let body = item
                .body
                .chars()
                .take(crate::app::search::MAX_MESSAGE_SEARCH_BODY_CHARS)
                .collect();
            items.push(MessageSearchItemDto {
                rank: item.rank,
                event_id: item.event_id,
                sender: item.sender,
                origin_server_ts: item.origin_server_ts,
                body,
                room_id: group.room_id.clone(),
            });
            items_seen += 1;
        }
        if items.is_empty() {
            continue;
        }
        groups.push(MessageSearchGroupDto {
            room_id: group.room_id,
            items,
        });
    }
    Ok(MessageSearchDto {
        next_token,
        highlights,
        groups,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ThreepidEmailDto {
    pub address: String,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ThreepidSnapshotDto {
    pub emails: Vec<ThreepidEmailDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ThreepidWriteDto {
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ThreepidEmailTokenDto {
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ThreepidAddDto {
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum ThreepidCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for ThreepidCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for ThreepidCommandError {}

pub(super) fn threepid_failed(code: &str, description: &'static str) -> ThreepidCommandError {
    ThreepidCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_threepid_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> ThreepidCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => threepid_failed(code, THREEPID_NO_SESSION_DESCRIPTION),
        Some(code) if code.starts_with("v-threepid.") => {
            threepid_failed(code, THREEPID_OWNER_DESCRIPTION)
        }
        _ => threepid_failed(THREEPID_FAILED_CODE, THREEPID_FAILED_DESCRIPTION),
    }
}

pub(super) fn threepid_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, ThreepidCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(threepid_failed(
            THREEPID_FAILED_CODE,
            THREEPID_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

pub(super) fn threepid_snapshot_dto(
    payload: serde_json::Value,
) -> Result<ThreepidSnapshotDto, ThreepidCommandError> {
    let snapshot: crate::app::user_profile::MatrixThreepidSnapshot =
        serde_json::from_value(payload)
            .map_err(|_| threepid_failed(THREEPID_FAILED_CODE, THREEPID_FAILED_DESCRIPTION))?;
    Ok(ThreepidSnapshotDto {
        emails: snapshot
            .emails
            .into_iter()
            .map(|email| ThreepidEmailDto {
                address: email.address,
            })
            .collect(),
    })
}

pub(super) fn threepid_write_dto(
    payload: serde_json::Value,
) -> Result<ThreepidWriteDto, ThreepidCommandError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .ok_or_else(|| threepid_failed(THREEPID_FAILED_CODE, THREEPID_FAILED_DESCRIPTION))?;
    Ok(ThreepidWriteDto {
        status: status.to_owned(),
    })
}

pub(super) fn threepid_email_token_dto(
    payload: serde_json::Value,
) -> Result<ThreepidEmailTokenDto, ThreepidCommandError> {
    let result: crate::app::user_profile::MatrixThreepidEmailTokenResult =
        serde_json::from_value(payload)
            .map_err(|_| threepid_failed(THREEPID_FAILED_CODE, THREEPID_FAILED_DESCRIPTION))?;
    Ok(ThreepidEmailTokenDto {
        session_id: result.session_id,
    })
}

pub(super) fn threepid_add_dto(
    payload: serde_json::Value,
) -> Result<ThreepidAddDto, ThreepidCommandError> {
    let result: crate::app::user_profile::MatrixThreepidAddResult = serde_json::from_value(payload)
        .map_err(|_| threepid_failed(THREEPID_FAILED_CODE, THREEPID_FAILED_DESCRIPTION))?;
    Ok(ThreepidAddDto {
        status: result.status,
    })
}

#[derive(Debug, Deserialize)]
pub(super) struct MediaConfigResultWire {
    #[serde(rename = "m.upload.size")]
    pub(super) upload_size: u64,
}

pub(super) fn media_config_dto(
    payload: serde_json::Value,
) -> Result<MediaConfigDto, SessionStatusError> {
    let result: MediaConfigResultWire = serde_json::from_value(payload).map_err(|_| {
        session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        )
    })?;
    Ok(MediaConfigDto {
        upload_size: result.upload_size,
    })
}

impl SharedCore {
    pub(super) async fn own_profile_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, OwnProfileCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: OWN_PROFILE_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_own_profile_core_error(no_session, error))?;
        Ok(response.payload)
    }

    pub(super) async fn ignored_users_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, IgnoredUsersCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: IGNORED_USERS_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_ignored_users_core_error(no_session, error))?;
        Ok(response.payload)
    }

    pub(super) async fn threepid_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, ThreepidCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: THREEPID_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_threepid_core_error(no_session, error))?;
        Ok(response.payload)
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SharedCore {
    pub async fn set_own_display_name(
        &self,
        display_name: String,
    ) -> Result<OwnProfileWriteDto, OwnProfileCommandError> {
        let payload = own_profile_envelope_payload(serde_json::json!({
            "displayName": display_name,
        }))?;
        self.own_profile_command(
            SET_OWN_DISPLAY_NAME_COMMAND,
            SET_OWN_DISPLAY_NAME_NO_SESSION_CODE,
            payload,
        )
        .await
        .and_then(own_profile_write_dto)
    }

    pub async fn set_own_avatar(
        &self,
        mxc: String,
    ) -> Result<OwnProfileWriteDto, OwnProfileCommandError> {
        let payload = own_profile_envelope_payload(serde_json::json!({ "mxc": mxc }))?;
        self.own_profile_command(
            SET_OWN_AVATAR_COMMAND,
            SET_OWN_AVATAR_NO_SESSION_CODE,
            payload,
        )
        .await
        .and_then(own_profile_write_dto)
    }

    pub async fn get_own_profile(&self) -> Result<OwnProfileDto, OwnProfileCommandError> {
        self.own_profile_command(
            GET_OWN_PROFILE_COMMAND,
            GET_OWN_PROFILE_NO_SESSION_CODE,
            serde_json::Value::Null,
        )
        .await
        .and_then(own_profile_dto)
    }

    pub async fn ignored_users_snapshot(
        &self,
    ) -> Result<IgnoredUsersSnapshotDto, IgnoredUsersCommandError> {
        self.ignored_users_command(
            IGNORED_USERS_SNAPSHOT_COMMAND,
            IGNORED_USERS_SNAPSHOT_NO_SESSION_CODE,
            serde_json::Value::Null,
        )
        .await
        .and_then(ignored_users_snapshot_dto)
    }

    pub async fn ignored_users_ignore(
        &self,
        user_id: String,
    ) -> Result<IgnoredUsersWriteDto, IgnoredUsersCommandError> {
        let payload = ignored_users_envelope_payload(serde_json::json!({ "userId": user_id }))?;
        self.ignored_users_command(
            IGNORED_USERS_IGNORE_COMMAND,
            IGNORED_USERS_IGNORE_NO_SESSION_CODE,
            payload,
        )
        .await
        .and_then(ignored_users_write_dto)
    }

    pub async fn ignored_users_unignore(
        &self,
        user_id: String,
    ) -> Result<IgnoredUsersWriteDto, IgnoredUsersCommandError> {
        let payload = ignored_users_envelope_payload(serde_json::json!({ "userId": user_id }))?;
        self.ignored_users_command(
            IGNORED_USERS_UNIGNORE_COMMAND,
            IGNORED_USERS_UNIGNORE_NO_SESSION_CODE,
            payload,
        )
        .await
        .and_then(ignored_users_write_dto)
    }

    pub async fn user_directory_search(
        &self,
        term: String,
        limit: Option<u64>,
    ) -> Result<UserDirectorySearchDto, UserDirectorySearchError> {
        let payload = user_directory_search_envelope_payload(serde_json::json!({
            "term": term,
            "limit": limit,
        }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: USER_DIRECTORY_SEARCH_COMMAND.to_owned(),
                session_generation: USER_DIRECTORY_SEARCH_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| {
                map_user_directory_search_core_error(USER_DIRECTORY_SEARCH_NO_SESSION_CODE, error)
            })?;
        user_directory_search_dto(response.payload)
    }

    pub async fn message_search(
        &self,
        term: String,
        next_token: Option<String>,
        rooms: Option<Vec<String>>,
        senders: Option<Vec<String>>,
        order: Option<String>,
    ) -> Result<MessageSearchDto, MessageSearchError> {
        let payload = message_search_envelope_payload(serde_json::json!({
            "term": term,
            "nextToken": next_token,
            "rooms": rooms,
            "senders": senders,
            "order": order,
        }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: MESSAGE_SEARCH_COMMAND.to_owned(),
                session_generation: MESSAGE_SEARCH_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| {
                map_message_search_core_error(MESSAGE_SEARCH_NO_SESSION_CODE, error)
            })?;
        message_search_dto(response.payload)
    }

    pub async fn threepid_snapshot(&self) -> Result<ThreepidSnapshotDto, ThreepidCommandError> {
        self.threepid_command(
            THREEPID_SNAPSHOT_COMMAND,
            THREEPID_SNAPSHOT_NO_SESSION_CODE,
            serde_json::Value::Null,
        )
        .await
        .and_then(threepid_snapshot_dto)
    }

    pub async fn threepid_delete(
        &self,
        address: String,
    ) -> Result<ThreepidWriteDto, ThreepidCommandError> {
        let payload = threepid_envelope_payload(serde_json::json!({ "address": address }))?;
        self.threepid_command(
            THREEPID_DELETE_COMMAND,
            THREEPID_DELETE_NO_SESSION_CODE,
            payload,
        )
        .await
        .and_then(threepid_write_dto)
    }

    pub async fn threepid_request_email_token(
        &self,
        email: String,
    ) -> Result<ThreepidEmailTokenDto, ThreepidCommandError> {
        let payload = threepid_envelope_payload(serde_json::json!({ "email": email }))?;
        self.threepid_command(
            THREEPID_REQUEST_EMAIL_TOKEN_COMMAND,
            THREEPID_REQUEST_EMAIL_TOKEN_NO_SESSION_CODE,
            payload,
        )
        .await
        .and_then(threepid_email_token_dto)
    }

    pub async fn threepid_add_email(&self) -> Result<ThreepidAddDto, ThreepidCommandError> {
        self.threepid_command(
            THREEPID_ADD_EMAIL_COMMAND,
            THREEPID_ADD_EMAIL_NO_SESSION_CODE,
            serde_json::Value::Null,
        )
        .await
        .and_then(threepid_add_dto)
    }

    pub async fn threepid_add_email_password(
        &self,
        password: String,
    ) -> Result<ThreepidAddDto, ThreepidCommandError> {
        if password.len() > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
            return Err(threepid_failed(
                THREEPID_FAILED_CODE,
                THREEPID_FAILED_DESCRIPTION,
            ));
        }
        let result = self
            .core
            .threepid_add_email_password(&password)
            .await
            .map_err(|error| {
                map_threepid_core_error(THREEPID_ADD_EMAIL_PASSWORD_NO_SESSION_CODE, error)
            })?;
        drop(password);
        Ok(ThreepidAddDto {
            status: result.status,
        })
    }

    pub async fn media_config(&self) -> Result<MediaConfigDto, SessionStatusError> {
        let payload = self.session_status_command(MEDIA_CONFIG_COMMAND).await?;
        media_config_dto(payload)
    }
}
