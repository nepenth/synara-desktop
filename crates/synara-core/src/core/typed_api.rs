//! Typed Core API: one async method per `matrix_*` command.
//!
//! Shells call these directly instead of building a JSON `CommandEnvelope`
//! for an in-process round trip. The registry handlers are thin adapters over
//! the same functions, so both paths share one implementation and wire shape.

use super::*;

impl Core {
    /// Typed `matrix_agent_approval_history_snapshot`.
    pub async fn agent_approval_history_snapshot(
        &self,
    ) -> Result<NativeAgentApprovalHistorySnapshot, MatrixIpcError> {
        agent_approval_history_snapshot(&self.state).await
    }

    /// Typed `matrix_room_directory_protocols`.
    pub async fn room_directory_protocols(
        &self,
    ) -> Result<NativeRoomDirectoryProtocols, MatrixIpcError> {
        room_directory_protocols(&self.state).await
    }

    /// Typed `matrix_room_directory_search`.
    pub async fn room_directory_search(
        &self,
        request: MatrixRoomDirectorySearchRequest,
    ) -> Result<NativeRoomDirectorySearchResponse, MatrixIpcError> {
        room_directory_search(&self.state, request).await
    }

    /// Typed `matrix_room_directory_cancel`.
    pub async fn room_directory_cancel(
        &self,
        request: MatrixRoomDirectoryCancelRequest,
    ) -> Result<NativeRoomDirectorySearchResponse, MatrixIpcError> {
        room_directory_cancel(&self.state, request).await
    }

    /// Typed `matrix_get_room_directory_visibility`.
    pub async fn get_room_directory_visibility(
        &self,
        request: MatrixGetRoomDirectoryVisibilityRequest,
    ) -> Result<MatrixRoomDirectoryVisibilityResult, MatrixIpcError> {
        get_room_directory_visibility(&self.state, request).await
    }

    /// Typed `matrix_set_room_directory_visibility`.
    pub async fn set_room_directory_visibility(
        &self,
        request: MatrixSetRoomDirectoryVisibilityRequest,
    ) -> Result<MatrixRoomDirectoryVisibilityWriteResult, MatrixIpcError> {
        set_room_directory_visibility(&self.state, request).await
    }

    /// Typed `matrix_get_global_image_packs`.
    pub async fn get_global_image_packs(
        &self,
    ) -> Result<NativeGlobalImagePacksSnapshot, MatrixIpcError> {
        get_global_image_packs(&self.state).await
    }

    /// Typed `matrix_get_user_image_pack`.
    pub async fn get_user_image_pack(&self) -> Result<NativeUserImagePackSnapshot, MatrixIpcError> {
        get_user_image_pack(&self.state).await
    }

    /// Typed `matrix_get_room_image_packs`.
    pub async fn get_room_image_packs(
        &self,
        request: MatrixGetRoomImagePacksRequest,
    ) -> Result<NativeRoomImagePacksSnapshot, MatrixIpcError> {
        get_room_image_packs(&self.state, request).await
    }

    /// Typed `matrix_later_snapshot`.
    pub async fn later_snapshot(&self) -> Result<NativeLaterSnapshot, MatrixIpcError> {
        later_snapshot(&self.state).await
    }

    /// Typed `matrix_later_upsert`.
    pub async fn later_upsert(
        &self,
        request: MatrixLaterUpsertRequest,
    ) -> Result<NativeLaterSnapshot, MatrixIpcError> {
        later_upsert(&self.state, request).await
    }

    /// Typed `matrix_later_complete`.
    pub async fn later_complete(
        &self,
        request: MatrixLaterCompleteRequest,
    ) -> Result<NativeLaterSnapshot, MatrixIpcError> {
        later_complete(&self.state, request).await
    }

    /// Typed `matrix_later_snooze`.
    pub async fn later_snooze(
        &self,
        request: MatrixLaterSnoozeRequest,
    ) -> Result<NativeLaterSnapshot, MatrixIpcError> {
        later_snooze(&self.state, request).await
    }

    /// Typed `matrix_later_clear_completed`.
    pub async fn later_clear_completed(&self) -> Result<NativeLaterSnapshot, MatrixIpcError> {
        later_clear_completed(&self.state).await
    }

    /// Typed `matrix_later_mark_reminded`.
    pub async fn later_mark_reminded(
        &self,
        request: MatrixLaterMarkRemindedRequest,
    ) -> Result<NativeLaterSnapshot, MatrixIpcError> {
        later_mark_reminded(&self.state, request).await
    }

    /// Typed `matrix_room_notes_snapshot`.
    pub async fn room_notes_snapshot(&self) -> Result<NativeRoomNotesSnapshot, MatrixIpcError> {
        room_notes_snapshot(&self.state).await
    }

    /// Typed `matrix_room_notes_upsert`.
    pub async fn room_notes_upsert(
        &self,
        request: MatrixRoomNotesUpsertRequest,
    ) -> Result<NativeRoomNotesSnapshot, MatrixIpcError> {
        room_notes_upsert(&self.state, request).await
    }

    /// Typed `matrix_room_notes_delete`.
    pub async fn room_notes_delete(
        &self,
        request: MatrixRoomNotesItemRequest,
    ) -> Result<NativeRoomNotesSnapshot, MatrixIpcError> {
        room_notes_delete(&self.state, request).await
    }

    /// Typed `matrix_room_notes_complete_todo`.
    pub async fn room_notes_complete_todo(
        &self,
        request: MatrixRoomNotesCompleteTodoRequest,
    ) -> Result<NativeRoomNotesSnapshot, MatrixIpcError> {
        room_notes_complete_todo(&self.state, request).await
    }

    /// Typed `matrix_room_notes_move_todo`.
    pub async fn room_notes_move_todo(
        &self,
        request: MatrixRoomNotesMoveTodoRequest,
    ) -> Result<NativeRoomNotesSnapshot, MatrixIpcError> {
        room_notes_move_todo(&self.state, request).await
    }

    /// Typed `matrix_mdirect_snapshot`.
    pub async fn mdirect_snapshot(&self) -> Result<NativeMDirectSnapshot, MatrixIpcError> {
        mdirect_snapshot(&self.state).await
    }

    /// Typed `matrix_mdirect_add`.
    pub async fn mdirect_add(
        &self,
        request: MatrixMDirectAddRequest,
    ) -> Result<NativeMDirectMutationResult, MatrixIpcError> {
        mdirect_add(&self.state, request).await
    }

    /// Typed `matrix_mdirect_remove`.
    pub async fn mdirect_remove(
        &self,
        request: MatrixMDirectRemoveRequest,
    ) -> Result<NativeMDirectMutationResult, MatrixIpcError> {
        mdirect_remove(&self.state, request).await
    }

    /// Typed `matrix_timeline_open`.
    pub async fn timeline_open(
        &self,
        request: MatrixTimelineOpenRequest,
    ) -> Result<NativeTimelineOpenReadback, MatrixIpcError> {
        timeline_open(&self.state, request).await
    }

    /// Typed `matrix_timeline_jump_latest`.
    pub async fn timeline_jump_latest(
        &self,
        request: MatrixTimelineJumpLatestRequest,
    ) -> Result<NativeTimelineOpenReadback, MatrixIpcError> {
        timeline_jump_latest(&self.state, request).await
    }

    /// Typed `matrix_timeline_snapshot`.
    pub async fn timeline_snapshot(
        &self,
        request: MatrixTimelineSnapshotRequest,
    ) -> Result<TimelineViewSnapshot, MatrixIpcError> {
        timeline_snapshot(&self.state, request).await
    }

    /// Typed `matrix_timeline_event_readback`.
    pub async fn timeline_event_readback(
        &self,
        request: MatrixTimelineEventReadbackRequest,
    ) -> Result<NativeTimelineEventReadback, MatrixIpcError> {
        timeline_event_readback(&self.state, request).await
    }

    /// Typed `matrix_timeline_timestamp_to_event`.
    pub async fn timeline_timestamp_to_event(
        &self,
        request: MatrixTimelineTimestampToEventRequest,
    ) -> Result<NativeTimelineTimestampToEventReadback, MatrixIpcError> {
        timeline_timestamp_to_event(&self.state, request).await
    }

    /// Typed `matrix_timeline_paginate`.
    pub async fn timeline_paginate(
        &self,
        request: MatrixTimelinePaginateRequest,
    ) -> Result<TimelineViewSnapshot, MatrixIpcError> {
        timeline_paginate(&self.state, request).await
    }

    /// Typed `matrix_timeline_set_read_state`.
    pub async fn timeline_set_read_state(
        &self,
        request: MatrixTimelineSetReadStateRequest,
    ) -> Result<NativeTimelineReadStateReadback, MatrixIpcError> {
        timeline_set_read_state(&self.state, request).await
    }

    /// Typed `matrix_timeline_follow_live`.
    pub async fn timeline_follow_live(
        &self,
        request: MatrixTimelineFollowLiveRequest,
    ) -> Result<TimelineViewSnapshot, MatrixIpcError> {
        timeline_follow_live(&self.state, request).await
    }

    /// Typed `matrix_timeline_reaction_toggle`.
    pub async fn timeline_reaction_toggle(
        &self,
        request: MatrixTimelineReactionKeyRequest,
    ) -> Result<NativeReactionMutationResult, MatrixIpcError> {
        timeline_reaction_toggle(&self.state, request).await
    }

    /// Typed `matrix_reaction_ensure`.
    pub async fn reaction_ensure(
        &self,
        request: MatrixTimelineReactionKeyRequest,
    ) -> Result<NativeReactionMutationResult, MatrixIpcError> {
        reaction_ensure(&self.state, request).await
    }

    /// Typed `matrix_agent_approval_decide`.
    pub async fn agent_approval_decide(
        &self,
        request: MatrixAgentApprovalDecisionRequest,
    ) -> Result<NativeAgentApprovalDecisionResult, MatrixIpcError> {
        agent_approval_decide(&self.state, request).await
    }

    /// Typed `matrix_reaction_redact`.
    pub async fn reaction_redact(
        &self,
        request: MatrixReactionRedactRequest,
    ) -> Result<NativeReactionMutationResult, MatrixIpcError> {
        reaction_redact(&self.state, request).await
    }

    /// Typed `matrix_send_text`.
    pub async fn send_text(
        &self,
        request: MatrixSendTextRequest,
    ) -> Result<MatrixSendTextResult, MatrixIpcError> {
        send_text(&self.state, request).await
    }

    /// Typed `matrix_local_echo_retry`.
    pub async fn local_echo_retry(
        &self,
        request: MatrixLocalEchoRequest,
    ) -> Result<MatrixLocalEchoRetryResult, MatrixIpcError> {
        local_echo_retry(&self.state, request).await
    }

    /// Typed `matrix_send_poll`.
    pub async fn send_poll(
        &self,
        request: MatrixSendPollRequest,
    ) -> Result<MatrixSendPollResult, MatrixIpcError> {
        send_poll(&self.state, request).await
    }

    /// Typed `matrix_poll_respond`.
    pub async fn poll_respond(
        &self,
        request: MatrixPollRespondRequest,
    ) -> Result<MatrixPollRespondResult, MatrixIpcError> {
        poll_respond(&self.state, request).await
    }

    /// Typed `matrix_edit_message`.
    pub async fn edit_message(
        &self,
        request: MatrixEditMessageRequest,
    ) -> Result<MatrixSendTextResult, MatrixIpcError> {
        edit_message(&self.state, request).await
    }

    /// Typed `matrix_timeline_edit_text`.
    pub async fn timeline_edit_text(
        &self,
        request: MatrixTimelineEditTextRequest,
    ) -> Result<NativeTimelineActionReadback, MatrixIpcError> {
        timeline_edit_text(&self.state, request).await
    }

    /// Typed `matrix_timeline_redact`.
    pub async fn timeline_redact(
        &self,
        request: MatrixTimelineRedactRequest,
    ) -> Result<NativeTimelineActionReadback, MatrixIpcError> {
        timeline_redact(&self.state, request).await
    }

    /// Typed `matrix_timeline_report`.
    pub async fn timeline_report(
        &self,
        request: MatrixTimelineReportRequest,
    ) -> Result<NativeTimelineActionReadback, MatrixIpcError> {
        timeline_report(&self.state, request).await
    }

    /// Typed `matrix_timeline_pin`.
    pub async fn timeline_pin(
        &self,
        request: MatrixTimelinePinRequest,
    ) -> Result<NativeTimelineActionReadback, MatrixIpcError> {
        timeline_pin(&self.state, request).await
    }

    /// Typed `matrix_timeline_unpin`.
    pub async fn timeline_unpin(
        &self,
        request: MatrixTimelinePinRequest,
    ) -> Result<NativeTimelineActionReadback, MatrixIpcError> {
        timeline_unpin(&self.state, request).await
    }

    /// Typed `matrix_pinned_events`.
    pub async fn pinned_events(
        &self,
        request: MatrixPinnedEventsRequest,
    ) -> Result<crate::app::timeline::PinnedEventsSnapshot, MatrixIpcError> {
        pinned_events(&self.state, request).await
    }

    /// Typed `matrix_timeline_poll_vote`.
    pub async fn timeline_poll_vote(
        &self,
        request: MatrixTimelinePollVoteRequest,
    ) -> Result<NativeTimelineActionReadback, MatrixIpcError> {
        timeline_poll_vote(&self.state, request).await
    }

    /// Typed `matrix_timeline_call_decline`.
    pub async fn timeline_call_decline(
        &self,
        request: MatrixTimelineCallDeclineRequest,
    ) -> Result<NativeTimelineActionReadback, MatrixIpcError> {
        timeline_call_decline(&self.state, request).await
    }

    /// Typed `matrix_timeline_forward_text`.
    pub async fn timeline_forward_text(
        &self,
        request: MatrixTimelineForwardTextRequest,
    ) -> Result<NativeTimelineActionReadback, MatrixIpcError> {
        timeline_forward_text(&self.state, request).await
    }

    /// Typed `matrix_timeline_forward_media`.
    pub async fn timeline_forward_media(
        &self,
        request: MatrixTimelineForwardMediaRequest,
    ) -> Result<NativeTimelineActionReadback, MatrixIpcError> {
        timeline_forward_media(&self.state, request).await
    }

    /// Typed `matrix_composer_set_reply_draft`.
    pub async fn composer_set_reply_draft(
        &self,
        request: MatrixComposerSetReplyDraftRequest,
    ) -> Result<NativeComposerReplyDraftReadback, MatrixIpcError> {
        composer_set_reply_draft(&self.state, request).await
    }

    /// Typed `matrix_composer_clear_reply_draft`.
    pub async fn composer_clear_reply_draft(
        &self,
        request: MatrixComposerClearReplyDraftRequest,
    ) -> Result<NativeComposerReplyDraftReadback, MatrixIpcError> {
        composer_clear_reply_draft(&self.state, request).await
    }

    /// Typed `matrix_composer_get_reply_draft`.
    pub async fn composer_get_reply_draft(
        &self,
        request: MatrixComposerReplyDraftRoomRequest,
    ) -> Result<NativeComposerReplyDraftReadback, MatrixIpcError> {
        composer_get_reply_draft(&self.state, request).await
    }

    /// Typed `matrix_thread_list`.
    pub async fn thread_list(
        &self,
        request: MatrixThreadListRequest,
    ) -> Result<NativeThreadListSnapshot, MatrixIpcError> {
        thread_list(&self.state, request).await
    }

    /// Typed `matrix_push_rules_snapshot`.
    pub async fn push_rules_snapshot(&self) -> Result<MatrixPushRulesSnapshot, MatrixIpcError> {
        push_rules_snapshot(&self.state).await
    }

    /// Typed `matrix_push_rules_set_default`.
    pub async fn push_rules_set_default(
        &self,
        request: MatrixPushRulesSetDefaultRequest,
    ) -> Result<MatrixPushRulesWriteResult, MatrixIpcError> {
        push_rules_set_default(&self.state, request).await
    }

    /// Typed `matrix_push_rules_set_mention`.
    pub async fn push_rules_set_mention(
        &self,
        request: MatrixPushRulesSetMentionRequest,
    ) -> Result<MatrixPushRulesWriteResult, MatrixIpcError> {
        push_rules_set_mention(&self.state, request).await
    }

    /// Typed `matrix_push_rules_add_keyword`.
    pub async fn push_rules_add_keyword(
        &self,
        request: MatrixPushRulesKeywordRequest,
    ) -> Result<MatrixPushRulesWriteResult, MatrixIpcError> {
        push_rules_add_keyword(&self.state, request).await
    }

    /// Typed `matrix_push_rules_remove_keyword`.
    pub async fn push_rules_remove_keyword(
        &self,
        request: MatrixPushRulesKeywordRequest,
    ) -> Result<MatrixPushRulesWriteResult, MatrixIpcError> {
        push_rules_remove_keyword(&self.state, request).await
    }

    /// Typed `matrix_room_notification_snapshot`.
    pub async fn room_notification_snapshot(
        &self,
        request: MatrixRoomNotificationRoomRequest,
    ) -> Result<MatrixRoomNotificationSnapshot, MatrixIpcError> {
        room_notification_snapshot(&self.state, request).await
    }

    /// Typed `matrix_room_notification_set`.
    pub async fn room_notification_set(
        &self,
        request: MatrixRoomNotificationSetRequest,
    ) -> Result<MatrixPushRulesWriteResult, MatrixIpcError> {
        room_notification_set(&self.state, request).await
    }

    /// Typed `matrix_inbox_notifications`.
    pub async fn inbox_notifications(
        &self,
        request: crate::app::notifications::MatrixInboxNotificationsRequest,
    ) -> Result<crate::app::notifications::MatrixInboxNotificationsPage, MatrixIpcError> {
        inbox_notifications(&self.state, request).await
    }

    /// Typed `matrix_room_notifications_snapshot`.
    pub async fn room_notifications_snapshot(
        &self,
    ) -> Result<MatrixRoomNotificationsSnapshot, MatrixIpcError> {
        room_notifications_snapshot(&self.state).await
    }

    /// Typed `matrix_notification_decide`.
    pub async fn notification_decide(
        &self,
        request: NativeNotificationDecideRequest,
    ) -> Result<NotificationDecisionReadback, MatrixIpcError> {
        notification_decide(&self.state, request).await
    }

    /// Typed `matrix_notification_dismiss`.
    pub async fn notification_dismiss(
        &self,
        request: NativeNotificationDismissRequest,
    ) -> Result<MatrixNotificationDismissResult, MatrixIpcError> {
        notification_dismiss(&self.state, request).await
    }

    /// Typed `matrix_notification_pending_snapshot`.
    pub async fn notification_pending_snapshot(
        &self,
    ) -> Result<MatrixNotificationPendingSnapshot, MatrixIpcError> {
        notification_pending_snapshot(&self.state).await
    }

    /// Typed `matrix_agent_notification_preferences_snapshot`.
    pub async fn agent_notification_preferences_snapshot(
        &self,
    ) -> Result<crate::app::notifications::AgentNotificationPreferences, MatrixIpcError> {
        agent_notification_preferences_snapshot(&self.state).await
    }

    /// Typed `matrix_agent_notification_preferences_set`.
    pub async fn agent_notification_preferences_set(
        &self,
        request: AgentPreferencesSetRequest,
    ) -> Result<crate::app::notifications::AgentNotificationPreferences, MatrixIpcError> {
        agent_notification_preferences_set(&self.state, request).await
    }

    /// Typed `matrix_media_preview`.
    pub async fn media_preview(
        &self,
        request: MatrixMediaPreviewRequest,
    ) -> Result<MatrixMediaPreviewSnapshot, MatrixIpcError> {
        media_preview(&self.state, request).await
    }

    /// Typed `matrix_set_own_display_name`.
    pub async fn set_own_display_name(
        &self,
        request: MatrixSetOwnDisplayNameRequest,
    ) -> Result<MatrixProfileWriteResult, MatrixIpcError> {
        set_own_display_name(&self.state, request).await
    }

    /// Typed `matrix_set_own_avatar`.
    pub async fn set_own_avatar(
        &self,
        request: MatrixSetOwnAvatarRequest,
    ) -> Result<MatrixProfileWriteResult, MatrixIpcError> {
        set_own_avatar(&self.state, request).await
    }

    /// Typed `matrix_get_own_profile`.
    pub async fn get_own_profile(&self) -> Result<MatrixOwnProfile, MatrixIpcError> {
        get_own_profile(&self.state).await
    }

    /// Typed `matrix_ignored_users_snapshot`.
    pub async fn ignored_users_snapshot(
        &self,
    ) -> Result<MatrixIgnoredUsersSnapshot, MatrixIpcError> {
        ignored_users_snapshot(&self.state).await
    }

    /// Typed `matrix_ignored_users_ignore`.
    pub async fn ignored_users_ignore(
        &self,
        request: MatrixIgnoredUsersUserRequest,
    ) -> Result<MatrixIgnoredUsersWriteResult, MatrixIpcError> {
        ignored_users_ignore(&self.state, request).await
    }

    /// Typed `matrix_ignored_users_unignore`.
    pub async fn ignored_users_unignore(
        &self,
        request: MatrixIgnoredUsersUserRequest,
    ) -> Result<MatrixIgnoredUsersWriteResult, MatrixIpcError> {
        ignored_users_unignore(&self.state, request).await
    }

    /// Typed `matrix_user_directory_search`.
    pub async fn user_directory_search(
        &self,
        request: MatrixUserDirectorySearchRequest,
    ) -> Result<MatrixUserDirectorySearchResult, MatrixIpcError> {
        user_directory_search(&self.state, request).await
    }

    /// Typed `matrix_message_search`.
    pub async fn message_search(
        &self,
        request: MatrixMessageSearchRequest,
    ) -> Result<MatrixMessageSearchResult, MatrixIpcError> {
        message_search(&self.state, request).await
    }

    /// Typed `matrix_threepid_snapshot`.
    pub async fn threepid_snapshot(&self) -> Result<MatrixThreepidSnapshot, MatrixIpcError> {
        threepid_snapshot(&self.state).await
    }

    /// Typed `matrix_threepid_delete`.
    pub async fn threepid_delete(
        &self,
        request: MatrixThreepidAddressRequest,
    ) -> Result<MatrixThreepidWriteResult, MatrixIpcError> {
        threepid_delete(&self.state, request).await
    }

    /// Typed `matrix_threepid_request_email_token`.
    pub async fn threepid_request_email_token(
        &self,
        request: MatrixThreepidEmailRequest,
    ) -> Result<MatrixThreepidEmailTokenResult, MatrixIpcError> {
        threepid_request_email_token(&self.state, request).await
    }

    /// Typed `matrix_threepid_add_email`.
    pub async fn threepid_add_email(&self) -> Result<MatrixThreepidAddResult, MatrixIpcError> {
        threepid_add_email(&self.state).await
    }

    /// Typed `matrix_media_config`.
    pub async fn media_config(&self) -> Result<MatrixMediaConfigResponse, MatrixIpcError> {
        media_config(&self.state).await
    }

    /// Typed `matrix_typing_set`.
    pub async fn typing_set(&self, request: MatrixTypingSetRequest) -> Result<(), MatrixIpcError> {
        typing_set(&self.state, request).await
    }

    /// Typed `matrix_typing_snapshot`.
    pub async fn typing_snapshot(&self) -> Result<NativeTypingSnapshot, MatrixIpcError> {
        typing_snapshot(&self.state).await
    }

    /// Typed `matrix_presence_snapshot`.
    pub async fn presence_snapshot(
        &self,
        request: MatrixPresenceSnapshotRequest,
    ) -> Result<NativePresenceSnapshotResult, MatrixIpcError> {
        presence_snapshot(&self.state, request).await
    }

    /// Typed `matrix_presence_subscribe`.
    pub async fn presence_subscribe(
        &self,
        request: MatrixPresenceSubscribeRequest,
    ) -> Result<NativePresenceSubscription, MatrixIpcError> {
        presence_subscribe(&self.state, request).await
    }

    /// Typed `matrix_presence_unsubscribe`.
    pub async fn presence_unsubscribe(
        &self,
        request: MatrixPresenceUnsubscribeRequest,
    ) -> Result<(), MatrixIpcError> {
        presence_unsubscribe(&self.state, request).await
    }

    /// Typed `matrix_presence_set`.
    pub async fn presence_set(
        &self,
        request: MatrixPresenceSetRequest,
    ) -> Result<NativePresenceWriteResult, MatrixIpcError> {
        presence_set(&self.state, request).await
    }

    /// Typed `matrix_rtc_transports_snapshot`.
    pub async fn rtc_transports_snapshot(
        &self,
    ) -> Result<NativeRtcTransportsSnapshot, MatrixIpcError> {
        rtc_transports_snapshot(&self.state).await
    }

    /// Typed `matrix_rtc_transports_refresh`.
    pub async fn rtc_transports_refresh(
        &self,
    ) -> Result<NativeRtcTransportsSnapshot, MatrixIpcError> {
        rtc_transports_refresh(&self.state).await
    }

    /// Typed `matrix_user_status_snapshot`.
    pub async fn user_status_snapshot(
        &self,
        request: MatrixUserStatusSnapshotRequest,
    ) -> Result<NativeUserStatusSnapshot, MatrixIpcError> {
        user_status_snapshot(&self.state, request).await
    }

    /// Typed `matrix_user_status_set`.
    pub async fn user_status_set(
        &self,
        request: MatrixUserStatusSetRequest,
    ) -> Result<NativeUserStatusWriteResult, MatrixIpcError> {
        user_status_set(&self.state, request).await
    }

    /// Typed `matrix_user_status_clear`.
    pub async fn user_status_clear(&self) -> Result<NativeUserStatusWriteResult, MatrixIpcError> {
        user_status_clear(&self.state).await
    }

    /// Typed `matrix_widgets_list`.
    pub async fn widgets_list(
        &self,
        request: MatrixWidgetsListRequest,
    ) -> Result<WidgetListSnapshot, MatrixIpcError> {
        widgets_list(&self.state, request).await
    }

    /// Typed `matrix_widget_open`.
    pub async fn widget_open(
        &self,
        request: MatrixWidgetOpenRequest,
    ) -> Result<WidgetOpenResult, MatrixIpcError> {
        widget_open(&self.state, request).await
    }

    /// Typed `matrix_widget_close`.
    pub async fn widget_close(
        &self,
        request: MatrixWidgetCloseRequest,
    ) -> Result<Vec<String>, MatrixIpcError> {
        widget_close(&self.state, request).await
    }

    /// Typed `matrix_widget_post`.
    pub async fn widget_post(
        &self,
        request: MatrixWidgetPostRequest,
    ) -> Result<(), MatrixIpcError> {
        widget_post(&self.state, request).await
    }

    /// Typed `matrix_widget_subscribe`.
    pub async fn widget_subscribe(
        &self,
        request: MatrixWidgetSubscribeRequest,
    ) -> Result<Vec<WidgetSessionRecord>, MatrixIpcError> {
        widget_subscribe(&self.state, request).await
    }

    /// Typed `matrix_room_list_snapshot`.
    pub async fn room_list_snapshot(&self) -> Result<NativeRoomListSnapshot, MatrixIpcError> {
        room_list_snapshot(&self.state).await
    }

    /// Typed `matrix_invites_snapshot`.
    pub async fn invites_snapshot(&self) -> Result<NativeInviteSnapshot, MatrixIpcError> {
        invites_snapshot(&self.state).await
    }

    /// Typed `matrix_invites_accept`.
    pub async fn invites_accept(
        &self,
        request: MatrixInviteActionRequest,
    ) -> Result<NativeInviteSnapshot, MatrixIpcError> {
        invites_accept(&self.state, request).await
    }

    /// Typed `matrix_invites_decline`.
    pub async fn invites_decline(
        &self,
        request: MatrixInviteActionRequest,
    ) -> Result<NativeInviteSnapshot, MatrixIpcError> {
        invites_decline(&self.state, request).await
    }

    /// Typed `matrix_invites_report_spam`.
    pub async fn invites_report_spam(
        &self,
        request: MatrixInviteActionRequest,
    ) -> Result<NativeInviteSnapshot, MatrixIpcError> {
        invites_report_spam(&self.state, request).await
    }

    /// Typed `matrix_invites_block_sender`.
    pub async fn invites_block_sender(
        &self,
        request: MatrixInviteActionRequest,
    ) -> Result<NativeInviteSnapshot, MatrixIpcError> {
        invites_block_sender(&self.state, request).await
    }

    /// Typed `matrix_room_join_rule_snapshot`.
    pub async fn room_join_rule_snapshot(
        &self,
        request: MatrixRoomJoinRuleSnapshotRequest,
    ) -> Result<MatrixRoomJoinRuleSnapshot, MatrixIpcError> {
        room_join_rule_snapshot(&self.state, request).await
    }

    /// Typed `matrix_room_set_join_rule`.
    pub async fn room_set_join_rule(
        &self,
        request: MatrixRoomSetJoinRuleRequest,
    ) -> Result<MatrixProfileWriteResult, MatrixIpcError> {
        room_set_join_rule(&self.state, request).await
    }

    /// Typed `matrix_room_leave`.
    pub async fn room_leave(&self, request: MatrixRoomLeaveRequest) -> Result<(), MatrixIpcError> {
        room_leave(&self.state, request).await
    }

    /// Typed `matrix_room_join`.
    pub async fn room_join(&self, request: MatrixRoomJoinRequest) -> Result<(), MatrixIpcError> {
        room_join(&self.state, request).await
    }

    /// Typed `matrix_room_set_favorite`.
    pub async fn room_set_favorite(
        &self,
        request: MatrixRoomSetFavoriteRequest,
    ) -> Result<(), MatrixIpcError> {
        room_set_favorite(&self.state, request).await
    }

    /// Typed `matrix_room_set_read_state`.
    pub async fn room_set_read_state(
        &self,
        request: MatrixRoomSetReadStateRequest,
    ) -> Result<crate::app::timeline::NativeRoomReadStateReadback, MatrixIpcError> {
        room_set_read_state(&self.state, request).await
    }

    /// Typed `matrix_room_invite`.
    pub async fn room_invite(
        &self,
        request: MatrixRoomModerationRequest,
    ) -> Result<(), MatrixIpcError> {
        room_invite(&self.state, request).await
    }

    /// Typed `matrix_room_kick`.
    pub async fn room_kick(
        &self,
        request: MatrixRoomModerationRequest,
    ) -> Result<(), MatrixIpcError> {
        room_kick(&self.state, request).await
    }

    /// Typed `matrix_room_ban`.
    pub async fn room_ban(
        &self,
        request: MatrixRoomModerationRequest,
    ) -> Result<(), MatrixIpcError> {
        room_ban(&self.state, request).await
    }

    /// Typed `matrix_room_unban`.
    pub async fn room_unban(&self, request: MatrixRoomUnbanRequest) -> Result<(), MatrixIpcError> {
        room_unban(&self.state, request).await
    }

    /// Typed `matrix_room_members_snapshot`.
    pub async fn room_members_snapshot(
        &self,
        request: MatrixRoomMembersSnapshotRequest,
    ) -> Result<NativeRoomMembersSnapshot, MatrixIpcError> {
        room_members_snapshot(&self.state, request).await
    }

    /// Typed `matrix_room_power_levels_snapshot`.
    pub async fn room_power_levels_snapshot(
        &self,
        request: MatrixRoomMembersSnapshotRequest,
    ) -> Result<NativeRoomPowerLevelsSnapshot, MatrixIpcError> {
        room_power_levels_snapshot(&self.state, request).await
    }

    /// Typed `matrix_room_creators_snapshot`.
    pub async fn room_creators_snapshot(
        &self,
        request: MatrixRoomMembersSnapshotRequest,
    ) -> Result<NativeRoomCreatorsSnapshot, MatrixIpcError> {
        room_creators_snapshot(&self.state, request).await
    }

    /// Typed `matrix_room_power_level_tags_snapshot`.
    pub async fn room_power_level_tags_snapshot(
        &self,
        request: MatrixRoomMembersSnapshotRequest,
    ) -> Result<NativeRoomPowerLevelTagsSnapshot, MatrixIpcError> {
        room_power_level_tags_snapshot(&self.state, request).await
    }

    /// Typed `matrix_space_parents_snapshot`.
    pub async fn space_parents_snapshot(
        &self,
    ) -> Result<NativeSpaceParentsSnapshot, MatrixIpcError> {
        space_parents_snapshot(&self.state).await
    }

    /// Typed `matrix_space_hierarchy_snapshot`.
    pub async fn space_hierarchy_snapshot(
        &self,
        request: MatrixSpaceHierarchySnapshotRequest,
    ) -> Result<NativeSpaceHierarchySnapshot, MatrixIpcError> {
        space_hierarchy_snapshot(&self.state, request).await
    }

    /// Typed `matrix_space_children_snapshot`.
    pub async fn space_children_snapshot(
        &self,
    ) -> Result<NativeSpaceChildrenSnapshot, MatrixIpcError> {
        space_children_snapshot(&self.state).await
    }

    /// Typed `matrix_space_child_set`.
    pub async fn space_child_set(
        &self,
        request: MatrixSpaceChildSetRequest,
    ) -> Result<NativeSpaceChildMutationResult, MatrixIpcError> {
        space_child_set(&self.state, request).await
    }

    /// Typed `matrix_space_child_remove`.
    pub async fn space_child_remove(
        &self,
        request: MatrixSpaceChildRemoveRequest,
    ) -> Result<NativeSpaceChildMutationResult, MatrixIpcError> {
        space_child_remove(&self.state, request).await
    }

    /// Typed `matrix_restricted_join_reparent`.
    pub async fn restricted_join_reparent(
        &self,
        request: MatrixRestrictedJoinReparentRequest,
    ) -> Result<NativeRestrictedJoinReparentResult, MatrixIpcError> {
        restricted_join_reparent(&self.state, request).await
    }

    /// Typed `matrix_room_set_power_level`.
    pub async fn room_set_power_level(
        &self,
        request: MatrixRoomSetPowerLevelRequest,
    ) -> Result<(), MatrixIpcError> {
        room_set_power_level(&self.state, request).await
    }

    /// Typed `matrix_room_set_power_levels`.
    pub async fn room_set_power_levels(
        &self,
        request: MatrixRoomSetPowerLevelStateRequest,
    ) -> Result<NativePowerLevelWriteResult, MatrixIpcError> {
        room_set_power_levels(&self.state, request).await
    }

    /// Typed `matrix_room_set_power_level_tags`.
    pub async fn room_set_power_level_tags(
        &self,
        request: MatrixRoomSetPowerLevelStateRequest,
    ) -> Result<NativePowerLevelWriteResult, MatrixIpcError> {
        room_set_power_level_tags(&self.state, request).await
    }

    /// Typed `matrix_set_room_name`.
    pub async fn set_room_name(
        &self,
        request: MatrixSetRoomNameRequest,
    ) -> Result<MatrixProfileWriteResult, MatrixIpcError> {
        set_room_name(&self.state, request).await
    }

    /// Typed `matrix_set_room_topic`.
    pub async fn set_room_topic(
        &self,
        request: MatrixSetRoomTopicRequest,
    ) -> Result<MatrixProfileWriteResult, MatrixIpcError> {
        set_room_topic(&self.state, request).await
    }

    /// Typed `matrix_set_room_avatar`.
    pub async fn set_room_avatar(
        &self,
        request: MatrixSetRoomAvatarRequest,
    ) -> Result<MatrixProfileWriteResult, MatrixIpcError> {
        set_room_avatar(&self.state, request).await
    }

    /// Typed `matrix_send_state_event`.
    pub async fn send_state_event(
        &self,
        request: MatrixSendStateEventRequest,
    ) -> Result<MatrixProfileWriteResult, MatrixIpcError> {
        send_state_event(&self.state, request).await
    }

    /// Typed `matrix_enable_room_encrypted_state`.
    pub async fn enable_room_encrypted_state(
        &self,
        request: MatrixEnableRoomEncryptedStateRequest,
    ) -> Result<MatrixProfileWriteResult, MatrixIpcError> {
        enable_room_encrypted_state(&self.state, request).await
    }

    /// Typed `matrix_room_retention`.
    pub async fn room_retention(
        &self,
        request: MatrixRoomRetentionRequest,
    ) -> Result<MatrixRoomRetentionSnapshot, MatrixIpcError> {
        room_retention(&self.state, request).await
    }

    /// Typed `matrix_verification_list`.
    pub async fn verification_list(&self) -> Result<NativeVerificationInbox, MatrixIpcError> {
        verification_list(&self.state).await
    }

    /// Typed `matrix_verification_accept`.
    pub async fn verification_accept(
        &self,
        request: MatrixVerificationAcceptRequest,
    ) -> Result<NativeVerificationRequest, MatrixIpcError> {
        verification_accept(&self.state, request).await
    }

    /// Typed `matrix_verification_begin_sas`.
    pub async fn verification_begin_sas(
        &self,
        request: MatrixVerificationBeginSasRequest,
    ) -> Result<NativeVerificationRequest, MatrixIpcError> {
        verification_begin_sas(&self.state, request).await
    }

    /// Typed `matrix_verification_cancel`.
    pub async fn verification_cancel(
        &self,
        request: MatrixVerificationCancelRequest,
    ) -> Result<NativeVerificationRequest, MatrixIpcError> {
        verification_cancel(&self.state, request).await
    }

    /// Typed `matrix_verification_confirm`.
    pub async fn verification_confirm(
        &self,
        request: MatrixVerificationConfirmRequest,
    ) -> Result<NativeVerificationRequest, MatrixIpcError> {
        verification_confirm(&self.state, request).await
    }

    /// Typed `matrix_verification_dismiss`.
    pub async fn verification_dismiss(
        &self,
        request: MatrixVerificationDismissRequest,
    ) -> Result<(), MatrixIpcError> {
        verification_dismiss(&self.state, request).await
    }

    /// Typed `matrix_verification_mismatch`.
    pub async fn verification_mismatch(
        &self,
        request: MatrixVerificationMismatchRequest,
    ) -> Result<NativeVerificationRequest, MatrixIpcError> {
        verification_mismatch(&self.state, request).await
    }

    /// Typed `matrix_verification_start`.
    pub async fn verification_start(
        &self,
        request: MatrixVerificationStartRequest,
    ) -> Result<NativeVerificationRequest, MatrixIpcError> {
        verification_start(&self.state, request).await
    }

    /// Typed `matrix_backup_status`.
    pub async fn backup_status(&self) -> Result<NativeBackupStatus, MatrixIpcError> {
        backup_status(&self.state).await
    }

    /// Typed `matrix_room_key_transfer_status`.
    pub async fn room_key_transfer_status(
        &self,
    ) -> Result<NativeRoomKeyTransferStatus, MatrixIpcError> {
        room_key_transfer_status(&self.state).await
    }

    /// Typed `matrix_cross_signing_setup`.
    pub async fn cross_signing_setup(
        &self,
    ) -> Result<NativeCrossSigningSetupResult, MatrixIpcError> {
        cross_signing_setup(&self.state).await
    }

    /// Typed `matrix_device_snapshot`.
    pub async fn device_snapshot(&self) -> Result<NativeDeviceSnapshot, MatrixIpcError> {
        device_snapshot(&self.state).await
    }

    /// Typed `matrix_device_rename`.
    pub async fn device_rename(
        &self,
        request: MatrixDeviceRenameRequest,
    ) -> Result<NativeDeviceSnapshot, MatrixIpcError> {
        device_rename(&self.state, request).await
    }

    /// Typed `matrix_device_delete_start`.
    pub async fn device_delete_start(
        &self,
        request: MatrixDeviceDeleteStartRequest,
    ) -> Result<NativeDeviceDeleteResult, MatrixIpcError> {
        device_delete_start(&self.state, request).await
    }

    /// Typed `matrix_device_delete_cancel`.
    pub async fn device_delete_cancel(
        &self,
        request: MatrixDeviceDeleteCancelRequest,
    ) -> Result<(), MatrixIpcError> {
        device_delete_cancel(&self.state, request).await
    }

    /// Typed `matrix_cross_signing_status`.
    pub async fn cross_signing_status(
        &self,
    ) -> Result<MatrixCrossSigningStatusResponse, MatrixIpcError> {
        cross_signing_status(&self.state).await
    }

    /// Typed `matrix_secret_storage_status`.
    pub async fn secret_storage_status(
        &self,
    ) -> Result<MatrixSecretStorageStatusResponse, MatrixIpcError> {
        secret_storage_status(&self.state).await
    }

    /// Typed `matrix_login_flows`.
    pub async fn login_flows(
        &self,
        request: MatrixLoginFlowsRequest,
    ) -> Result<MatrixLoginFlowsResponse, MatrixIpcError> {
        login_flows(&self.state, request).await
    }

    /// Typed `matrix_register_flows`.
    pub async fn register_flows(
        &self,
        request: MatrixRegisterFlowsRequest,
    ) -> Result<RegisterFlowsProbe, MatrixIpcError> {
        register_flows(&self.state, request).await
    }

    /// Typed `matrix_timeline_close`.
    pub async fn timeline_close(
        &self,
        request: MatrixTimelineCloseRequest,
    ) -> Result<bool, MatrixIpcError> {
        timeline_close(&self.state, request).await
    }

    /// Typed `matrix_timeline_retry_decryption`.
    pub async fn timeline_retry_decryption(
        &self,
        request: MatrixTimelineSnapshotRequest,
    ) -> Result<bool, MatrixIpcError> {
        timeline_retry_decryption(&self.state, request).await
    }

    /// Typed `matrix_agent_approvals_list`.
    pub async fn agent_approvals_list(
        &self,
        request: MatrixAgentApprovalsListRequest,
    ) -> Result<crate::app::timeline::NativeAgentApprovalInboxSnapshot, MatrixIpcError> {
        agent_approvals_list(&self.state, request).await
    }

    /// Typed `matrix_room_create`.
    pub async fn room_create(
        &self,
        request: MatrixRoomCreateRequest,
    ) -> Result<String, MatrixIpcError> {
        room_create(&self.state, request).await
    }

    /// Typed `matrix_set_user_image_pack`.
    pub async fn set_user_image_pack(
        &self,
        request: MatrixSetImagePackContentRequest,
    ) -> Result<MatrixStatusOk, MatrixIpcError> {
        set_user_image_pack(&self.state, request).await
    }

    /// Typed `matrix_set_global_image_packs`.
    pub async fn set_global_image_packs(
        &self,
        request: MatrixSetImagePackContentRequest,
    ) -> Result<MatrixStatusOk, MatrixIpcError> {
        set_global_image_packs(&self.state, request).await
    }

    /// Typed `matrix_set_room_image_pack`.
    pub async fn set_room_image_pack(
        &self,
        request: MatrixSetRoomImagePackRequest,
    ) -> Result<MatrixStatusOk, MatrixIpcError> {
        set_room_image_pack(&self.state, request).await
    }

    /// Typed `matrix_local_echo_discard`.
    pub async fn local_echo_discard(
        &self,
        request: MatrixLocalEchoRequest,
    ) -> Result<MatrixLocalEchoDiscardResult, MatrixIpcError> {
        local_echo_discard(&self.state, request).await
    }

    /// Typed `matrix_notification_focus_set`.
    pub async fn notification_focus_set(
        &self,
        request: NativeNotificationFocusSetRequest,
    ) -> Result<MatrixNotificationFocusSetResult, MatrixIpcError> {
        notification_focus_set(&self.state, request).await
    }

    /// Typed `matrix_set_encrypted_state_events_setting`.
    pub fn set_encrypted_state_events_setting(
        &self,
        request: MatrixSetEncryptedStateEventsSettingRequest,
    ) -> MatrixEncryptedStateEventsSettingResult {
        set_encrypted_state_events_setting(request)
    }
}
