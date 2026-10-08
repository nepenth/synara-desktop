//! Typed SharedCore operations and projections for profile search.

use super::*;
use crate::app::search::MatrixMessageSearchResult;
use crate::app::user_profile::MatrixIgnoredUsersSnapshot;
use crate::app::user_profile::MatrixIgnoredUsersWriteResult;
use crate::app::user_profile::MatrixOwnProfile;
use crate::app::user_profile::MatrixProfileWriteResult;
use crate::app::user_profile::MatrixThreepidAddResult;
use crate::app::user_profile::MatrixThreepidEmailTokenResult;
use crate::app::user_profile::MatrixThreepidSnapshot;
use crate::app::user_profile::MatrixThreepidWriteResult;
use crate::app::user_profile::MatrixUserDirectorySearchResult;
use crate::core_api::MatrixMediaConfigResponse;

/// Privacy-safe media upload-size config from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MediaConfigDto {
    pub upload_size: u64,
}

/// Privacy-safe own-profile write ack. Status only; no display name or mxc.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct OwnProfileWriteDto {
    pub status: WriteAckDto,
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
    payload: MatrixProfileWriteResult,
) -> Result<OwnProfileWriteDto, OwnProfileCommandError> {
    let status = Some(payload.status).ok_or_else(|| {
        own_profile_failed(OWN_PROFILE_FAILED_CODE, OWN_PROFILE_FAILED_DESCRIPTION)
    })?;
    Ok(OwnProfileWriteDto {
        status: status.into(),
    })
}

pub(super) fn own_profile_dto(
    payload: MatrixOwnProfile,
) -> Result<OwnProfileDto, OwnProfileCommandError> {
    let profile: crate::app::user_profile::MatrixOwnProfile = payload;
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
    pub status: WriteAckDto,
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
    payload: MatrixIgnoredUsersSnapshot,
) -> Result<IgnoredUsersSnapshotDto, IgnoredUsersCommandError> {
    let snapshot: crate::app::user_profile::MatrixIgnoredUsersSnapshot = payload;
    Ok(IgnoredUsersSnapshotDto {
        user_ids: snapshot.user_ids,
    })
}

pub(super) fn ignored_users_write_dto(
    payload: MatrixIgnoredUsersWriteResult,
) -> Result<IgnoredUsersWriteDto, IgnoredUsersCommandError> {
    let status = Some(payload.status).ok_or_else(|| {
        ignored_users_failed(IGNORED_USERS_FAILED_CODE, IGNORED_USERS_FAILED_DESCRIPTION)
    })?;
    Ok(IgnoredUsersWriteDto {
        status: status.into(),
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
    payload: MatrixUserDirectorySearchResult,
) -> Result<UserDirectorySearchDto, UserDirectorySearchError> {
    let result: crate::app::user_profile::MatrixUserDirectorySearchResult = payload;
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
    payload: MatrixMessageSearchResult,
) -> Result<MessageSearchDto, MessageSearchError> {
    let result: crate::app::search::MatrixMessageSearchResult = payload;
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
    pub status: WriteAckDto,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ThreepidEmailTokenDto {
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ThreepidAddDto {
    pub status: ThreepidAddStatusDto,
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
    payload: MatrixThreepidSnapshot,
) -> Result<ThreepidSnapshotDto, ThreepidCommandError> {
    let snapshot: crate::app::user_profile::MatrixThreepidSnapshot = payload;
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
    payload: MatrixThreepidWriteResult,
) -> Result<ThreepidWriteDto, ThreepidCommandError> {
    let status = Some(payload.status)
        .ok_or_else(|| threepid_failed(THREEPID_FAILED_CODE, THREEPID_FAILED_DESCRIPTION))?;
    Ok(ThreepidWriteDto {
        status: status.into(),
    })
}

pub(super) fn threepid_email_token_dto(
    payload: MatrixThreepidEmailTokenResult,
) -> Result<ThreepidEmailTokenDto, ThreepidCommandError> {
    let result: crate::app::user_profile::MatrixThreepidEmailTokenResult = payload;
    Ok(ThreepidEmailTokenDto {
        session_id: result.session_id,
    })
}

pub(super) fn threepid_add_dto(
    payload: MatrixThreepidAddResult,
) -> Result<ThreepidAddDto, ThreepidCommandError> {
    let result: crate::app::user_profile::MatrixThreepidAddResult = payload;
    Ok(ThreepidAddDto {
        status: result.status.into(),
    })
}

pub(super) fn media_config_dto(result: MatrixMediaConfigResponse) -> MediaConfigDto {
    MediaConfigDto {
        upload_size: result.upload_size,
    }
}

impl SharedCore {
    pub(super) async fn own_profile_command<T>(
        &self,
        no_session: &'static str,
        request: impl std::future::Future<Output = Result<T, MatrixIpcError>>,
    ) -> Result<T, OwnProfileCommandError> {
        let response = request
            .await
            .map_err(|error| map_own_profile_core_error(no_session, error))?;
        Ok(response)
    }

    pub(super) async fn ignored_users_command<T>(
        &self,
        no_session: &'static str,
        request: impl std::future::Future<Output = Result<T, MatrixIpcError>>,
    ) -> Result<T, IgnoredUsersCommandError> {
        let response = request
            .await
            .map_err(|error| map_ignored_users_core_error(no_session, error))?;
        Ok(response)
    }

    pub(super) async fn threepid_command<T>(
        &self,
        no_session: &'static str,
        request: impl std::future::Future<Output = Result<T, MatrixIpcError>>,
    ) -> Result<T, ThreepidCommandError> {
        let response = request
            .await
            .map_err(|error| map_threepid_core_error(no_session, error))?;
        Ok(response)
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SharedCore {
    pub async fn set_own_display_name(
        &self,
        display_name: String,
    ) -> Result<OwnProfileWriteDto, OwnProfileCommandError> {
        own_profile_envelope_payload(serde_json::json!({
            "displayName": display_name}))?;
        self.own_profile_command(
            SET_OWN_DISPLAY_NAME_NO_SESSION_CODE,
            self.core
                .set_own_display_name(crate::core_api::MatrixSetOwnDisplayNameRequest {
                    display_name,
                }),
        )
        .await
        .and_then(own_profile_write_dto)
    }

    pub async fn set_own_avatar(
        &self,
        mxc: String,
    ) -> Result<OwnProfileWriteDto, OwnProfileCommandError> {
        own_profile_envelope_payload(serde_json::json!({ "mxc": mxc }))?;
        self.own_profile_command(
            SET_OWN_AVATAR_NO_SESSION_CODE,
            self.core
                .set_own_avatar(crate::core_api::MatrixSetOwnAvatarRequest { mxc }),
        )
        .await
        .and_then(own_profile_write_dto)
    }

    pub async fn get_own_profile(&self) -> Result<OwnProfileDto, OwnProfileCommandError> {
        self.own_profile_command(GET_OWN_PROFILE_NO_SESSION_CODE, self.core.get_own_profile())
            .await
            .and_then(own_profile_dto)
    }

    pub async fn ignored_users_snapshot(
        &self,
    ) -> Result<IgnoredUsersSnapshotDto, IgnoredUsersCommandError> {
        self.ignored_users_command(
            IGNORED_USERS_SNAPSHOT_NO_SESSION_CODE,
            self.core.ignored_users_snapshot(),
        )
        .await
        .and_then(ignored_users_snapshot_dto)
    }

    pub async fn ignored_users_ignore(
        &self,
        user_id: String,
    ) -> Result<IgnoredUsersWriteDto, IgnoredUsersCommandError> {
        ignored_users_envelope_payload(serde_json::json!({ "userId": user_id }))?;
        self.ignored_users_command(
            IGNORED_USERS_IGNORE_NO_SESSION_CODE,
            self.core
                .ignored_users_ignore(crate::core_api::MatrixIgnoredUsersUserRequest { user_id }),
        )
        .await
        .and_then(ignored_users_write_dto)
    }

    pub async fn ignored_users_unignore(
        &self,
        user_id: String,
    ) -> Result<IgnoredUsersWriteDto, IgnoredUsersCommandError> {
        ignored_users_envelope_payload(serde_json::json!({ "userId": user_id }))?;
        self.ignored_users_command(
            IGNORED_USERS_UNIGNORE_NO_SESSION_CODE,
            self.core
                .ignored_users_unignore(crate::core_api::MatrixIgnoredUsersUserRequest { user_id }),
        )
        .await
        .and_then(ignored_users_write_dto)
    }

    pub async fn user_directory_search(
        &self,
        term: String,
        limit: Option<u64>,
    ) -> Result<UserDirectorySearchDto, UserDirectorySearchError> {
        user_directory_search_envelope_payload(serde_json::json!({
            "term": term,
            "limit": limit}))?;
        let response = self
            .core
            .user_directory_search(crate::core_api::MatrixUserDirectorySearchRequest {
                term,
                limit,
            })
            .await
            .map_err(|error| {
                map_user_directory_search_core_error(USER_DIRECTORY_SEARCH_NO_SESSION_CODE, error)
            })?;
        user_directory_search_dto(response)
    }

    pub async fn message_search(
        &self,
        term: String,
        next_token: Option<String>,
        rooms: Option<Vec<String>>,
        senders: Option<Vec<String>>,
        order: Option<String>,
    ) -> Result<MessageSearchDto, MessageSearchError> {
        message_search_envelope_payload(serde_json::json!({
            "term": term,
            "nextToken": next_token,
            "rooms": rooms,
            "senders": senders,
            "order": order}))?;
        let response = self
            .core
            .message_search(crate::core_api::MatrixMessageSearchRequest {
                from_ts: Default::default(),
                listing_kind: Default::default(),
                to_ts: Default::default(),
                term,
                next_token,
                rooms,
                senders,
                order,
            })
            .await
            .map_err(|error| {
                map_message_search_core_error(MESSAGE_SEARCH_NO_SESSION_CODE, error)
            })?;
        message_search_dto(response)
    }

    pub async fn threepid_snapshot(&self) -> Result<ThreepidSnapshotDto, ThreepidCommandError> {
        self.threepid_command(
            THREEPID_SNAPSHOT_NO_SESSION_CODE,
            self.core.threepid_snapshot(),
        )
        .await
        .and_then(threepid_snapshot_dto)
    }

    pub async fn threepid_delete(
        &self,
        address: String,
    ) -> Result<ThreepidWriteDto, ThreepidCommandError> {
        threepid_envelope_payload(serde_json::json!({ "address": address }))?;
        self.threepid_command(
            THREEPID_DELETE_NO_SESSION_CODE,
            self.core
                .threepid_delete(crate::core_api::MatrixThreepidAddressRequest { address }),
        )
        .await
        .and_then(threepid_write_dto)
    }

    pub async fn threepid_request_email_token(
        &self,
        email: String,
    ) -> Result<ThreepidEmailTokenDto, ThreepidCommandError> {
        threepid_envelope_payload(serde_json::json!({ "email": email }))?;
        self.threepid_command(
            THREEPID_REQUEST_EMAIL_TOKEN_NO_SESSION_CODE,
            self.core
                .threepid_request_email_token(crate::core_api::MatrixThreepidEmailRequest {
                    email,
                }),
        )
        .await
        .and_then(threepid_email_token_dto)
    }

    pub async fn threepid_add_email(&self) -> Result<ThreepidAddDto, ThreepidCommandError> {
        self.threepid_command(
            THREEPID_ADD_EMAIL_NO_SESSION_CODE,
            self.core.threepid_add_email(),
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
            status: result.status.into(),
        })
    }

    pub async fn media_config(&self) -> Result<MediaConfigDto, SessionStatusError> {
        let payload = self
            .session_status_command(self.core.media_config())
            .await?;
        Ok(media_config_dto(payload))
    }
}
