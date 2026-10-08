//! Core command adapters for notifications.

use super::*;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixPushRulesSetDefaultRequest {
    pub encrypted: bool,
    pub one_to_one: bool,
    pub mode: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixPushRulesSetMentionRequest {
    pub rule_id: String,
    pub enabled: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixPushRulesKeywordRequest {
    pub keyword: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomNotificationRoomRequest {
    pub room_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomNotificationSetRequest {
    pub room_id: String,
    pub mode: String,
}

pub(super) fn push_rules_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-push.invalid-mode"
        | "v-push.invalid-rule"
        | "v-push.invalid-keyword"
        | "v-push.invalid-room" => MatrixIpcErrorCategory::SdkInvariant,
        "v-push.no-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn http_pusher_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-pusher.invalid-push-key"
        | "v-pusher.invalid-app-id"
        | "v-pusher.invalid-gateway"
        | "v-pusher.invalid-name"
        | "v-pusher.invalid-lang" => MatrixIpcErrorCategory::SdkInvariant,
        "v-pusher.no-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

/// Typed `matrix_push_rules_snapshot`.
pub(super) async fn push_rules_snapshot(
    state: &Arc<CoreState>,
) -> Result<MatrixPushRulesSnapshot, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-push-rules-snapshot-no-session")
    })?;
    let result: MatrixPushRulesSnapshot = owner
        .snapshot_push_rules()
        .await
        .map_err(push_rules_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_push_rules_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !own_profile_read_payload_is_empty(&request.payload) {
            return Err(core_state_error("p2-push-rules-snapshot-invalid-payload"));
        }
        let response = push_rules_snapshot(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-push-rules-snapshot-serialization-failed"))
    })
}

/// Typed `matrix_push_rules_set_default`.
pub(super) async fn push_rules_set_default(
    state: &Arc<CoreState>,
    payload: MatrixPushRulesSetDefaultRequest,
) -> Result<MatrixPushRulesWriteResult, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-push-rules-set-default-no-session")
    })?;
    let result: MatrixPushRulesWriteResult = owner
        .set_push_rule_default(payload.encrypted, payload.one_to_one, &payload.mode)
        .await
        .map_err(push_rules_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_push_rules_set_default(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixPushRulesSetDefaultRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-push-rules-set-default-invalid-payload"))?;
        let response = push_rules_set_default(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-push-rules-set-default-serialization-failed"))
    })
}

/// Typed `matrix_push_rules_set_mention`.
pub(super) async fn push_rules_set_mention(
    state: &Arc<CoreState>,
    payload: MatrixPushRulesSetMentionRequest,
) -> Result<MatrixPushRulesWriteResult, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-push-rules-set-mention-no-session")
    })?;
    let result: MatrixPushRulesWriteResult = owner
        .set_push_rule_mention(&payload.rule_id, payload.enabled)
        .await
        .map_err(push_rules_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_push_rules_set_mention(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixPushRulesSetMentionRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-push-rules-set-mention-invalid-payload"))?;
        let response = push_rules_set_mention(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-push-rules-set-mention-serialization-failed"))
    })
}

/// Typed `matrix_push_rules_add_keyword`.
pub(super) async fn push_rules_add_keyword(
    state: &Arc<CoreState>,
    payload: MatrixPushRulesKeywordRequest,
) -> Result<MatrixPushRulesWriteResult, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-push-rules-add-keyword-no-session")
    })?;
    let result: MatrixPushRulesWriteResult = owner
        .add_push_keyword(&payload.keyword)
        .await
        .map_err(push_rules_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_push_rules_add_keyword(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixPushRulesKeywordRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-push-rules-add-keyword-invalid-payload"))?;
        let response = push_rules_add_keyword(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-push-rules-add-keyword-serialization-failed"))
    })
}

/// Typed `matrix_push_rules_remove_keyword`.
pub(super) async fn push_rules_remove_keyword(
    state: &Arc<CoreState>,
    payload: MatrixPushRulesKeywordRequest,
) -> Result<MatrixPushRulesWriteResult, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-push-rules-remove-keyword-no-session")
    })?;
    let result: MatrixPushRulesWriteResult = owner
        .remove_push_keyword(&payload.keyword)
        .await
        .map_err(push_rules_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_push_rules_remove_keyword(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixPushRulesKeywordRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-push-rules-remove-keyword-invalid-payload"))?;
        let response = push_rules_remove_keyword(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-push-rules-remove-keyword-serialization-failed"))
    })
}

/// Typed `matrix_room_notification_snapshot`.
pub(super) async fn room_notification_snapshot(
    state: &Arc<CoreState>,
    payload: MatrixRoomNotificationRoomRequest,
) -> Result<MatrixRoomNotificationSnapshot, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-notification-snapshot-no-session")
    })?;
    let result: MatrixRoomNotificationSnapshot = owner
        .snapshot_room_notification(&payload.room_id)
        .await
        .map_err(push_rules_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_room_notification_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomNotificationRoomRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-notification-snapshot-invalid-payload"))?;
        let response = room_notification_snapshot(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-notification-snapshot-serialization-failed"))
    })
}

/// Typed `matrix_room_notification_set`.
pub(super) async fn room_notification_set(
    state: &Arc<CoreState>,
    payload: MatrixRoomNotificationSetRequest,
) -> Result<MatrixPushRulesWriteResult, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-notification-set-no-session")
    })?;
    let result: MatrixRoomNotificationWriteResult = owner
        .set_room_notification(&payload.room_id, &payload.mode)
        .await
        .map_err(push_rules_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_room_notification_set(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomNotificationSetRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-notification-set-invalid-payload"))?;
        let response = room_notification_set(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-notification-set-serialization-failed"))
    })
}

/// Typed `matrix_inbox_notifications`.
pub(super) async fn inbox_notifications(
    state: &Arc<CoreState>,
    payload: crate::app::notifications::MatrixInboxNotificationsRequest,
) -> Result<crate::app::notifications::MatrixInboxNotificationsPage, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("inbox-notifications.no-session")
    })?;
    let result = owner
        .fetch_inbox_notifications(payload)
        .await
        .map_err(|diagnostic| {
            MatrixIpcError::new(if diagnostic == "inbox-notifications.invalid-request" {
                MatrixIpcErrorCategory::SdkInvariant
            } else {
                MatrixIpcErrorCategory::Unknown
            })
            .with_diagnostic(diagnostic)
        })?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_inbox_notifications(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: crate::app::notifications::MatrixInboxNotificationsRequest =
            serde_json::from_value(request.payload)
                .map_err(|_| core_state_error("inbox-notifications.invalid-request"))?;
        let response = inbox_notifications(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("inbox-notifications.serialization-failed"))
    })
}

/// Typed `matrix_room_notifications_snapshot`.
pub(super) async fn room_notifications_snapshot(
    state: &Arc<CoreState>,
) -> Result<MatrixRoomNotificationsSnapshot, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-notifications-snapshot-no-session")
    })?;
    let result: MatrixRoomNotificationsSnapshot = owner
        .snapshot_room_notifications()
        .await
        .map_err(push_rules_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_room_notifications_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !own_profile_read_payload_is_empty(&request.payload) {
            return Err(core_state_error(
                "p2-room-notifications-snapshot-invalid-payload",
            ));
        }
        let response = room_notifications_snapshot(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-notifications-snapshot-serialization-failed"))
    })
}

pub(super) fn notification_decision_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "p2-notification-decide-no-session"
        | "p2-notification-dismiss-no-session"
        | "p2-notification-focus-set-no-session"
        | "p2-notification-pending-snapshot-no-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

/// Typed `matrix_notification_focus_set`.
/// Readback of `matrix_notification_focus_set`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatrixNotificationFocusSetResult {
    /// Always `ok`.
    pub status: String,
}

/// Readback of `matrix_notification_dismiss`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MatrixNotificationDismissResult {
    pub dismissed: bool,
    pub delivery: crate::app::notifications::NotificationDeliveryLedger,
}

/// Readback of `matrix_notification_pending_snapshot`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MatrixNotificationPendingSnapshot {
    pub candidates: Vec<crate::dto::NotificationCandidate>,
    pub delivery: crate::app::notifications::NotificationDeliveryLedger,
}

pub(super) async fn notification_focus_set(
    state: &Arc<CoreState>,
    payload: NativeNotificationFocusSetRequest,
) -> Result<MatrixNotificationFocusSetResult, MatrixIpcError> {
    let owner = state
        .notification_decision_owner()?
        .ok_or_else(|| notification_decision_owner_error("p2-notification-focus-set-no-session"))?;
    owner
        .set_focused_room(payload.room_id.as_deref())
        .map_err(|error| {
            MatrixIpcError::new(error.category()).with_diagnostic(error.diagnostic_id())
        })?;
    Ok(MatrixNotificationFocusSetResult {
        status: "ok".to_owned(),
    })
}

#[cfg(test)]
pub(super) fn matrix_notification_focus_set(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: NativeNotificationFocusSetRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-notification-focus-set-invalid-payload"))?;
        let response = notification_focus_set(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-notification-focus-set-serialization-failed"))
    })
}

/// Typed `matrix_notification_decide`.
pub(super) async fn notification_decide(
    state: &Arc<CoreState>,
    payload: NativeNotificationDecideRequest,
) -> Result<NotificationDecisionReadback, MatrixIpcError> {
    let owner = state
        .notification_decision_owner()?
        .ok_or_else(|| notification_decision_owner_error("p2-notification-decide-no-session"))?;
    // Core resolves the event, its sender, and the SDK push evaluation
    // itself; the renderer supplied identity and presentation only.
    let readback: NotificationDecisionReadback =
        owner.decide_observed(payload).await.map_err(|error| {
            MatrixIpcError::new(error.category()).with_diagnostic(error.diagnostic_id())
        })?;
    // Loading the event can span logout or account replacement. A result
    // from the detached owner must never reach the new session's renderer.
    if !state
        .notification_decision_owner()?
        .is_some_and(|current| Arc::ptr_eq(&current, &owner))
    {
        return Err(notification_decision_owner_error(
            "p2-notification-decide-no-session",
        ));
    }
    Ok(readback)
}

#[cfg(test)]
pub(super) fn matrix_notification_decide(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: NativeNotificationDecideRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-notification-decide-invalid-payload"))?;
        let response = notification_decide(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-notification-decide-serialization-failed"))
    })
}

/// Typed `matrix_notification_dismiss`.
pub(super) async fn notification_dismiss(
    state: &Arc<CoreState>,
    payload: NativeNotificationDismissRequest,
) -> Result<MatrixNotificationDismissResult, MatrixIpcError> {
    let owner = state
        .notification_decision_owner()?
        .ok_or_else(|| notification_decision_owner_error("p2-notification-dismiss-no-session"))?;
    let dismissed = owner
        .dismiss(&payload.candidate_id, payload.outcome)
        .map_err(|error| {
            MatrixIpcError::new(error.category()).with_diagnostic(error.diagnostic_id())
        })?;
    let delivery = owner.delivery_ledger().map_err(|error| {
        MatrixIpcError::new(error.category()).with_diagnostic(error.diagnostic_id())
    })?;
    Ok(MatrixNotificationDismissResult {
        dismissed,
        delivery,
    })
}

#[cfg(test)]
pub(super) fn matrix_notification_dismiss(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: NativeNotificationDismissRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-notification-dismiss-invalid-payload"))?;
        let response = notification_dismiss(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-notification-dismiss-serialization-failed"))
    })
}

/// Typed `matrix_notification_pending_snapshot`.
pub(super) async fn notification_pending_snapshot(
    state: &Arc<CoreState>,
) -> Result<MatrixNotificationPendingSnapshot, MatrixIpcError> {
    let owner = state.notification_decision_owner()?.ok_or_else(|| {
        notification_decision_owner_error("p2-notification-pending-snapshot-no-session")
    })?;
    let candidates = owner.list_pending().map_err(|error| {
        MatrixIpcError::new(error.category()).with_diagnostic(error.diagnostic_id())
    })?;
    let delivery = owner.delivery_ledger().map_err(|error| {
        MatrixIpcError::new(error.category()).with_diagnostic(error.diagnostic_id())
    })?;
    Ok(MatrixNotificationPendingSnapshot {
        candidates,
        delivery,
    })
}

#[cfg(test)]
pub(super) fn matrix_notification_pending_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !own_profile_read_payload_is_empty(&request.payload) {
            return Err(core_state_error(
                "p2-notification-pending-snapshot-invalid-payload",
            ));
        }
        let response = notification_pending_snapshot(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-notification-pending-snapshot-serialization-failed"))
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentPreferencesSetRequest {
    pub preferences: crate::app::notifications::AgentNotificationPreferences,
}
fn agent_preferences_error(diagnostic: &'static str) -> MatrixIpcError {
    MatrixIpcError::new(if diagnostic.ends_with("no-session") {
        MatrixIpcErrorCategory::Forbidden
    } else if diagnostic.ends_with("invalid") {
        MatrixIpcErrorCategory::SdkInvariant
    } else {
        MatrixIpcErrorCategory::Unknown
    })
    .with_diagnostic(diagnostic)
}
/// Typed `matrix_agent_notification_preferences_snapshot`.
pub(super) async fn agent_notification_preferences_snapshot(
    state: &Arc<CoreState>,
) -> Result<crate::app::notifications::AgentNotificationPreferences, MatrixIpcError> {
    let owner = state
        .notification_decision_owner()?
        .ok_or_else(|| agent_preferences_error("agent-notification-preferences-no-session"))?;
    let preferences = owner
        .agent_notification_preferences_snapshot()
        .await
        .map_err(agent_preferences_error)?;
    if !state
        .notification_decision_owner()?
        .is_some_and(|current| Arc::ptr_eq(&owner, &current))
    {
        return Err(agent_preferences_error(
            "agent-notification-preferences-no-session",
        ));
    }
    Ok(preferences)
}

#[cfg(test)]
pub(super) fn matrix_agent_notification_preferences_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !own_profile_read_payload_is_empty(&request.payload) {
            return Err(agent_preferences_error(
                "agent-notification-preferences-invalid",
            ));
        }
        let response = agent_notification_preferences_snapshot(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| agent_preferences_error("agent-notification-preferences-invalid"))
    })
}
/// Typed `matrix_agent_notification_preferences_set`.
pub(super) async fn agent_notification_preferences_set(
    state: &Arc<CoreState>,
    payload: AgentPreferencesSetRequest,
) -> Result<crate::app::notifications::AgentNotificationPreferences, MatrixIpcError> {
    let owner = state
        .notification_decision_owner()?
        .ok_or_else(|| agent_preferences_error("agent-notification-preferences-no-session"))?;
    let preferences = owner
        .agent_notification_preferences_set(payload.preferences)
        .await
        .map_err(agent_preferences_error)?;
    if !state
        .notification_decision_owner()?
        .is_some_and(|current| Arc::ptr_eq(&owner, &current))
    {
        return Err(agent_preferences_error(
            "agent-notification-preferences-no-session",
        ));
    }
    Ok(preferences)
}

#[cfg(test)]
pub(super) fn matrix_agent_notification_preferences_set(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: AgentPreferencesSetRequest = serde_json::from_value(request.payload)
            .map_err(|_| agent_preferences_error("agent-notification-preferences-invalid"))?;
        let response = agent_notification_preferences_set(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| agent_preferences_error("agent-notification-preferences-invalid"))
    })
}

/// Registers this domain's JSON adapters with the test-only command registry.
#[cfg(test)]
pub(super) fn register_commands(registry: &mut CommandRegistry) {
    registry
        .register("matrix_inbox_notifications", matrix_inbox_notifications)
        .expect("built-in matrix_inbox_notifications must remain in the command census");
    registry
        .register(
            "matrix_agent_notification_preferences_snapshot",
            matrix_agent_notification_preferences_snapshot,
        )
        .expect("agent preferences snapshot census");
    registry
        .register(
            "matrix_agent_notification_preferences_set",
            matrix_agent_notification_preferences_set,
        )
        .expect("agent preferences set census");
    registry
        .register("matrix_push_rules_snapshot", matrix_push_rules_snapshot)
        .expect("built-in matrix_push_rules_snapshot must remain in the command census");
    registry
        .register(
            "matrix_push_rules_set_default",
            matrix_push_rules_set_default,
        )
        .expect("built-in matrix_push_rules_set_default must remain in the command census");
    registry
        .register(
            "matrix_push_rules_set_mention",
            matrix_push_rules_set_mention,
        )
        .expect("built-in matrix_push_rules_set_mention must remain in the command census");
    registry
        .register(
            "matrix_push_rules_add_keyword",
            matrix_push_rules_add_keyword,
        )
        .expect("built-in matrix_push_rules_add_keyword must remain in the command census");
    registry
        .register(
            "matrix_push_rules_remove_keyword",
            matrix_push_rules_remove_keyword,
        )
        .expect("built-in matrix_push_rules_remove_keyword must remain in the command census");
    registry
        .register(
            "matrix_room_notification_snapshot",
            matrix_room_notification_snapshot,
        )
        .expect("built-in matrix_room_notification_snapshot must remain in the command census");
    registry
        .register("matrix_room_notification_set", matrix_room_notification_set)
        .expect("built-in matrix_room_notification_set must remain in the command census");
    registry
        .register(
            "matrix_room_notifications_snapshot",
            matrix_room_notifications_snapshot,
        )
        .expect("built-in matrix_room_notifications_snapshot must remain in the command census");
    registry
        .register("matrix_notification_decide", matrix_notification_decide)
        .expect("built-in matrix_notification_decide must remain in the command census");
    registry
        .register("matrix_notification_dismiss", matrix_notification_dismiss)
        .expect("built-in matrix_notification_dismiss must remain in the command census");
    registry
        .register(
            "matrix_notification_focus_set",
            matrix_notification_focus_set,
        )
        .expect("built-in matrix_notification_focus_set must remain in the command census");
    registry
        .register(
            "matrix_notification_pending_snapshot",
            matrix_notification_pending_snapshot,
        )
        .expect("built-in matrix_notification_pending_snapshot must remain in the command census");
}
