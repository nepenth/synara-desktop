//! Authoritative React-facing Matrix command census (P2).
//!
//! This list is deliberately a transport-only, Tauri-free snapshot of the
//! desktop invoke surface. Command-group migrations may register handlers only
//! for these exact names; an entry without a handler remains fail-closed.

/// Exact `matrix_*` commands exposed by the desktop Tauri invoke handler.
///
/// Keep this lexically sorted. The parity test below compares it with the
/// actual desktop invoke list, making surface drift explicit in CI.
pub const REACT_MATRIX_COMMAND_CENSUS: &[&str] = &[
    "matrix_agent_approval_decide",
    "matrix_backup_repair",
    "matrix_backup_restore",
    "matrix_backup_setup",
    "matrix_backup_status",
    "matrix_composer_clear_reply_draft",
    "matrix_composer_get_reply_draft",
    "matrix_composer_set_reply_draft",
    "matrix_cross_signing_setup",
    "matrix_cross_signing_setup_password",
    "matrix_cross_signing_status",
    "matrix_crypto_status",
    "matrix_device_delete_cancel",
    "matrix_device_delete_password",
    "matrix_device_delete_start",
    "matrix_device_rename",
    "matrix_device_snapshot",
    "matrix_edit_message",
    "matrix_get_global_image_packs",
    "matrix_get_own_profile",
    "matrix_get_room_directory_visibility",
    "matrix_get_room_image_packs",
    "matrix_get_user_image_pack",
    "matrix_ignored_users_ignore",
    "matrix_ignored_users_snapshot",
    "matrix_ignored_users_unignore",
    "matrix_invites_accept",
    "matrix_invites_block_sender",
    "matrix_invites_decline",
    "matrix_invites_report_spam",
    "matrix_invites_snapshot",
    "matrix_later_clear_completed",
    "matrix_later_complete",
    "matrix_later_mark_reminded",
    "matrix_later_snapshot",
    "matrix_later_snooze",
    "matrix_later_upsert",
    "matrix_login_flows",
    "matrix_login_password",
    "matrix_logout",
    "matrix_mdirect_add",
    "matrix_mdirect_remove",
    "matrix_mdirect_snapshot",
    "matrix_media_config",
    "matrix_media_download",
    "matrix_message_search",
    "matrix_notification_decide",
    "matrix_notification_dismiss",
    "matrix_notification_focus_set",
    "matrix_notification_pending_snapshot",
    "matrix_password_reset_complete",
    "matrix_password_reset_request_email_token",
    "matrix_poll_respond",
    "matrix_presence_set",
    "matrix_presence_snapshot",
    "matrix_presence_subscribe",
    "matrix_presence_unsubscribe",
    "matrix_push_rules_add_keyword",
    "matrix_push_rules_remove_keyword",
    "matrix_push_rules_set_default",
    "matrix_push_rules_set_mention",
    "matrix_push_rules_snapshot",
    "matrix_reaction_ensure",
    "matrix_reaction_redact",
    "matrix_register",
    "matrix_register_flows",
    "matrix_register_request_email_token",
    "matrix_restore_session",
    "matrix_restricted_join_reparent",
    "matrix_room_ban",
    "matrix_room_create",
    "matrix_room_creators_snapshot",
    "matrix_room_directory_cancel",
    "matrix_room_directory_protocols",
    "matrix_room_directory_search",
    "matrix_room_invite",
    "matrix_room_join",
    "matrix_room_join_rule_snapshot",
    "matrix_room_key_export",
    "matrix_room_key_import",
    "matrix_room_key_import_select",
    "matrix_room_key_transfer_status",
    "matrix_room_kick",
    "matrix_room_leave",
    "matrix_room_list_snapshot",
    "matrix_room_members_snapshot",
    "matrix_room_notes_complete_todo",
    "matrix_room_notes_delete",
    "matrix_room_notes_move_todo",
    "matrix_room_notes_snapshot",
    "matrix_room_notes_upsert",
    "matrix_room_notification_set",
    "matrix_room_notification_snapshot",
    "matrix_room_notifications_snapshot",
    "matrix_room_power_level_tags_snapshot",
    "matrix_room_power_levels_snapshot",
    "matrix_room_set_favorite",
    "matrix_room_set_join_rule",
    "matrix_room_set_power_level",
    "matrix_room_set_power_level_tags",
    "matrix_room_set_power_levels",
    "matrix_room_set_read_state",
    "matrix_room_unban",
    "matrix_secret_storage_bootstrap",
    "matrix_secret_storage_reset",
    "matrix_secret_storage_status",
    "matrix_secret_storage_unlock",
    "matrix_send_attachment",
    "matrix_send_poll",
    "matrix_send_text",
    "matrix_session_identity",
    "matrix_session_snapshot",
    "matrix_set_global_image_packs",
    "matrix_set_own_avatar",
    "matrix_set_own_display_name",
    "matrix_set_room_avatar",
    "matrix_set_room_directory_visibility",
    "matrix_set_room_image_pack",
    "matrix_set_room_name",
    "matrix_set_room_topic",
    "matrix_set_user_image_pack",
    "matrix_space_child_remove",
    "matrix_space_child_set",
    "matrix_space_children_snapshot",
    "matrix_space_hierarchy_snapshot",
    "matrix_space_parents_snapshot",
    "matrix_store_recovery_confirm",
    "matrix_store_recovery_prepare",
    "matrix_sync_status",
    "matrix_threepid_add_email",
    "matrix_threepid_add_email_password",
    "matrix_threepid_delete",
    "matrix_threepid_request_email_token",
    "matrix_threepid_snapshot",
    "matrix_timeline_call_decline",
    "matrix_timeline_close",
    "matrix_timeline_edit_text",
    "matrix_timeline_event_readback",
    "matrix_timeline_forward_media",
    "matrix_timeline_forward_text",
    "matrix_timeline_jump_latest",
    "matrix_timeline_open",
    "matrix_timeline_paginate",
    "matrix_timeline_pin",
    "matrix_timeline_poll_vote",
    "matrix_timeline_reaction_toggle",
    "matrix_timeline_redact",
    "matrix_timeline_report",
    "matrix_timeline_set_read_state",
    "matrix_timeline_snapshot",
    "matrix_timeline_unpin",
    "matrix_typing_set",
    "matrix_typing_snapshot",
    "matrix_upload_media",
    "matrix_user_directory_search",
    "matrix_verification_accept",
    "matrix_verification_begin_sas",
    "matrix_verification_cancel",
    "matrix_verification_confirm",
    "matrix_verification_dismiss",
    "matrix_verification_list",
    "matrix_verification_mismatch",
    "matrix_verification_start",
];

/// Whether `command` is an exact React-facing Matrix invoke command.
pub fn is_known_matrix_command(command: &str) -> bool {
    REACT_MATRIX_COMMAND_CENSUS.binary_search(&command).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DESKTOP_LIB_RS: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../src-tauri/src/lib.rs"
    ));

    fn desktop_matrix_commands() -> Vec<&'static str> {
        let marker = ".invoke_handler(tauri::generate_handler![";
        let start = DESKTOP_LIB_RS
            .find(marker)
            .expect("desktop invoke handler marker must remain present")
            + marker.len();
        let end = DESKTOP_LIB_RS[start..]
            .find("])")
            .map(|offset| start + offset)
            .expect("desktop invoke handler must close");

        DESKTOP_LIB_RS[start..end]
            .split(',')
            .filter_map(|entry| {
                let command = entry.trim().rsplit("::").next()?;
                command.starts_with("matrix_").then_some(command)
            })
            .collect()
    }

    #[test]
    fn census_is_unique_and_lexically_sorted() {
        assert!(
            REACT_MATRIX_COMMAND_CENSUS
                .windows(2)
                .all(|pair| pair[0] < pair[1]),
            "command census must be unique and lexically sorted"
        );
    }

    #[test]
    fn census_matches_actual_desktop_matrix_invoke_list() {
        let mut actual = desktop_matrix_commands();
        actual.sort_unstable();
        assert_eq!(actual, REACT_MATRIX_COMMAND_CENSUS);
    }
}
