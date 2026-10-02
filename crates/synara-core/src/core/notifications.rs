//! Core command adapters for notifications.

use super::*;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixPushRulesSetDefaultRequest {
    pub(super) encrypted: bool,
    pub(super) one_to_one: bool,
    pub(super) mode: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixPushRulesSetMentionRequest {
    pub(super) rule_id: String,
    pub(super) enabled: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixPushRulesKeywordRequest {
    pub(super) keyword: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixRoomNotificationRoomRequest {
    pub(super) room_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixRoomNotificationSetRequest {
    pub(super) room_id: String,
    pub(super) mode: String,
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

pub(super) fn matrix_push_rules_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !own_profile_read_payload_is_empty(&request.payload) {
            return Err(core_state_error("p2-push-rules-snapshot-invalid-payload"));
        }
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-push-rules-snapshot-no-session")
        })?;
        let result: MatrixPushRulesSnapshot = owner
            .snapshot_push_rules()
            .await
            .map_err(push_rules_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-push-rules-snapshot-serialization-failed"))
    })
}

pub(super) fn matrix_push_rules_set_default(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixPushRulesSetDefaultRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-push-rules-set-default-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-push-rules-set-default-no-session")
        })?;
        let result: MatrixPushRulesWriteResult = owner
            .set_push_rule_default(payload.encrypted, payload.one_to_one, &payload.mode)
            .await
            .map_err(push_rules_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-push-rules-set-default-serialization-failed"))
    })
}

pub(super) fn matrix_push_rules_set_mention(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixPushRulesSetMentionRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-push-rules-set-mention-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-push-rules-set-mention-no-session")
        })?;
        let result: MatrixPushRulesWriteResult = owner
            .set_push_rule_mention(&payload.rule_id, payload.enabled)
            .await
            .map_err(push_rules_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-push-rules-set-mention-serialization-failed"))
    })
}

pub(super) fn matrix_push_rules_add_keyword(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixPushRulesKeywordRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-push-rules-add-keyword-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-push-rules-add-keyword-no-session")
        })?;
        let result: MatrixPushRulesWriteResult = owner
            .add_push_keyword(&payload.keyword)
            .await
            .map_err(push_rules_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-push-rules-add-keyword-serialization-failed"))
    })
}

pub(super) fn matrix_push_rules_remove_keyword(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixPushRulesKeywordRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-push-rules-remove-keyword-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-push-rules-remove-keyword-no-session")
        })?;
        let result: MatrixPushRulesWriteResult = owner
            .remove_push_keyword(&payload.keyword)
            .await
            .map_err(push_rules_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-push-rules-remove-keyword-serialization-failed"))
    })
}

pub(super) fn matrix_room_notification_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomNotificationRoomRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-notification-snapshot-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-notification-snapshot-no-session")
        })?;
        let result: MatrixRoomNotificationSnapshot = owner
            .snapshot_room_notification(&payload.room_id)
            .await
            .map_err(push_rules_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-room-notification-snapshot-serialization-failed"))
    })
}

pub(super) fn matrix_room_notification_set(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomNotificationSetRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-notification-set-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-notification-set-no-session")
        })?;
        let result: MatrixRoomNotificationWriteResult = owner
            .set_room_notification(&payload.room_id, &payload.mode)
            .await
            .map_err(push_rules_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-room-notification-set-serialization-failed"))
    })
}

pub(super) fn matrix_inbox_notifications(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: crate::app::notifications::MatrixInboxNotificationsRequest =
            serde_json::from_value(request.payload)
                .map_err(|_| core_state_error("inbox-notifications.invalid-request"))?;
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
        serde_json::to_value(result)
            .map_err(|_| core_state_error("inbox-notifications.serialization-failed"))
    })
}

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
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-notifications-snapshot-no-session")
        })?;
        let result: MatrixRoomNotificationsSnapshot = owner
            .snapshot_room_notifications()
            .await
            .map_err(push_rules_owner_error)?;
        serde_json::to_value(result)
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

pub(super) fn matrix_notification_focus_set(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: NativeNotificationFocusSetRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-notification-focus-set-invalid-payload"))?;
        let owner = state.notification_decision_owner()?.ok_or_else(|| {
            notification_decision_owner_error("p2-notification-focus-set-no-session")
        })?;
        owner
            .set_focused_room(payload.room_id.as_deref())
            .map_err(|error| {
                MatrixIpcError::new(error.category()).with_diagnostic(error.diagnostic_id())
            })?;
        serde_json::to_value(serde_json::json!({ "status": "ok" }))
            .map_err(|_| core_state_error("p2-notification-focus-set-serialization-failed"))
    })
}

pub(super) fn matrix_notification_decide(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: NativeNotificationDecideRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-notification-decide-invalid-payload"))?;
        let owner = state.notification_decision_owner()?.ok_or_else(|| {
            notification_decision_owner_error("p2-notification-decide-no-session")
        })?;
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
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-notification-decide-serialization-failed"))
    })
}

pub(super) fn matrix_notification_dismiss(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: NativeNotificationDismissRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-notification-dismiss-invalid-payload"))?;
        let owner = state.notification_decision_owner()?.ok_or_else(|| {
            notification_decision_owner_error("p2-notification-dismiss-no-session")
        })?;
        let dismissed = owner
            .dismiss(&payload.candidate_id, payload.outcome)
            .map_err(|error| {
                MatrixIpcError::new(error.category()).with_diagnostic(error.diagnostic_id())
            })?;
        let delivery = owner.delivery_ledger().map_err(|error| {
            MatrixIpcError::new(error.category()).with_diagnostic(error.diagnostic_id())
        })?;
        serde_json::to_value(serde_json::json!({ "dismissed": dismissed, "delivery": delivery }))
            .map_err(|_| core_state_error("p2-notification-dismiss-serialization-failed"))
    })
}

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
        let owner = state.notification_decision_owner()?.ok_or_else(|| {
            notification_decision_owner_error("p2-notification-pending-snapshot-no-session")
        })?;
        let candidates = owner.list_pending().map_err(|error| {
            MatrixIpcError::new(error.category()).with_diagnostic(error.diagnostic_id())
        })?;
        let delivery = owner.delivery_ledger().map_err(|error| {
            MatrixIpcError::new(error.category()).with_diagnostic(error.diagnostic_id())
        })?;
        serde_json::to_value(serde_json::json!({ "candidates": candidates, "delivery": delivery }))
            .map_err(|_| core_state_error("p2-notification-pending-snapshot-serialization-failed"))
    })
}
