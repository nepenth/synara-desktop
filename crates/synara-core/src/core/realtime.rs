//! Core command adapters for realtime.

use super::*;

/// Exact React/Tauri envelope payload for `matrix_presence_snapshot`.
///
/// The renderer sends the camel-case `userId` key; unknown keys are rejected
/// so this read-only route cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixPresenceSnapshotRequest {
    pub user_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_presence_subscribe`.
///
/// Shares the snapshot's camel-case `userId` key. Unknown keys are rejected
/// so this subscribe route cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixPresenceSubscribeRequest {
    pub user_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_presence_unsubscribe`.
///
/// The renderer sends the camel-case `subscriptionId` key; unknown keys are
/// rejected so this release route cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixPresenceUnsubscribeRequest {
    pub subscription_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_widgets_list`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixWidgetsListRequest {
    pub experimental_widgets_enabled: bool,
    pub room_id: String,
    #[serde(default)]
    pub agent_widgets: Vec<AgentWidgetEntry>,
}

/// Exact React/Tauri envelope payload for `matrix_widget_open`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixWidgetOpenRequest {
    pub experimental_widgets_enabled: bool,
    pub room_id: String,
    pub widget_id: String,
    pub name: String,
    pub url: String,
    pub kind: WidgetKind,
    pub init_on_content_load: bool,
    pub receive_room: bool,
    pub send_room_message: bool,
}

/// Exact React/Tauri envelope payload for `matrix_widget_close`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixWidgetCloseRequest {
    pub session_id: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_widget_post`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixWidgetPostRequest {
    pub experimental_widgets_enabled: bool,
    pub session_id: String,
    pub message: String,
}

/// Exact React/Tauri envelope payload for `matrix_widget_subscribe`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixWidgetSubscribeRequest {
    pub experimental_widgets_enabled: bool,
}

/// Exact React/Tauri envelope payload for `matrix_presence_set`.
///
/// `state` is the closed online/offline/unavailable vocabulary. Optional
/// `statusMsg` is omitted or empty for no message. Unknown keys are rejected
/// so this write cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixPresenceSetRequest {
    pub state: String,
    #[serde(default)]
    pub status_msg: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_user_status_snapshot`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixUserStatusSnapshotRequest {
    pub user_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_user_status_set`.
///
/// `emoji` and `text` are MSC4426 `m.status` fields. Both empty clears.
/// Unknown keys are rejected so this cannot grow presence `state` or secrets.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixUserStatusSetRequest {
    #[serde(default)]
    pub emoji: String,
    #[serde(default)]
    pub text: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixTypingSetRequest {
    pub room_id: String,
    pub typing: bool,
}

/// Typed `matrix_typing_set`.
pub(super) async fn typing_set(
    state: &Arc<CoreState>,
    payload: MatrixTypingSetRequest,
) -> Result<(), MatrixIpcError> {
    let owner = state.typing_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-typing-set-no-session")
    })?;
    owner
        .set(&payload.room_id, payload.typing)
        .await
        .map_err(typing_set_owner_error)?;
    Ok(())
}

#[cfg(test)]
pub(super) fn matrix_typing_set(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTypingSetRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-typing-set-invalid-payload"))?;
        typing_set(&state, payload).await?;
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

/// Typed `matrix_typing_snapshot`.
pub(super) async fn typing_snapshot(
    state: &Arc<CoreState>,
) -> Result<NativeTypingSnapshot, MatrixIpcError> {
    let owner = state.typing_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-typing-snapshot-no-session")
    })?;
    let snapshot: NativeTypingSnapshot = owner.snapshot().await;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_typing_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-typing-snapshot-invalid-payload"));
        }
        let response = typing_snapshot(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-typing-snapshot-serialization-failed"))
    })
}

/// Typed `matrix_presence_snapshot`.
pub(super) async fn presence_snapshot(
    state: &Arc<CoreState>,
    payload: MatrixPresenceSnapshotRequest,
) -> Result<NativePresenceSnapshotResult, MatrixIpcError> {
    let owner = state.presence_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-presence-snapshot-no-session")
    })?;
    let snapshot: NativePresenceSnapshotResult = owner
        .snapshot(&payload.user_id)
        .await
        .map_err(presence_snapshot_owner_error)?;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_presence_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixPresenceSnapshotRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-presence-snapshot-invalid-payload"))?;
        let response = presence_snapshot(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-presence-snapshot-serialization-failed"))
    })
}

/// Typed `matrix_presence_subscribe`.
pub(super) async fn presence_subscribe(
    state: &Arc<CoreState>,
    payload: MatrixPresenceSubscribeRequest,
) -> Result<NativePresenceSubscription, MatrixIpcError> {
    let owner = state.presence_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-presence-subscribe-no-session")
    })?;
    let subscription: NativePresenceSubscription = owner
        .subscribe(&payload.user_id)
        .await
        .map_err(presence_snapshot_owner_error)?;
    Ok(subscription)
}

#[cfg(test)]
pub(super) fn matrix_presence_subscribe(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixPresenceSubscribeRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-presence-subscribe-invalid-payload"))?;
        let response = presence_subscribe(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-presence-subscribe-serialization-failed"))
    })
}

/// Typed `matrix_presence_unsubscribe`.
pub(super) async fn presence_unsubscribe(
    state: &Arc<CoreState>,
    payload: MatrixPresenceUnsubscribeRequest,
) -> Result<(), MatrixIpcError> {
    let owner = state.presence_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-presence-unsubscribe-no-session")
    })?;
    owner
        .unsubscribe(&payload.subscription_id)
        .await
        .map_err(presence_snapshot_owner_error)?;
    Ok(())
}

#[cfg(test)]
pub(super) fn matrix_presence_unsubscribe(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixPresenceUnsubscribeRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-presence-unsubscribe-invalid-payload"))?;
        presence_unsubscribe(&state, payload).await?;
        Ok(serde_json::Value::Null)
    })
}

/// Typed `matrix_presence_set`.
pub(super) async fn presence_set(
    state: &Arc<CoreState>,
    payload: MatrixPresenceSetRequest,
) -> Result<NativePresenceWriteResult, MatrixIpcError> {
    let owner = state.presence_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-presence-set-no-session")
    })?;
    let result: NativePresenceWriteResult = owner
        .set(&payload.state, payload.status_msg)
        .await
        .map_err(presence_set_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_presence_set(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixPresenceSetRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-presence-set-invalid-payload"))?;
        let response = presence_set(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-presence-set-serialization-failed"))
    })
}

/// Typed `matrix_rtc_transports_snapshot`.
pub(super) async fn rtc_transports_snapshot(
    state: &Arc<CoreState>,
) -> Result<NativeRtcTransportsSnapshot, MatrixIpcError> {
    let owner = state.rtc_transports_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-rtc-transports-snapshot-no-session")
    })?;
    let snapshot: NativeRtcTransportsSnapshot = owner.snapshot().await;
    Ok(snapshot)
}

#[cfg(test)]
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
        let response = rtc_transports_snapshot(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-rtc-transports-snapshot-serialization-failed"))
    })
}

/// Typed `matrix_rtc_transports_refresh`.
pub(super) async fn rtc_transports_refresh(
    state: &Arc<CoreState>,
) -> Result<NativeRtcTransportsSnapshot, MatrixIpcError> {
    let owner = state.rtc_transports_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-rtc-transports-refresh-no-session")
    })?;
    let snapshot: NativeRtcTransportsSnapshot = owner.refresh().await;
    Ok(snapshot)
}

#[cfg(test)]
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
        let response = rtc_transports_refresh(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-rtc-transports-refresh-serialization-failed"))
    })
}

/// Typed `matrix_user_status_snapshot`.
pub(super) async fn user_status_snapshot(
    state: &Arc<CoreState>,
    payload: MatrixUserStatusSnapshotRequest,
) -> Result<NativeUserStatusSnapshot, MatrixIpcError> {
    let owner = state.user_status_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-user-status-snapshot-no-session")
    })?;
    let snapshot: NativeUserStatusSnapshot = owner
        .snapshot(&payload.user_id)
        .await
        .map_err(user_status_owner_error)?;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_user_status_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixUserStatusSnapshotRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-user-status-snapshot-invalid-payload"))?;
        let response = user_status_snapshot(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-user-status-snapshot-serialization-failed"))
    })
}

/// Typed `matrix_user_status_set`.
pub(super) async fn user_status_set(
    state: &Arc<CoreState>,
    payload: MatrixUserStatusSetRequest,
) -> Result<NativeUserStatusWriteResult, MatrixIpcError> {
    let owner = state.user_status_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-user-status-set-no-session")
    })?;
    let result: NativeUserStatusWriteResult = owner
        .set(&payload.emoji, &payload.text)
        .await
        .map_err(user_status_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_user_status_set(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixUserStatusSetRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-user-status-set-invalid-payload"))?;
        let response = user_status_set(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-user-status-set-serialization-failed"))
    })
}

/// Typed `matrix_user_status_clear`.
pub(super) async fn user_status_clear(
    state: &Arc<CoreState>,
) -> Result<NativeUserStatusWriteResult, MatrixIpcError> {
    let owner = state.user_status_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-user-status-clear-no-session")
    })?;
    let result: NativeUserStatusWriteResult =
        owner.clear().await.map_err(user_status_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_user_status_clear(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-user-status-clear-invalid-payload"));
        }
        let response = user_status_clear(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-user-status-clear-serialization-failed"))
    })
}

/// Typed `matrix_widgets_list`.
pub(super) async fn widgets_list(
    state: &Arc<CoreState>,
    payload: MatrixWidgetsListRequest,
) -> Result<WidgetListSnapshot, MatrixIpcError> {
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
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_widgets_list(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixWidgetsListRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-widgets-list-invalid-payload"))?;
        let response = widgets_list(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-widgets-list-serialization-failed"))
    })
}

/// Typed `matrix_widget_open`.
pub(super) async fn widget_open(
    state: &Arc<CoreState>,
    payload: MatrixWidgetOpenRequest,
) -> Result<WidgetOpenResult, MatrixIpcError> {
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
    Ok(opened)
}

#[cfg(test)]
pub(super) fn matrix_widget_open(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixWidgetOpenRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-widget-open-invalid-payload"))?;
        let response = widget_open(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-widget-open-serialization-failed"))
    })
}

/// Typed `matrix_widget_close`.
pub(super) async fn widget_close(
    state: &Arc<CoreState>,
    payload: MatrixWidgetCloseRequest,
) -> Result<Vec<String>, MatrixIpcError> {
    let owner = state.widget_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-widget-close-no-session")
    })?;
    let closed = owner
        .close(payload.session_id.as_deref())
        .await
        .map_err(widget_owner_error)?;
    Ok(closed)
}

#[cfg(test)]
pub(super) fn matrix_widget_close(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixWidgetCloseRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-widget-close-invalid-payload"))?;
        let response = widget_close(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-widget-close-serialization-failed"))
    })
}

/// Typed `matrix_widget_post`.
pub(super) async fn widget_post(
    state: &Arc<CoreState>,
    payload: MatrixWidgetPostRequest,
) -> Result<(), MatrixIpcError> {
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
    Ok(())
}

#[cfg(test)]
pub(super) fn matrix_widget_post(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixWidgetPostRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-widget-post-invalid-payload"))?;
        widget_post(&state, payload).await?;
        Ok(serde_json::Value::Null)
    })
}

/// Typed `matrix_widget_subscribe`.
pub(super) async fn widget_subscribe(
    state: &Arc<CoreState>,
    payload: MatrixWidgetSubscribeRequest,
) -> Result<Vec<WidgetSessionRecord>, MatrixIpcError> {
    let owner = state.widget_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-widget-subscribe-no-session")
    })?;
    let sessions: Vec<WidgetSessionRecord> = owner
        .subscribe_snapshot(payload.experimental_widgets_enabled)
        .await
        .map_err(widget_owner_error)?;
    Ok(sessions)
}

#[cfg(test)]
pub(super) fn matrix_widget_subscribe(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixWidgetSubscribeRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-widget-subscribe-invalid-payload"))?;
        let response = widget_subscribe(&state, payload).await?;
        serde_json::to_value(response)
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

/// Registers this domain's JSON adapters with the test-only command registry.
#[cfg(test)]
pub(super) fn register_commands(registry: &mut CommandRegistry) {
    registry
        .register("matrix_typing_snapshot", matrix_typing_snapshot)
        .expect("built-in matrix_typing_snapshot must remain in the command census");
    registry
        .register("matrix_presence_set", matrix_presence_set)
        .expect("built-in matrix_presence_set must remain in the command census");
    registry
        .register("matrix_presence_snapshot", matrix_presence_snapshot)
        .expect("built-in matrix_presence_snapshot must remain in the command census");
    registry
        .register("matrix_presence_subscribe", matrix_presence_subscribe)
        .expect("built-in matrix_presence_subscribe must remain in the command census");
    registry
        .register("matrix_presence_unsubscribe", matrix_presence_unsubscribe)
        .expect("built-in matrix_presence_unsubscribe must remain in the command census");
    registry
        .register(
            "matrix_rtc_transports_refresh",
            matrix_rtc_transports_refresh,
        )
        .expect("built-in matrix_rtc_transports_refresh must remain in the command census");
    registry
        .register(
            "matrix_rtc_transports_snapshot",
            matrix_rtc_transports_snapshot,
        )
        .expect("built-in matrix_rtc_transports_snapshot must remain in the command census");
    registry
        .register("matrix_widgets_list", matrix_widgets_list)
        .expect("built-in matrix_widgets_list must remain in the command census");
    registry
        .register("matrix_widget_open", matrix_widget_open)
        .expect("built-in matrix_widget_open must remain in the command census");
    registry
        .register("matrix_widget_close", matrix_widget_close)
        .expect("built-in matrix_widget_close must remain in the command census");
    registry
        .register("matrix_widget_post", matrix_widget_post)
        .expect("built-in matrix_widget_post must remain in the command census");
    registry
        .register("matrix_widget_subscribe", matrix_widget_subscribe)
        .expect("built-in matrix_widget_subscribe must remain in the command census");
    registry
        .register("matrix_user_status_clear", matrix_user_status_clear)
        .expect("built-in matrix_user_status_clear must remain in the command census");
    registry
        .register("matrix_user_status_set", matrix_user_status_set)
        .expect("built-in matrix_user_status_set must remain in the command census");
    registry
        .register("matrix_user_status_snapshot", matrix_user_status_snapshot)
        .expect("built-in matrix_user_status_snapshot must remain in the command census");
    registry
        .register("matrix_typing_set", matrix_typing_set)
        .expect("built-in matrix_typing_set must remain in the command census");
}
