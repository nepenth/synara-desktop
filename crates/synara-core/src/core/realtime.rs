//! Core command adapters for realtime.

use super::*;

/// Exact React/Tauri envelope payload for `matrix_presence_snapshot`.
///
/// The renderer sends the camel-case `userId` key; unknown keys are rejected
/// so this read-only route cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixPresenceSnapshotRequest {
    pub(super) user_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_presence_subscribe`.
///
/// Shares the snapshot's camel-case `userId` key. Unknown keys are rejected
/// so this subscribe route cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixPresenceSubscribeRequest {
    pub(super) user_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_presence_unsubscribe`.
///
/// The renderer sends the camel-case `subscriptionId` key; unknown keys are
/// rejected so this release route cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixPresenceUnsubscribeRequest {
    pub(super) subscription_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_widgets_list`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixWidgetsListRequest {
    pub(super) experimental_widgets_enabled: bool,
    pub(super) room_id: String,
    #[serde(default)]
    pub(super) agent_widgets: Vec<AgentWidgetEntry>,
}

/// Exact React/Tauri envelope payload for `matrix_widget_open`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixWidgetOpenRequest {
    pub(super) experimental_widgets_enabled: bool,
    pub(super) room_id: String,
    pub(super) widget_id: String,
    pub(super) name: String,
    pub(super) url: String,
    pub(super) kind: WidgetKind,
    pub(super) init_on_content_load: bool,
    pub(super) receive_room: bool,
    pub(super) send_room_message: bool,
}

/// Exact React/Tauri envelope payload for `matrix_widget_close`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixWidgetCloseRequest {
    pub(super) session_id: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_widget_post`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixWidgetPostRequest {
    pub(super) experimental_widgets_enabled: bool,
    pub(super) session_id: String,
    pub(super) message: String,
}

/// Exact React/Tauri envelope payload for `matrix_widget_subscribe`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixWidgetSubscribeRequest {
    pub(super) experimental_widgets_enabled: bool,
}

/// Exact React/Tauri envelope payload for `matrix_presence_set`.
///
/// `state` is the closed online/offline/unavailable vocabulary. Optional
/// `statusMsg` is omitted or empty for no message. Unknown keys are rejected
/// so this write cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixPresenceSetRequest {
    pub(super) state: String,
    #[serde(default)]
    pub(super) status_msg: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_user_status_snapshot`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixUserStatusSnapshotRequest {
    pub(super) user_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_user_status_set`.
///
/// `emoji` and `text` are MSC4426 `m.status` fields. Both empty clears.
/// Unknown keys are rejected so this cannot grow presence `state` or secrets.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixUserStatusSetRequest {
    #[serde(default)]
    pub(super) emoji: String,
    #[serde(default)]
    pub(super) text: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixTypingSetRequest {
    pub(super) room_id: String,
    pub(super) typing: bool,
}

pub(super) fn matrix_typing_set(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTypingSetRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-typing-set-invalid-payload"))?;
        let owner = state.typing_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-typing-set-no-session")
        })?;
        owner
            .set(&payload.room_id, payload.typing)
            .await
            .map_err(typing_set_owner_error)?;
        Ok(serde_json::Value::Null)
    })
}

pub(super) fn typing_set_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms.4-typing-invalid-room" => MatrixIpcErrorCategory::SdkInvariant,
        "v-rooms.4-typing-owner-user-missing" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn matrix_typing_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-typing-snapshot-invalid-payload"));
        }
        let owner = state.typing_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-typing-snapshot-no-session")
        })?;
        let snapshot: NativeTypingSnapshot = owner.snapshot().await;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-typing-snapshot-serialization-failed"))
    })
}

pub(super) fn matrix_presence_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixPresenceSnapshotRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-presence-snapshot-invalid-payload"))?;
        let owner = state.presence_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-presence-snapshot-no-session")
        })?;
        let snapshot: NativePresenceSnapshotResult = owner
            .snapshot(&payload.user_id)
            .await
            .map_err(presence_snapshot_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-presence-snapshot-serialization-failed"))
    })
}

pub(super) fn matrix_presence_subscribe(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixPresenceSubscribeRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-presence-subscribe-invalid-payload"))?;
        let owner = state.presence_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-presence-subscribe-no-session")
        })?;
        let subscription: NativePresenceSubscription = owner
            .subscribe(&payload.user_id)
            .await
            .map_err(presence_snapshot_owner_error)?;
        serde_json::to_value(subscription)
            .map_err(|_| core_state_error("p2-presence-subscribe-serialization-failed"))
    })
}

pub(super) fn matrix_presence_unsubscribe(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixPresenceUnsubscribeRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-presence-unsubscribe-invalid-payload"))?;
        let owner = state.presence_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-presence-unsubscribe-no-session")
        })?;
        owner
            .unsubscribe(&payload.subscription_id)
            .await
            .map_err(presence_snapshot_owner_error)?;
        Ok(serde_json::Value::Null)
    })
}

pub(super) fn matrix_presence_set(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixPresenceSetRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-presence-set-invalid-payload"))?;
        let owner = state.presence_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-presence-set-no-session")
        })?;
        let result: NativePresenceWriteResult = owner
            .set(&payload.state, payload.status_msg)
            .await
            .map_err(presence_set_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-presence-set-serialization-failed"))
    })
}

pub(super) fn matrix_rtc_transports_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error(
                "p2-rtc-transports-snapshot-invalid-payload",
            ));
        }
        let owner = state.rtc_transports_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-rtc-transports-snapshot-no-session")
        })?;
        let snapshot: NativeRtcTransportsSnapshot = owner.snapshot().await;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-rtc-transports-snapshot-serialization-failed"))
    })
}

pub(super) fn matrix_rtc_transports_refresh(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error(
                "p2-rtc-transports-refresh-invalid-payload",
            ));
        }
        let owner = state.rtc_transports_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-rtc-transports-refresh-no-session")
        })?;
        let snapshot: NativeRtcTransportsSnapshot = owner.refresh().await;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-rtc-transports-refresh-serialization-failed"))
    })
}

pub(super) fn matrix_user_status_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixUserStatusSnapshotRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-user-status-snapshot-invalid-payload"))?;
        let owner = state.user_status_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-user-status-snapshot-no-session")
        })?;
        let snapshot: NativeUserStatusSnapshot = owner
            .snapshot(&payload.user_id)
            .await
            .map_err(user_status_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-user-status-snapshot-serialization-failed"))
    })
}

pub(super) fn matrix_user_status_set(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixUserStatusSetRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-user-status-set-invalid-payload"))?;
        let owner = state.user_status_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-user-status-set-no-session")
        })?;
        let result: NativeUserStatusWriteResult = owner
            .set(&payload.emoji, &payload.text)
            .await
            .map_err(user_status_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-user-status-set-serialization-failed"))
    })
}

pub(super) fn matrix_user_status_clear(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-user-status-clear-invalid-payload"));
        }
        let owner = state.user_status_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-user-status-clear-no-session")
        })?;
        let result: NativeUserStatusWriteResult =
            owner.clear().await.map_err(user_status_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-user-status-clear-serialization-failed"))
    })
}

pub(super) fn matrix_widgets_list(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixWidgetsListRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-widgets-list-invalid-payload"))?;
        let owner = state.widget_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-widgets-list-no-session")
        })?;
        let snapshot: WidgetListSnapshot = owner
            .list(
                payload.experimental_widgets_enabled,
                &payload.room_id,
                &payload.agent_widgets,
            )
            .await
            .map_err(widget_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-widgets-list-serialization-failed"))
    })
}

pub(super) fn matrix_widget_open(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixWidgetOpenRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-widget-open-invalid-payload"))?;
        let owner = state.widget_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-widget-open-no-session")
        })?;
        let policy = WidgetGrantPolicy {
            receive_room: payload.receive_room,
            send_room_message: payload.send_room_message,
        };
        let opened: WidgetOpenResult = owner
            .open(
                payload.experimental_widgets_enabled,
                &payload.room_id,
                &payload.widget_id,
                &payload.name,
                &payload.url,
                payload.kind,
                payload.init_on_content_load,
                policy,
            )
            .await
            .map_err(widget_owner_error)?;
        serde_json::to_value(opened)
            .map_err(|_| core_state_error("p2-widget-open-serialization-failed"))
    })
}

pub(super) fn matrix_widget_close(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixWidgetCloseRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-widget-close-invalid-payload"))?;
        let owner = state.widget_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-widget-close-no-session")
        })?;
        let closed = owner
            .close(payload.session_id.as_deref())
            .await
            .map_err(widget_owner_error)?;
        serde_json::to_value(closed)
            .map_err(|_| core_state_error("p2-widget-close-serialization-failed"))
    })
}

pub(super) fn matrix_widget_post(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixWidgetPostRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-widget-post-invalid-payload"))?;
        let owner = state.widget_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-widget-post-no-session")
        })?;
        owner
            .post(
                payload.experimental_widgets_enabled,
                &payload.session_id,
                payload.message,
            )
            .await
            .map_err(widget_owner_error)?;
        Ok(serde_json::Value::Null)
    })
}

pub(super) fn matrix_widget_subscribe(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixWidgetSubscribeRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-widget-subscribe-invalid-payload"))?;
        let owner = state.widget_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-widget-subscribe-no-session")
        })?;
        let sessions: Vec<WidgetSessionRecord> = owner
            .subscribe_snapshot(payload.experimental_widgets_enabled)
            .await
            .map_err(widget_owner_error)?;
        serde_json::to_value(sessions)
            .map_err(|_| core_state_error("p2-widget-subscribe-serialization-failed"))
    })
}

pub(super) fn presence_snapshot_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-presence-invalid-user-id" | "v-presence-invalid-subscription-id" => {
            MatrixIpcErrorCategory::SdkInvariant
        }
        "v-presence-user-owner-missing" | "v-presence-session-not-live" => {
            MatrixIpcErrorCategory::Forbidden
        }
        "v-presence-stale-session-generation" => MatrixIpcErrorCategory::StaleSessionGeneration,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn presence_set_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-presence-state-unsupported" | "p4.7-status-msg-cap" => {
            MatrixIpcErrorCategory::SdkInvariant
        }
        "v-presence-user-owner-missing" | "v-presence-session-not-live" => {
            MatrixIpcErrorCategory::Forbidden
        }
        "v-presence-stale-session-generation" => MatrixIpcErrorCategory::StaleSessionGeneration,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn user_status_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-user-status-emoji-cap"
        | "v-user-status-text-cap"
        | "v-user-status-invalid-user-id"
        | "v-user-status-unsupported" => MatrixIpcErrorCategory::SdkInvariant,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn widget_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "experimental-widgets-disabled"
        | "experimental-widgets-session-not-live"
        | "experimental-widgets-owner-missing" => MatrixIpcErrorCategory::Forbidden,
        "experimental-widgets-url-rejected"
        | "experimental-widgets-room-state-url-rejected"
        | "experimental-widgets-invalid-room"
        | "experimental-widgets-room-missing"
        | "experimental-widgets-session-missing"
        | "experimental-widgets-registry-full"
        | "experimental-widgets-driver-stopped"
        | "experimental-widgets-state-read-failed" => MatrixIpcErrorCategory::SdkInvariant,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}
