//! Typed SharedCore operations and projections for realtime.

use super::*;

/// Privacy-safe owner emit summary. No user id, tokens, or password.
/// iOS re-fetches via the existing snapshot commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerUpdateDto {
    pub family: String,
    pub session_generation: u64,
    pub room_id: Option<String>,
}

/// Static fail-closed owner-update poll error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnerUpdateError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for OwnerUpdateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for OwnerUpdateError {}

pub(super) fn owner_update_poll_failed(
    code: &'static str,
    description: &'static str,
) -> OwnerUpdateError {
    OwnerUpdateError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn push_owner_update(
    queue: &Mutex<Vec<OwnerUpdateDto>>,
    family: impl Into<String>,
    session_generation: u64,
    room_id: Option<String>,
) {
    if let Ok(mut guard) = queue.lock() {
        if guard.len() >= OWNER_UPDATE_QUEUE_CAP {
            guard.remove(0);
        }
        guard.push(OwnerUpdateDto {
            family: family.into(),
            session_generation,
            room_id,
        });
    }
}

/// Privacy-safe typing room row. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypingRoomDto {
    pub room_id: String,
    pub user_ids: Vec<String>,
}

/// Privacy-safe typing snapshot. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypingSnapshotDto {
    pub session_generation: u64,
    pub rooms: Vec<TypingRoomDto>,
}

/// Static fail-closed typing error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypingCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for TypingCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for TypingCommandError {}

pub(super) fn typing_failed(code: &'static str, description: &'static str) -> TypingCommandError {
    TypingCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_typing_snapshot_core_error(error: MatrixIpcError) -> TypingCommandError {
    match error.diagnostic_id.as_deref() {
        Some("p2-typing-snapshot-no-session") => typing_failed(
            TYPING_SNAPSHOT_NO_SESSION_CODE,
            TYPING_NO_SESSION_DESCRIPTION,
        ),
        _ => typing_failed(
            TYPING_SNAPSHOT_FAILED_CODE,
            TYPING_SNAPSHOT_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn map_typing_set_core_error(error: MatrixIpcError) -> TypingCommandError {
    match error.diagnostic_id.as_deref() {
        Some("p2-typing-set-no-session") => {
            typing_failed(TYPING_SET_NO_SESSION_CODE, TYPING_NO_SESSION_DESCRIPTION)
        }
        Some("v-rooms.4-typing-invalid-room") => {
            typing_failed(TYPING_INVALID_ROOM_CODE, TYPING_INVALID_ROOM_DESCRIPTION)
        }
        Some("v-rooms.4-typing-room-missing") => {
            typing_failed(TYPING_ROOM_MISSING_CODE, TYPING_ROOM_MISSING_DESCRIPTION)
        }
        Some("v-rooms.4-typing-room-not-joined") => typing_failed(
            "v-rooms.4-typing-room-not-joined",
            TYPING_ROOM_MISSING_DESCRIPTION,
        ),
        _ => typing_failed(TYPING_SET_FAILED_CODE, TYPING_SET_FAILED_DESCRIPTION),
    }
}

/// Privacy-safe presence snapshot. Identity fields only; no tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceSnapshotDto {
    pub status: String,
    pub session_generation: u64,
    pub user_id: String,
    pub state: Option<String>,
    pub currently_active: bool,
    pub last_active_ts: Option<u64>,
    pub status_msg: Option<String>,
}

/// Privacy-safe presence subscription. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceSubscriptionDto {
    pub subscription_id: String,
    pub user_id: String,
    pub session_generation: u64,
}

/// Privacy-safe presence SET ack. Status only; never echo state or statusMsg.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceWriteDto {
    pub status: String,
}

/// Privacy-safe MatrixRTC transport. URLs only; no JWTs or tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtcTransportDto {
    pub kind: String,
    pub service_url: Option<String>,
}

/// Privacy-safe MatrixRTC discovery snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtcTransportsSnapshotDto {
    pub session_generation: u64,
    pub status: String,
    pub transports: Vec<RtcTransportDto>,
}

/// Static fail-closed RTC transport error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RtcTransportsCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RtcTransportsCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RtcTransportsCommandError {}

pub(super) fn rtc_transports_failed(
    code: &'static str,
    description: &'static str,
) -> RtcTransportsCommandError {
    RtcTransportsCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_rtc_transports_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> RtcTransportsCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            rtc_transports_failed(no_session, RTC_TRANSPORTS_NO_SESSION_DESCRIPTION)
        }
        _ => rtc_transports_failed(
            RTC_TRANSPORTS_FAILED_CODE,
            RTC_TRANSPORTS_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn rtc_transports_snapshot_dto(
    snapshot: NativeRtcTransportsSnapshot,
) -> RtcTransportsSnapshotDto {
    RtcTransportsSnapshotDto {
        session_generation: snapshot.session_generation,
        status: match snapshot.status {
            crate::app::rtc_transports::NativeRtcTransportsStatus::Ready => "ready".to_owned(),
            crate::app::rtc_transports::NativeRtcTransportsStatus::Unsupported => {
                "unsupported".to_owned()
            }
            crate::app::rtc_transports::NativeRtcTransportsStatus::Unavailable => {
                "unavailable".to_owned()
            }
        },
        transports: snapshot
            .transports
            .into_iter()
            .map(|row: NativeRtcTransport| RtcTransportDto {
                kind: row.kind.as_str().to_owned(),
                service_url: row.service_url,
            })
            .collect(),
    }
}

/// Privacy-safe MSC4426 status field. No tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserStatusFieldDto {
    pub emoji: String,
    pub text: String,
}

/// Privacy-safe MSC4426 snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserStatusSnapshotDto {
    pub session_generation: u64,
    pub user_id: String,
    pub user_status: Option<UserStatusFieldDto>,
    pub in_call: Option<UserInCallDto>,
}

/// Privacy-safe MSC4426 write ack. Status only; never echo emoji or text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserStatusWriteDto {
    pub status: String,
}

/// Static fail-closed user-status error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserStatusCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for UserStatusCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for UserStatusCommandError {}

pub(super) fn user_status_failed(
    code: &'static str,
    description: &'static str,
) -> UserStatusCommandError {
    UserStatusCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_user_status_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> UserStatusCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            user_status_failed(no_session, USER_STATUS_NO_SESSION_DESCRIPTION)
        }
        Some("v-user-status-unsupported") => {
            user_status_failed("v-user-status-unsupported", USER_STATUS_FAILED_DESCRIPTION)
        }
        Some("v-user-status-emoji-cap") => {
            user_status_failed("v-user-status-emoji-cap", USER_STATUS_FAILED_DESCRIPTION)
        }
        Some("v-user-status-text-cap") => {
            user_status_failed("v-user-status-text-cap", USER_STATUS_FAILED_DESCRIPTION)
        }
        Some("v-user-status-invalid-user-id") => user_status_failed(
            "v-user-status-invalid-user-id",
            USER_STATUS_FAILED_DESCRIPTION,
        ),
        _ => user_status_failed(USER_STATUS_FAILED_CODE, USER_STATUS_FAILED_DESCRIPTION),
    }
}

pub(super) fn user_status_snapshot_dto(
    snapshot: NativeUserStatusSnapshot,
) -> UserStatusSnapshotDto {
    UserStatusSnapshotDto {
        session_generation: snapshot.session_generation,
        user_id: snapshot.user_id,
        user_status: snapshot
            .user_status
            .map(|status: NativeUserStatus| UserStatusFieldDto {
                emoji: status.emoji,
                text: status.text,
            }),
        in_call: snapshot.in_call.map(|call: NativeInCall| UserInCallDto {
            call_joined_ts: call.call_joined_ts,
        }),
    }
}

/// Static fail-closed presence error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresenceCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for PresenceCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for PresenceCommandError {}

pub(super) fn presence_failed(
    code: &'static str,
    description: &'static str,
) -> PresenceCommandError {
    PresenceCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_presence_snapshot_core_error(error: MatrixIpcError) -> PresenceCommandError {
    match error.diagnostic_id.as_deref() {
        Some("p2-presence-snapshot-no-session") => presence_failed(
            PRESENCE_SNAPSHOT_NO_SESSION_CODE,
            PRESENCE_NO_SESSION_DESCRIPTION,
        ),
        Some("v-presence-invalid-user-id") => presence_failed(
            PRESENCE_INVALID_USER_CODE,
            PRESENCE_INVALID_USER_DESCRIPTION,
        ),
        _ => presence_failed(
            PRESENCE_SNAPSHOT_FAILED_CODE,
            PRESENCE_SNAPSHOT_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn map_presence_subscribe_core_error(error: MatrixIpcError) -> PresenceCommandError {
    match error.diagnostic_id.as_deref() {
        Some("p2-presence-subscribe-no-session") => presence_failed(
            PRESENCE_SUBSCRIBE_NO_SESSION_CODE,
            PRESENCE_NO_SESSION_DESCRIPTION,
        ),
        Some("v-presence-invalid-user-id") => presence_failed(
            PRESENCE_INVALID_USER_CODE,
            PRESENCE_INVALID_USER_DESCRIPTION,
        ),
        _ => presence_failed(
            PRESENCE_SUBSCRIBE_FAILED_CODE,
            PRESENCE_SUBSCRIBE_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn map_presence_unsubscribe_core_error(error: MatrixIpcError) -> PresenceCommandError {
    match error.diagnostic_id.as_deref() {
        Some("p2-presence-unsubscribe-no-session") => presence_failed(
            PRESENCE_UNSUBSCRIBE_NO_SESSION_CODE,
            PRESENCE_NO_SESSION_DESCRIPTION,
        ),
        Some("v-presence-invalid-subscription-id") => presence_failed(
            PRESENCE_INVALID_SUBSCRIPTION_CODE,
            PRESENCE_INVALID_SUBSCRIPTION_DESCRIPTION,
        ),
        _ => presence_failed(
            PRESENCE_UNSUBSCRIBE_FAILED_CODE,
            PRESENCE_UNSUBSCRIBE_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn map_presence_set_core_error(error: MatrixIpcError) -> PresenceCommandError {
    match error.diagnostic_id.as_deref() {
        Some("p2-presence-set-no-session") => presence_failed(
            PRESENCE_SET_NO_SESSION_CODE,
            PRESENCE_NO_SESSION_DESCRIPTION,
        ),
        Some("v-presence-state-unsupported") => presence_failed(
            PRESENCE_INVALID_STATE_CODE,
            PRESENCE_INVALID_STATE_DESCRIPTION,
        ),
        Some("p4.7-status-msg-cap") => presence_failed(
            PRESENCE_STATUS_MSG_CAP_CODE,
            PRESENCE_STATUS_MSG_CAP_DESCRIPTION,
        ),
        _ => presence_failed(PRESENCE_SET_FAILED_CODE, PRESENCE_SET_FAILED_DESCRIPTION),
    }
}

pub(super) fn presence_state_as_str(state: NativePresenceState) -> String {
    match state {
        NativePresenceState::Unknown => "unknown",
        NativePresenceState::Offline => "offline",
        NativePresenceState::Online => "online",
        NativePresenceState::Unavailable => "unavailable",
    }
    .to_owned()
}

pub(super) fn presence_snapshot_dto(result: NativePresenceSnapshotResult) -> PresenceSnapshotDto {
    match result {
        NativePresenceSnapshotResult::Ready {
            session_generation,
            user_id,
            snapshot,
        } => PresenceSnapshotDto {
            status: "ready".to_owned(),
            session_generation,
            user_id,
            state: Some(presence_state_as_str(snapshot.state)),
            currently_active: snapshot.currently_active,
            last_active_ts: snapshot.last_active_ts,
            status_msg: snapshot.status_msg,
        },
        NativePresenceSnapshotResult::Unknown {
            session_generation,
            user_id,
        } => PresenceSnapshotDto {
            status: "unknown".to_owned(),
            session_generation,
            user_id,
            state: None,
            currently_active: false,
            last_active_ts: None,
            status_msg: None,
        },
    }
}

impl SharedCore {
    /// Drain queued owner emit summaries. Not `Core.command`.
    ///
    /// NSE forbids this. An empty queue returns an empty list. Presence
    /// user ids are never included. This is not Platform::emit.
    pub async fn poll_owner_updates(&self) -> Result<Vec<OwnerUpdateDto>, OwnerUpdateError> {
        if self.is_nse_read_only() {
            return Err(owner_update_poll_failed(
                NSE_FORBIDS_OWNER_POLL_CODE,
                NSE_FORBIDS_OWNER_POLL_DESCRIPTION,
            ));
        }
        let mut guard = self.owner_updates.lock().map_err(|_| {
            owner_update_poll_failed(
                OWNER_UPDATE_POLL_FAILED_CODE,
                OWNER_UPDATE_POLL_FAILED_DESCRIPTION,
            )
        })?;
        Ok(guard.drain(..).collect())
    }

    /// Test-only enqueue onto the attach owner emit queue. Not on UDL.
    #[doc(hidden)]
    pub fn enqueue_owner_update_for_test(
        &self,
        family: String,
        session_generation: u64,
        room_id: Option<String>,
    ) {
        push_owner_update(&self.owner_updates, family, session_generation, room_id);
    }

    pub async fn typing_snapshot(&self) -> Result<TypingSnapshotDto, TypingCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: TYPING_SNAPSHOT_COMMAND.to_owned(),
                session_generation: TYPING_PRESENCE_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .map_err(map_typing_snapshot_core_error)?;
        let snapshot: NativeTypingSnapshot =
            serde_json::from_value(response.payload).map_err(|_| {
                typing_failed(
                    TYPING_SNAPSHOT_FAILED_CODE,
                    TYPING_SNAPSHOT_FAILED_DESCRIPTION,
                )
            })?;
        Ok(TypingSnapshotDto {
            session_generation: snapshot.session_generation,
            rooms: snapshot
                .rooms
                .into_iter()
                .map(|room| TypingRoomDto {
                    room_id: room.room_id,
                    user_ids: room.user_ids,
                })
                .collect(),
        })
    }

    pub async fn typing_set(
        &self,
        room_id: String,
        typing: bool,
    ) -> Result<(), TypingCommandError> {
        self.core
            .command(CommandEnvelope {
                command: TYPING_SET_COMMAND.to_owned(),
                session_generation: TYPING_PRESENCE_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "roomId": room_id, "typing": typing }),
            })
            .await
            .map_err(map_typing_set_core_error)?;
        Ok(())
    }

    pub async fn presence_snapshot(
        &self,
        user_id: String,
    ) -> Result<PresenceSnapshotDto, PresenceCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: PRESENCE_SNAPSHOT_COMMAND.to_owned(),
                session_generation: TYPING_PRESENCE_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "userId": user_id }),
            })
            .await
            .map_err(map_presence_snapshot_core_error)?;
        let result: NativePresenceSnapshotResult = serde_json::from_value(response.payload)
            .map_err(|_| {
                presence_failed(
                    PRESENCE_SNAPSHOT_FAILED_CODE,
                    PRESENCE_SNAPSHOT_FAILED_DESCRIPTION,
                )
            })?;
        Ok(presence_snapshot_dto(result))
    }

    pub async fn presence_subscribe(
        &self,
        user_id: String,
    ) -> Result<PresenceSubscriptionDto, PresenceCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: PRESENCE_SUBSCRIBE_COMMAND.to_owned(),
                session_generation: TYPING_PRESENCE_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "userId": user_id }),
            })
            .await
            .map_err(map_presence_subscribe_core_error)?;
        let subscription: NativePresenceSubscription = serde_json::from_value(response.payload)
            .map_err(|_| {
                presence_failed(
                    PRESENCE_SUBSCRIBE_FAILED_CODE,
                    PRESENCE_SUBSCRIBE_FAILED_DESCRIPTION,
                )
            })?;
        Ok(PresenceSubscriptionDto {
            subscription_id: subscription.subscription_id,
            user_id: subscription.user_id,
            session_generation: subscription.session_generation,
        })
    }

    pub async fn presence_unsubscribe(
        &self,
        subscription_id: String,
    ) -> Result<(), PresenceCommandError> {
        self.core
            .command(CommandEnvelope {
                command: PRESENCE_UNSUBSCRIBE_COMMAND.to_owned(),
                session_generation: TYPING_PRESENCE_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "subscriptionId": subscription_id }),
            })
            .await
            .map_err(map_presence_unsubscribe_core_error)?;
        Ok(())
    }

    pub async fn presence_set(
        &self,
        state: String,
        status_msg: Option<String>,
    ) -> Result<PresenceWriteDto, PresenceCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: PRESENCE_SET_COMMAND.to_owned(),
                session_generation: TYPING_PRESENCE_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "state": state, "statusMsg": status_msg }),
            })
            .await
            .map_err(map_presence_set_core_error)?;
        let result: NativePresenceWriteResult =
            serde_json::from_value(response.payload).map_err(|_| {
                presence_failed(PRESENCE_SET_FAILED_CODE, PRESENCE_SET_FAILED_DESCRIPTION)
            })?;
        Ok(PresenceWriteDto {
            status: result.status,
        })
    }

    pub async fn rtc_transports_snapshot(
        &self,
    ) -> Result<RtcTransportsSnapshotDto, RtcTransportsCommandError> {
        self.rtc_transports_command(
            RTC_TRANSPORTS_SNAPSHOT_COMMAND,
            RTC_TRANSPORTS_NO_SESSION_CODE,
        )
        .await
    }

    pub async fn rtc_transports_refresh(
        &self,
    ) -> Result<RtcTransportsSnapshotDto, RtcTransportsCommandError> {
        self.rtc_transports_command(
            RTC_TRANSPORTS_REFRESH_COMMAND,
            RTC_TRANSPORTS_REFRESH_NO_SESSION_CODE,
        )
        .await
    }

    pub(super) async fn rtc_transports_command(
        &self,
        command: &'static str,
        no_session: &'static str,
    ) -> Result<RtcTransportsSnapshotDto, RtcTransportsCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: TYPING_PRESENCE_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .map_err(|error| map_rtc_transports_core_error(no_session, error))?;
        let snapshot: NativeRtcTransportsSnapshot = serde_json::from_value(response.payload)
            .map_err(|_| {
                rtc_transports_failed(
                    RTC_TRANSPORTS_FAILED_CODE,
                    RTC_TRANSPORTS_FAILED_DESCRIPTION,
                )
            })?;
        Ok(rtc_transports_snapshot_dto(snapshot))
    }

    pub async fn user_status_snapshot(
        &self,
        user_id: String,
    ) -> Result<UserStatusSnapshotDto, UserStatusCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: USER_STATUS_SNAPSHOT_COMMAND.to_owned(),
                session_generation: TYPING_PRESENCE_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "userId": user_id }),
            })
            .await
            .map_err(|error| {
                map_user_status_core_error(USER_STATUS_SNAPSHOT_NO_SESSION_CODE, error)
            })?;
        let snapshot: NativeUserStatusSnapshot =
            serde_json::from_value(response.payload).map_err(|_| {
                user_status_failed(USER_STATUS_FAILED_CODE, USER_STATUS_FAILED_DESCRIPTION)
            })?;
        Ok(user_status_snapshot_dto(snapshot))
    }

    pub async fn user_status_set(
        &self,
        emoji: String,
        text: String,
    ) -> Result<UserStatusWriteDto, UserStatusCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: USER_STATUS_SET_COMMAND.to_owned(),
                session_generation: TYPING_PRESENCE_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "emoji": emoji, "text": text }),
            })
            .await
            .map_err(|error| map_user_status_core_error(USER_STATUS_SET_NO_SESSION_CODE, error))?;
        let result: NativeUserStatusWriteResult = serde_json::from_value(response.payload)
            .map_err(|_| {
                user_status_failed(USER_STATUS_FAILED_CODE, USER_STATUS_FAILED_DESCRIPTION)
            })?;
        Ok(UserStatusWriteDto {
            status: result.status,
        })
    }

    pub async fn user_status_clear(&self) -> Result<UserStatusWriteDto, UserStatusCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: USER_STATUS_CLEAR_COMMAND.to_owned(),
                session_generation: TYPING_PRESENCE_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .map_err(|error| {
                map_user_status_core_error(USER_STATUS_CLEAR_NO_SESSION_CODE, error)
            })?;
        let result: NativeUserStatusWriteResult = serde_json::from_value(response.payload)
            .map_err(|_| {
                user_status_failed(USER_STATUS_FAILED_CODE, USER_STATUS_FAILED_DESCRIPTION)
            })?;
        Ok(UserStatusWriteDto {
            status: result.status,
        })
    }
}
