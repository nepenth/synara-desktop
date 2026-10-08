//! Core command adapters for profile media.

use super::*;

/// Exact React/Tauri payload for `matrix_media_config`.
///
/// The legacy command deliberately has no renderer-supplied input and returns
/// this one-key object verbatim. Keep it independent from the Platform
/// projection: Core owns the public field spelling and serializes only after
/// checking the shared JavaScript-safe counter bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MatrixMediaConfigResponse {
    #[serde(rename = "m.upload.size")]
    pub upload_size: u64,
}

impl MatrixMediaConfigResponse {
    pub(super) fn from_platform(config: PlatformMediaConfig) -> Result<Self, MatrixIpcError> {
        let response = Self {
            upload_size: config.upload_size(),
        };
        (response.upload_size <= MAX_WIRE_COUNTER)
            .then_some(response)
            .ok_or_else(|| core_state_error("p2-media-config-invalid-platform-projection"))
    }
}

/// Exact React/Tauri envelope payload for `matrix_set_own_display_name`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixSetOwnDisplayNameRequest {
    pub display_name: String,
}

/// Exact React/Tauri envelope payload for `matrix_set_own_avatar`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixSetOwnAvatarRequest {
    pub mxc: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixIgnoredUsersUserRequest {
    pub user_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_user_directory_search`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixUserDirectorySearchRequest {
    pub term: String,
    #[serde(default)]
    pub limit: Option<u64>,
}

/// Exact React/Tauri envelope payload for `matrix_message_search`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixMessageSearchRequest {
    pub term: String,
    #[serde(default)]
    pub next_token: Option<String>,
    #[serde(default)]
    pub rooms: Option<Vec<String>>,
    #[serde(default)]
    pub senders: Option<Vec<String>>,
    #[serde(default)]
    pub order: Option<String>,
    #[serde(default)]
    pub listing_kind: Option<String>,
    #[serde(default)]
    pub from_ts: Option<u64>,
    #[serde(default)]
    pub to_ts: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixThreepidAddressRequest {
    pub address: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixThreepidEmailRequest {
    pub email: String,
}

#[cfg(test)]
pub(super) fn own_profile_read_payload_is_empty(payload: &serde_json::Value) -> bool {
    payload.is_null() || payload.as_object().is_some_and(serde_json::Map::is_empty)
}

/// Exact React/Tauri envelope payload for `matrix_media_preview`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixMediaPreviewRequest {
    pub room_id: String,
    pub session_generation: u64,
    pub url: String,
    #[serde(default)]
    pub ts: Option<u64>,
}

/// Typed `matrix_media_preview`.
pub(super) async fn media_preview(
    state: &Arc<CoreState>,
    payload: MatrixMediaPreviewRequest,
) -> Result<MatrixMediaPreviewSnapshot, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-media-preview-no-session")
    })?;
    let result: MatrixMediaPreviewSnapshot = owner
        .get_media_preview(
            &payload.room_id,
            payload.session_generation,
            &payload.url,
            payload.ts,
        )
        .await
        .map_err(media_preview_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_media_preview(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixMediaPreviewRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-media-preview-invalid-payload"))?;
        let response = media_preview(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-media-preview-serialization-failed"))
    })
}

pub(super) fn media_preview_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-send.r-media-preview-invalid"
        | "v-send.r-media-preview-url-invalid"
        | "v-send.r-media-preview-room-not-found" => MatrixIpcErrorCategory::SdkInvariant,
        "v-send.r-media-preview-requires-session" => MatrixIpcErrorCategory::Forbidden,
        "v-send.r-media-preview-stale-generation" => MatrixIpcErrorCategory::StaleSessionGeneration,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

/// Typed `matrix_set_own_display_name`.
pub(super) async fn set_own_display_name(
    state: &Arc<CoreState>,
    payload: MatrixSetOwnDisplayNameRequest,
) -> Result<MatrixProfileWriteResult, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-set-own-display-name-no-session")
    })?;
    let result: MatrixProfileWriteResult = owner
        .set_own_display_name(&payload.display_name)
        .await
        .map_err(own_profile_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_set_own_display_name(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetOwnDisplayNameRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-set-own-display-name-invalid-payload"))?;
        let response = set_own_display_name(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-set-own-display-name-serialization-failed"))
    })
}

/// Typed `matrix_set_own_avatar`.
pub(super) async fn set_own_avatar(
    state: &Arc<CoreState>,
    payload: MatrixSetOwnAvatarRequest,
) -> Result<MatrixProfileWriteResult, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-set-own-avatar-no-session")
    })?;
    let result: MatrixProfileWriteResult = owner
        .set_own_avatar(&payload.mxc)
        .await
        .map_err(own_profile_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_set_own_avatar(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetOwnAvatarRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-set-own-avatar-invalid-payload"))?;
        let response = set_own_avatar(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-set-own-avatar-serialization-failed"))
    })
}

/// Typed `matrix_get_own_profile`.
pub(super) async fn get_own_profile(
    state: &Arc<CoreState>,
) -> Result<MatrixOwnProfile, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-get-own-profile-no-session")
    })?;
    let result: MatrixOwnProfile = owner
        .get_own_profile()
        .await
        .map_err(own_profile_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_get_own_profile(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !own_profile_read_payload_is_empty(&request.payload) {
            return Err(core_state_error("p2-get-own-profile-invalid-payload"));
        }
        let response = get_own_profile(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-get-own-profile-serialization-failed"))
    })
}

pub(super) fn own_profile_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-send.r-avatar-display-name-too-long"
        | "v-send.r-avatar-invalid-mxc"
        | "v-send.r-avatar-upload-empty"
        | "v-send.r-avatar-upload-invalid-mime"
        | "v-send.r-avatar-upload-too-large" => MatrixIpcErrorCategory::SdkInvariant,
        "v-send.r-avatar-profile-no-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn plain_media_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-send.r-media-invalid-content-uri" | "v-send.r-media-download-too-large" => {
            MatrixIpcErrorCategory::SdkInvariant
        }
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn ignored_users_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-profile.ignore-invalid-user" | "v-profile.ignore-self" => {
            MatrixIpcErrorCategory::SdkInvariant
        }
        "v-profile.ignore-no-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn threepid_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-threepid.invalid-email"
        | "v-threepid.password-empty"
        | "v-threepid.not-pending"
        | "v-threepid.auth-unsupported" => MatrixIpcErrorCategory::SdkInvariant,
        "v-threepid.no-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

/// Typed `matrix_ignored_users_snapshot`.
pub(super) async fn ignored_users_snapshot(
    state: &Arc<CoreState>,
) -> Result<MatrixIgnoredUsersSnapshot, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-ignored-users-snapshot-no-session")
    })?;
    let result: MatrixIgnoredUsersSnapshot = owner
        .snapshot_ignored_users()
        .await
        .map_err(ignored_users_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_ignored_users_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !own_profile_read_payload_is_empty(&request.payload) {
            return Err(core_state_error(
                "p2-ignored-users-snapshot-invalid-payload",
            ));
        }
        let response = ignored_users_snapshot(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-ignored-users-snapshot-serialization-failed"))
    })
}

/// Typed `matrix_ignored_users_ignore`.
pub(super) async fn ignored_users_ignore(
    state: &Arc<CoreState>,
    payload: MatrixIgnoredUsersUserRequest,
) -> Result<MatrixIgnoredUsersWriteResult, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-ignored-users-ignore-no-session")
    })?;
    let result: MatrixIgnoredUsersWriteResult = owner
        .ignore_user(&payload.user_id)
        .await
        .map_err(ignored_users_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_ignored_users_ignore(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixIgnoredUsersUserRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-ignored-users-ignore-invalid-payload"))?;
        let response = ignored_users_ignore(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-ignored-users-ignore-serialization-failed"))
    })
}

/// Typed `matrix_ignored_users_unignore`.
pub(super) async fn ignored_users_unignore(
    state: &Arc<CoreState>,
    payload: MatrixIgnoredUsersUserRequest,
) -> Result<MatrixIgnoredUsersWriteResult, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-ignored-users-unignore-no-session")
    })?;
    let result: MatrixIgnoredUsersWriteResult = owner
        .unignore_user(&payload.user_id)
        .await
        .map_err(ignored_users_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_ignored_users_unignore(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixIgnoredUsersUserRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-ignored-users-unignore-invalid-payload"))?;
        let response = ignored_users_unignore(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-ignored-users-unignore-serialization-failed"))
    })
}

pub(super) fn user_directory_search_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-search.directory-empty-term"
        | "v-search.directory-term-too-long"
        | "v-search.directory-invalid-term"
        | "v-search.directory-invalid-limit" => MatrixIpcErrorCategory::SdkInvariant,
        "v-search.directory-no-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

/// Typed `matrix_user_directory_search`.
pub(super) async fn user_directory_search(
    state: &Arc<CoreState>,
    payload: MatrixUserDirectorySearchRequest,
) -> Result<MatrixUserDirectorySearchResult, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-user-directory-search-no-session")
    })?;
    let result: MatrixUserDirectorySearchResult = owner
        .search_user_directory(&payload.term, payload.limit)
        .await
        .map_err(user_directory_search_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_user_directory_search(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixUserDirectorySearchRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-user-directory-search-invalid-payload"))?;
        let response = user_directory_search(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-user-directory-search-serialization-failed"))
    })
}

pub(super) fn message_search_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-search.term-too-long"
        | "v-search.invalid-term"
        | "v-search.invalid-token"
        | "v-search.invalid-order"
        | "v-search.invalid-room"
        | "v-search.invalid-sender"
        | "v-search.invalid-listing"
        | "v-search.invalid-range" => MatrixIpcErrorCategory::SdkInvariant,
        "v-search.no-session" => MatrixIpcErrorCategory::Forbidden,
        // Off disables product search; no Client-Server `/search` fallback.
        "v-search.index-disabled" => MatrixIpcErrorCategory::Unknown,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

/// Typed `matrix_message_search`.
pub(super) async fn message_search(
    state: &Arc<CoreState>,
    payload: MatrixMessageSearchRequest,
) -> Result<MatrixMessageSearchResult, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-message-search-no-session")
    })?;
    let listing_kind = payload
        .listing_kind
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let result: MatrixMessageSearchResult = if payload.term.trim().is_empty() {
        if let Some(kind) = listing_kind {
            let rooms = payload.rooms.as_deref().unwrap_or(&[]);
            if rooms.len() != 1 {
                return Err(message_search_owner_error("v-search.invalid-room"));
            }
            let room_id = rooms[0].clone();
            let from_ts = payload
                .from_ts
                .ok_or_else(|| message_search_owner_error("v-search.invalid-range"))?;
            let to_ts = payload
                .to_ts
                .ok_or_else(|| message_search_owner_error("v-search.invalid-range"))?;
            owner
                .list_room_attachments(&room_id, kind, from_ts, to_ts)
                .await
                .map_err(message_search_owner_error)?
        } else {
            owner
                .search_messages(
                    &payload.term,
                    payload.next_token.as_deref(),
                    payload.rooms.as_deref(),
                    payload.senders.as_deref(),
                    payload.order.as_deref(),
                )
                .await
                .map_err(message_search_owner_error)?
        }
    } else {
        owner
            .search_messages(
                &payload.term,
                payload.next_token.as_deref(),
                payload.rooms.as_deref(),
                payload.senders.as_deref(),
                payload.order.as_deref(),
            )
            .await
            .map_err(message_search_owner_error)?
    };
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_message_search(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixMessageSearchRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-message-search-invalid-payload"))?;
        let response = message_search(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-message-search-serialization-failed"))
    })
}

/// Typed `matrix_threepid_snapshot`.
pub(super) async fn threepid_snapshot(
    state: &Arc<CoreState>,
) -> Result<MatrixThreepidSnapshot, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-threepid-snapshot-no-session")
    })?;
    let result: MatrixThreepidSnapshot = owner
        .snapshot_threepids()
        .await
        .map_err(threepid_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_threepid_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !own_profile_read_payload_is_empty(&request.payload) {
            return Err(core_state_error("p2-threepid-snapshot-invalid-payload"));
        }
        let response = threepid_snapshot(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-threepid-snapshot-serialization-failed"))
    })
}

/// Typed `matrix_threepid_delete`.
pub(super) async fn threepid_delete(
    state: &Arc<CoreState>,
    payload: MatrixThreepidAddressRequest,
) -> Result<MatrixThreepidWriteResult, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-threepid-delete-no-session")
    })?;
    let result: MatrixThreepidWriteResult = owner
        .delete_threepid_email(&payload.address)
        .await
        .map_err(threepid_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_threepid_delete(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixThreepidAddressRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-threepid-delete-invalid-payload"))?;
        let response = threepid_delete(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-threepid-delete-serialization-failed"))
    })
}

/// Typed `matrix_threepid_request_email_token`.
pub(super) async fn threepid_request_email_token(
    state: &Arc<CoreState>,
    payload: MatrixThreepidEmailRequest,
) -> Result<MatrixThreepidEmailTokenResult, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-threepid-request-email-token-no-session")
    })?;
    let result: MatrixThreepidEmailTokenResult = owner
        .request_threepid_email_token(&payload.email)
        .await
        .map_err(threepid_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_threepid_request_email_token(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixThreepidEmailRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-threepid-request-email-token-invalid-payload"))?;
        let response = threepid_request_email_token(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-threepid-request-email-token-serialization-failed"))
    })
}

/// Typed `matrix_threepid_add_email`.
pub(super) async fn threepid_add_email(
    state: &Arc<CoreState>,
) -> Result<MatrixThreepidAddResult, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-threepid-add-email-no-session")
    })?;
    let result: MatrixThreepidAddResult = owner
        .add_threepid_email()
        .await
        .map_err(threepid_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_threepid_add_email(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !own_profile_read_payload_is_empty(&request.payload) {
            return Err(core_state_error("p2-threepid-add-email-invalid-payload"));
        }
        let response = threepid_add_email(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-threepid-add-email-serialization-failed"))
    })
}

/// `matrix_media_config` has no renderer payload. Core owns the envelope and
/// exact legacy object serialization only; the Platform remains the sole owner
/// of the Matrix SDK client/session/cache/store and its cache/network load.
/// Typed `matrix_media_config`.
pub(super) async fn media_config(
    state: &Arc<CoreState>,
) -> Result<MatrixMediaConfigResponse, MatrixIpcError> {
    let platform = state.platform();
    let config = platform
        .media_config()
        .await
        .map_err(media_config_transport_error)?;
    let response = MatrixMediaConfigResponse::from_platform(config)?;
    Ok(response)
}

#[cfg(test)]
pub(super) fn matrix_media_config(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-media-config-invalid-payload"));
        }
        let response = media_config(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-media-config-serialization-failed"))
    })
}

/// Map the closed Platform media observation to static Core transport errors.
/// No Platform string, SDK error, URL, credential, key, or Core error object
/// enters this mapping. The desktop bridge uses only the resulting category to
/// restore its established static command diagnostics.
pub(super) fn media_config_transport_error(error: PlatformMediaConfigError) -> MatrixIpcError {
    match error {
        PlatformMediaConfigError::NoSession => {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-media-config-no-session")
        }
        PlatformMediaConfigError::LoadFailed => {
            MatrixIpcError::new(MatrixIpcErrorCategory::Unknown)
                .with_diagnostic("p2-media-config-load-failed")
        }
        // The source value was outside the shared JSON-safe range. Keep this
        // distinct in Core so the bridge can retain the legacy unsafe-size
        // diagnostic instead of conflating it with an SDK load failure.
        PlatformMediaConfigError::UnsafeSize => {
            MatrixIpcError::new(MatrixIpcErrorCategory::MediaTooLarge)
                .with_diagnostic("p2-media-config-unsafe-size")
        }
    }
}

/// Registers this domain's JSON adapters with the test-only command registry.
#[cfg(test)]
pub(super) fn register_commands(registry: &mut CommandRegistry) {
    registry
        .register("matrix_media_config", matrix_media_config)
        .expect("built-in matrix_media_config must remain in the command census");
    registry
        .register("matrix_media_preview", matrix_media_preview)
        .expect("built-in matrix_media_preview must remain in the command census");
    registry
        .register("matrix_set_own_display_name", matrix_set_own_display_name)
        .expect("built-in matrix_set_own_display_name must remain in the command census");
    registry
        .register("matrix_set_own_avatar", matrix_set_own_avatar)
        .expect("built-in matrix_set_own_avatar must remain in the command census");
    registry
        .register("matrix_get_own_profile", matrix_get_own_profile)
        .expect("built-in matrix_get_own_profile must remain in the command census");
    registry
        .register(
            "matrix_ignored_users_snapshot",
            matrix_ignored_users_snapshot,
        )
        .expect("built-in matrix_ignored_users_snapshot must remain in the command census");
    registry
        .register("matrix_ignored_users_ignore", matrix_ignored_users_ignore)
        .expect("built-in matrix_ignored_users_ignore must remain in the command census");
    registry
        .register(
            "matrix_ignored_users_unignore",
            matrix_ignored_users_unignore,
        )
        .expect("built-in matrix_ignored_users_unignore must remain in the command census");
    registry
        .register("matrix_user_directory_search", matrix_user_directory_search)
        .expect("built-in matrix_user_directory_search must remain in the command census");
    registry
        .register("matrix_message_search", matrix_message_search)
        .expect("built-in matrix_message_search must remain in the command census");
    registry
        .register("matrix_threepid_snapshot", matrix_threepid_snapshot)
        .expect("built-in matrix_threepid_snapshot must remain in the command census");
    registry
        .register("matrix_threepid_delete", matrix_threepid_delete)
        .expect("built-in matrix_threepid_delete must remain in the command census");
    registry
        .register(
            "matrix_threepid_request_email_token",
            matrix_threepid_request_email_token,
        )
        .expect("built-in matrix_threepid_request_email_token must remain in the command census");
    registry
        .register("matrix_threepid_add_email", matrix_threepid_add_email)
        .expect("built-in matrix_threepid_add_email must remain in the command census");
}
