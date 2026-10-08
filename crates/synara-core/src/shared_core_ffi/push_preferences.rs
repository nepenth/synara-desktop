//! Typed SharedCore operations and projections for push preferences.

use super::*;
use crate::app::notifications::MatrixPushRulesSnapshot;
use crate::app::notifications::MatrixPushRulesWriteResult;
use crate::app::notifications::MatrixRoomNotificationSnapshot;
use crate::app::notifications::MatrixRoomNotificationsSnapshot;

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PushRuleMentionsDto {
    pub user_mention: bool,
    pub display_name: bool,
    pub user_name: bool,
    pub room_mention: bool,
    pub at_room: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PushRulesSnapshotDto {
    pub dm: String,
    pub dm_encrypted: String,
    pub group: String,
    pub group_encrypted: String,
    pub mentions: PushRuleMentionsDto,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PushRulesWriteDto {
    pub status: WriteAckDto,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum PushRulesCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for PushRulesCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for PushRulesCommandError {}

pub(super) fn push_rules_failed(code: &str, description: &'static str) -> PushRulesCommandError {
    PushRulesCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_push_rules_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> PushRulesCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            push_rules_failed(code, PUSH_RULES_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-push.") => {
            push_rules_failed(code, PUSH_RULES_OWNER_DESCRIPTION)
        }
        _ => push_rules_failed(PUSH_RULES_FAILED_CODE, PUSH_RULES_FAILED_DESCRIPTION),
    }
}

pub(super) fn push_rules_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, PushRulesCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(push_rules_failed(
            PUSH_RULES_FAILED_CODE,
            PUSH_RULES_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

pub(super) fn push_rules_snapshot_dto(
    payload: MatrixPushRulesSnapshot,
) -> Result<PushRulesSnapshotDto, PushRulesCommandError> {
    let snapshot: crate::app::notifications::MatrixPushRulesSnapshot = payload;
    Ok(PushRulesSnapshotDto {
        dm: snapshot.dm,
        dm_encrypted: snapshot.dm_encrypted,
        group: snapshot.group,
        group_encrypted: snapshot.group_encrypted,
        mentions: PushRuleMentionsDto {
            user_mention: snapshot.mentions.user_mention,
            display_name: snapshot.mentions.display_name,
            user_name: snapshot.mentions.user_name,
            room_mention: snapshot.mentions.room_mention,
            at_room: snapshot.mentions.at_room,
        },
        keywords: snapshot.keywords,
    })
}

pub(super) fn push_rules_write_dto(
    payload: MatrixPushRulesWriteResult,
) -> Result<PushRulesWriteDto, PushRulesCommandError> {
    let status = Some(payload.status)
        .ok_or_else(|| push_rules_failed(PUSH_RULES_FAILED_CODE, PUSH_RULES_FAILED_DESCRIPTION))?;
    Ok(PushRulesWriteDto {
        status: status.into(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomNotificationSnapshotDto {
    pub room_id: String,
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomNotificationsSnapshotDto {
    pub rooms: Vec<RoomNotificationSnapshotDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomNotificationWriteDto {
    pub status: WriteAckDto,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum RoomNotificationCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomNotificationCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomNotificationCommandError {}

pub(super) fn room_notification_failed(
    code: &str,
    description: &'static str,
) -> RoomNotificationCommandError {
    RoomNotificationCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_room_notification_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> RoomNotificationCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            room_notification_failed(code, ROOM_NOTIFICATION_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-push.") => {
            room_notification_failed(code, ROOM_NOTIFICATION_OWNER_DESCRIPTION)
        }
        _ => room_notification_failed(
            ROOM_NOTIFICATION_FAILED_CODE,
            ROOM_NOTIFICATION_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn room_notification_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, RoomNotificationCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(room_notification_failed(
            ROOM_NOTIFICATION_FAILED_CODE,
            ROOM_NOTIFICATION_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

pub(super) fn room_notification_snapshot_dto(
    payload: MatrixRoomNotificationSnapshot,
) -> Result<RoomNotificationSnapshotDto, RoomNotificationCommandError> {
    let snapshot: crate::app::notifications::MatrixRoomNotificationSnapshot = payload;
    Ok(RoomNotificationSnapshotDto {
        room_id: snapshot.room_id,
        mode: snapshot.mode,
    })
}

pub(super) fn room_notifications_snapshot_dto(
    payload: MatrixRoomNotificationsSnapshot,
) -> Result<RoomNotificationsSnapshotDto, RoomNotificationCommandError> {
    let snapshot: crate::app::notifications::MatrixRoomNotificationsSnapshot = payload;
    Ok(RoomNotificationsSnapshotDto {
        rooms: snapshot
            .rooms
            .into_iter()
            .map(|room| RoomNotificationSnapshotDto {
                room_id: room.room_id,
                mode: room.mode,
            })
            .collect(),
    })
}

pub(super) fn room_notification_write_dto(
    payload: MatrixPushRulesWriteResult,
) -> Result<RoomNotificationWriteDto, RoomNotificationCommandError> {
    let status = Some(payload.status).ok_or_else(|| {
        room_notification_failed(
            ROOM_NOTIFICATION_FAILED_CODE,
            ROOM_NOTIFICATION_FAILED_DESCRIPTION,
        )
    })?;
    Ok(RoomNotificationWriteDto {
        status: status.into(),
    })
}

/// Privacy-safe HTTP pusher write ack. Status only; never push key or URL.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PusherWriteDto {
    pub status: WriteAckDto,
}

/// Account-bound HTTP pusher capability. It retains the exact Core owner—and
/// therefore the exact authenticated Matrix client—captured at bind time.
/// No account identity, token, push key, or gateway is projected back out.
#[derive(uniffi::Object)]
pub struct HttpPusherOwner {
    pub(super) owner: Arc<NativeHttpPusherOwner>,
}

#[uniffi::export(async_runtime = "tokio")]
impl HttpPusherOwner {
    pub async fn register_http_pusher(
        &self,
        push_key: String,
        app_id: String,
        gateway_url: String,
        app_display_name: String,
        lang: String,
    ) -> Result<PusherWriteDto, PusherCommandError> {
        http_pusher_reject_oversize(push_key.len())?;
        http_pusher_reject_oversize(app_id.len())?;
        http_pusher_reject_oversize(gateway_url.len())?;
        http_pusher_reject_oversize(app_display_name.len())?;
        http_pusher_reject_oversize(lang.len())?;
        // UniFFI 0.28 does not propagate Swift Task cancellation into this
        // future. Logout awaits reconciliation, so bound the entire write.
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.owner
                .register(&push_key, &app_id, &gateway_url, &app_display_name, &lang),
        )
        .await
        .unwrap_or(Err("pusher-registration-timeout"))
        .map_err(|error| {
            map_http_pusher_core_error(
                REGISTER_HTTP_PUSHER_NO_SESSION_CODE,
                MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant).with_diagnostic(error),
            )
        })?;
        Ok(PusherWriteDto {
            status: result.status.into(),
        })
    }

    pub async fn delete_http_pusher(
        &self,
        push_key: String,
        app_id: String,
    ) -> Result<PusherWriteDto, PusherCommandError> {
        http_pusher_reject_oversize(push_key.len())?;
        http_pusher_reject_oversize(app_id.len())?;
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.owner.delete(&push_key, &app_id),
        )
        .await
        .unwrap_or(Err("pusher-deletion-timeout"))
        .map_err(|error| {
            map_http_pusher_core_error(
                DELETE_HTTP_PUSHER_NO_SESSION_CODE,
                MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant).with_diagnostic(error),
            )
        })?;
        Ok(PusherWriteDto {
            status: result.status.into(),
        })
    }

    pub async fn delete_http_pushers_for_device(
        &self,
        app_id: String,
        last_push_key: Option<String>,
    ) -> Result<PusherWriteDto, PusherCommandError> {
        http_pusher_reject_oversize(app_id.len())?;
        if let Some(push_key) = last_push_key.as_ref() {
            http_pusher_reject_oversize(push_key.len())?;
        }
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.owner
                .delete_for_device(&app_id, last_push_key.as_deref()),
        )
        .await
        .unwrap_or(Err("pusher-cleanup-timeout"))
        .map_err(|error| {
            map_http_pusher_core_error(
                DELETE_HTTP_PUSHER_NO_SESSION_CODE,
                MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant).with_diagnostic(error),
            )
        })?;
        Ok(PusherWriteDto {
            status: result.status.into(),
        })
    }
}

/// Static fail-closed HTTP pusher-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum PusherCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for PusherCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for PusherCommandError {}

pub(super) fn http_pusher_failed(code: &str, description: &'static str) -> PusherCommandError {
    PusherCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn http_pusher_reject_oversize(size: usize) -> Result<(), PusherCommandError> {
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(http_pusher_failed(
            HTTP_PUSHER_FAILED_CODE,
            HTTP_PUSHER_FAILED_DESCRIPTION,
        ));
    }
    Ok(())
}

pub(super) fn map_http_pusher_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> PusherCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            http_pusher_failed(code, HTTP_PUSHER_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-pusher.") || code.starts_with("v-push.") => {
            http_pusher_failed(code, HTTP_PUSHER_OWNER_DESCRIPTION)
        }
        _ => http_pusher_failed(HTTP_PUSHER_FAILED_CODE, HTTP_PUSHER_FAILED_DESCRIPTION),
    }
}

impl SharedCore {
    /// Test-only attach of the production HTTP-pusher owner from the retained
    /// Matrix client, without starting unrelated account/device owners.
    /// Not exported through UniFFI.
    #[doc(hidden)]
    pub fn attach_http_pusher_owner_for_test(&self) -> Result<(), PusherCommandError> {
        let client = {
            let guard = self.restored_client.lock().map_err(|_| {
                http_pusher_failed(
                    BIND_HTTP_PUSHER_NO_SESSION_CODE,
                    HTTP_PUSHER_OWNER_DESCRIPTION,
                )
            })?;
            match &*guard {
                RestoredClientSlot::Ready(client, _) => client.clone(),
                RestoredClientSlot::Empty | RestoredClientSlot::InFlight => {
                    return Err(http_pusher_failed(
                        BIND_HTTP_PUSHER_NO_SESSION_CODE,
                        HTTP_PUSHER_OWNER_DESCRIPTION,
                    ));
                }
            }
        };
        let owner = Arc::new(NativeHttpPusherOwner::new(&client).map_err(|_| {
            http_pusher_failed(
                BIND_HTTP_PUSHER_NO_SESSION_CODE,
                HTTP_PUSHER_OWNER_DESCRIPTION,
            )
        })?);
        self.core.attach_http_pusher(owner).map_err(|_| {
            http_pusher_failed(
                BIND_HTTP_PUSHER_NO_SESSION_CODE,
                HTTP_PUSHER_OWNER_DESCRIPTION,
            )
        })
    }

    pub(super) async fn push_rules_command<T>(
        &self,
        no_session: &'static str,
        request: impl std::future::Future<Output = Result<T, MatrixIpcError>>,
    ) -> Result<T, PushRulesCommandError> {
        let response = request
            .await
            .map_err(|error| map_push_rules_core_error(no_session, error))?;
        Ok(response)
    }

    pub(super) async fn room_notification_command<T>(
        &self,
        no_session: &'static str,
        request: impl std::future::Future<Output = Result<T, MatrixIpcError>>,
    ) -> Result<T, RoomNotificationCommandError> {
        let response = request
            .await
            .map_err(|error| map_room_notification_core_error(no_session, error))?;
        Ok(response)
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SharedCore {
    pub async fn register_http_pusher(
        &self,
        push_key: String,
        app_id: String,
        gateway_url: String,
        app_display_name: String,
        lang: String,
    ) -> Result<PusherWriteDto, PusherCommandError> {
        http_pusher_reject_oversize(push_key.len())?;
        http_pusher_reject_oversize(app_id.len())?;
        http_pusher_reject_oversize(gateway_url.len())?;
        http_pusher_reject_oversize(app_display_name.len())?;
        http_pusher_reject_oversize(lang.len())?;
        let result = self
            .core
            .register_http_pusher(&push_key, &app_id, &gateway_url, &app_display_name, &lang)
            .await
            .map_err(|error| {
                map_http_pusher_core_error(REGISTER_HTTP_PUSHER_NO_SESSION_CODE, error)
            })?;
        Ok(PusherWriteDto {
            status: result.status.into(),
        })
    }

    /// Capture a pusher owner bound to the exact retained Matrix client.
    /// Identity inputs are used only for fail-closed owner selection and are
    /// never returned or included in errors.
    pub fn bind_http_pusher_owner(
        &self,
        user_id: String,
        device_id: String,
        homeserver_url: String,
    ) -> Result<Arc<HttpPusherOwner>, PusherCommandError> {
        http_pusher_reject_oversize(user_id.len())?;
        http_pusher_reject_oversize(device_id.len())?;
        http_pusher_reject_oversize(homeserver_url.len())?;
        let owner = self
            .core
            .http_pusher_owner()
            .map_err(|error| map_http_pusher_core_error(BIND_HTTP_PUSHER_NO_SESSION_CODE, error))?;
        if !owner.owns_session(&user_id, &device_id, &homeserver_url) {
            return Err(http_pusher_failed(
                HTTP_PUSHER_SESSION_MISMATCH_CODE,
                HTTP_PUSHER_OWNER_DESCRIPTION,
            ));
        }
        Ok(Arc::new(HttpPusherOwner { owner }))
    }

    pub async fn delete_http_pusher(
        &self,
        push_key: String,
        app_id: String,
    ) -> Result<PusherWriteDto, PusherCommandError> {
        http_pusher_reject_oversize(push_key.len())?;
        http_pusher_reject_oversize(app_id.len())?;
        let result = self
            .core
            .delete_http_pusher(&push_key, &app_id)
            .await
            .map_err(|error| {
                map_http_pusher_core_error(DELETE_HTTP_PUSHER_NO_SESSION_CODE, error)
            })?;
        Ok(PusherWriteDto {
            status: result.status.into(),
        })
    }

    pub async fn push_rules_snapshot(&self) -> Result<PushRulesSnapshotDto, PushRulesCommandError> {
        self.push_rules_command(
            PUSH_RULES_SNAPSHOT_NO_SESSION_CODE,
            self.core.push_rules_snapshot(),
        )
        .await
        .and_then(push_rules_snapshot_dto)
    }

    pub async fn push_rules_set_default(
        &self,
        encrypted: bool,
        one_to_one: bool,
        mode: String,
    ) -> Result<PushRulesWriteDto, PushRulesCommandError> {
        push_rules_envelope_payload(serde_json::json!({
            "encrypted": encrypted,
            "oneToOne": one_to_one,
            "mode": mode,
        }))?;
        self.push_rules_command(
            PUSH_RULES_SET_DEFAULT_NO_SESSION_CODE,
            self.core
                .push_rules_set_default(crate::core_api::MatrixPushRulesSetDefaultRequest {
                    encrypted,
                    one_to_one,
                    mode,
                }),
        )
        .await
        .and_then(push_rules_write_dto)
    }

    pub async fn push_rules_set_mention(
        &self,
        rule_id: String,
        enabled: bool,
    ) -> Result<PushRulesWriteDto, PushRulesCommandError> {
        push_rules_envelope_payload(serde_json::json!({
            "ruleId": rule_id,
            "enabled": enabled,
        }))?;
        self.push_rules_command(
            PUSH_RULES_SET_MENTION_NO_SESSION_CODE,
            self.core
                .push_rules_set_mention(crate::core_api::MatrixPushRulesSetMentionRequest {
                    rule_id,
                    enabled,
                }),
        )
        .await
        .and_then(push_rules_write_dto)
    }

    pub async fn push_rules_add_keyword(
        &self,
        keyword: String,
    ) -> Result<PushRulesWriteDto, PushRulesCommandError> {
        push_rules_envelope_payload(serde_json::json!({ "keyword": keyword }))?;
        self.push_rules_command(
            PUSH_RULES_ADD_KEYWORD_NO_SESSION_CODE,
            self.core
                .push_rules_add_keyword(crate::core_api::MatrixPushRulesKeywordRequest { keyword }),
        )
        .await
        .and_then(push_rules_write_dto)
    }

    pub async fn push_rules_remove_keyword(
        &self,
        keyword: String,
    ) -> Result<PushRulesWriteDto, PushRulesCommandError> {
        push_rules_envelope_payload(serde_json::json!({ "keyword": keyword }))?;
        self.push_rules_command(
            PUSH_RULES_REMOVE_KEYWORD_NO_SESSION_CODE,
            self.core
                .push_rules_remove_keyword(crate::core_api::MatrixPushRulesKeywordRequest {
                    keyword,
                }),
        )
        .await
        .and_then(push_rules_write_dto)
    }

    pub async fn room_notification_snapshot(
        &self,
        room_id: String,
    ) -> Result<RoomNotificationSnapshotDto, RoomNotificationCommandError> {
        room_notification_envelope_payload(serde_json::json!({ "roomId": room_id }))?;
        self.room_notification_command(
            ROOM_NOTIFICATION_SNAPSHOT_NO_SESSION_CODE,
            self.core.room_notification_snapshot(
                crate::core_api::MatrixRoomNotificationRoomRequest { room_id },
            ),
        )
        .await
        .and_then(room_notification_snapshot_dto)
    }

    pub async fn room_notification_set(
        &self,
        room_id: String,
        mode: String,
    ) -> Result<RoomNotificationWriteDto, RoomNotificationCommandError> {
        room_notification_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "mode": mode,
        }))?;
        self.room_notification_command(
            ROOM_NOTIFICATION_SET_NO_SESSION_CODE,
            self.core
                .room_notification_set(crate::core_api::MatrixRoomNotificationSetRequest {
                    room_id,
                    mode,
                }),
        )
        .await
        .and_then(room_notification_write_dto)
    }

    pub async fn room_notifications_snapshot(
        &self,
    ) -> Result<RoomNotificationsSnapshotDto, RoomNotificationCommandError> {
        self.room_notification_command(
            ROOM_NOTIFICATIONS_SNAPSHOT_NO_SESSION_CODE,
            self.core.room_notifications_snapshot(),
        )
        .await
        .and_then(room_notifications_snapshot_dto)
    }

    pub async fn set_notification_mode(
        &self,
        room_id: String,
        mode: String,
    ) -> Result<LeftoverAckDto, LeftoverCommandError> {
        leftover_reject_oversize(room_id.len() + mode.len())?;
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

    pub async fn pusher_set(
        &self,
        push_key: String,
        app_id: String,
        gateway_url: String,
        app_display_name: String,
        device_display_name: String,
        lang: String,
    ) -> Result<LeftoverAckDto, LeftoverCommandError> {
        leftover_reject_oversize(
            push_key.len()
                + app_id.len()
                + gateway_url.len()
                + app_display_name.len()
                + device_display_name.len()
                + lang.len(),
        )?;
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

    pub async fn pusher_delete(
        &self,
        push_key: String,
        app_id: String,
    ) -> Result<LeftoverAckDto, LeftoverCommandError> {
        leftover_reject_oversize(push_key.len() + app_id.len())?;
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
