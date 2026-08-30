use std::path::Path;
use std::process::Command;

const DESKTOP_COMMANDS: &[&str] = &[
    "desktop_show",
    "desktop_hide",
    "desktop_navigate",
    "desktop_set_badge_count",
    "desktop_set_shortcuts",
    "desktop_secret_store_status",
    "desktop_get_integration_status",
    "desktop_update_tray_state",
    "desktop_get_notification_permission",
    "desktop_request_notification_permission",
    "desktop_notify",
    "desktop_open_external_url",
    "desktop_save_file",
    "desktop_save_file_begin",
    "desktop_save_file_chunk",
    "desktop_save_file_end",
    "desktop_save_file_abort",
    "desktop_read_dropped_files",
    "desktop_read_dropped_file_chunk",
    "desktop_read_dropped_file_end",
    "desktop_get_performance_capabilities",
    "desktop_append_log",
    "desktop_log_path",
    "desktop_record_diagnostic",
    "desktop_diagnostics_status",
    "desktop_read_diagnostics",
    "desktop_clear_diagnostics",
    "desktop_agent_action",
    "matrix_login_flows",
    "matrix_store_recovery_prepare",
    "matrix_threepid_add_email",
    "matrix_threepid_add_email_password",
    "matrix_threepid_delete",
    "matrix_threepid_request_email_token",
    "matrix_threepid_snapshot",
    "matrix_store_recovery_confirm",
    "matrix_password_reset_request_email_token",
    "matrix_password_reset_complete",
    "matrix_register_flows",
    "matrix_register_request_email_token",
    "matrix_register",
    "matrix_session_identity",
    "matrix_cross_signing_status",
    "matrix_cross_signing_setup",
    "matrix_cross_signing_setup_password",
    "matrix_backup_status",
    "matrix_backup_setup",
    "matrix_backup_restore",
    "matrix_backup_repair",
    "matrix_secret_storage_status",
    "matrix_secret_storage_bootstrap",
    "matrix_secret_storage_unlock",
    "matrix_secret_storage_reset",
    "matrix_room_key_transfer_status",
    "matrix_room_key_export",
    "matrix_room_key_import_select",
    "matrix_room_key_import",
    "matrix_device_snapshot",
    "matrix_device_rename",
    "matrix_device_delete_start",
    "matrix_device_delete_password",
    "matrix_device_delete_cancel",
    "matrix_invites_snapshot",
    "matrix_invites_accept",
    "matrix_invites_decline",
    "matrix_room_create",
    "matrix_room_leave",
    "matrix_room_join",
    "matrix_room_set_favorite",
    "matrix_room_set_read_state",
    "matrix_room_invite",
    "matrix_room_kick",
    "matrix_room_ban",
    "matrix_room_unban",
    "matrix_room_set_power_level",
    "matrix_room_set_power_levels",
    "matrix_room_set_power_level_tags",
    "matrix_room_members_snapshot",
    "matrix_room_power_levels_snapshot",
    "matrix_room_creators_snapshot",
    "matrix_room_power_level_tags_snapshot",
    "matrix_room_directory_protocols",
    "matrix_room_directory_search",
    "matrix_room_directory_cancel",
    "matrix_invites_report_spam",
    "matrix_invites_block_sender",
    "matrix_typing_snapshot",
    "matrix_typing_set",
    "matrix_presence_set",
    "matrix_presence_snapshot",
    "matrix_presence_subscribe",
    "matrix_presence_unsubscribe",
    "matrix_push_rules_add_keyword",
    "matrix_push_rules_remove_keyword",
    "matrix_push_rules_set_default",
    "matrix_push_rules_set_mention",
    "matrix_push_rules_snapshot",
    "matrix_room_notification_set",
    "matrix_room_notification_snapshot",
    "matrix_room_notifications_snapshot",
    "matrix_timeline_reaction_toggle",
    "matrix_reaction_ensure",
    "matrix_agent_approval_decide",
    "matrix_reaction_redact",
    "matrix_space_parents_snapshot",
    "matrix_space_hierarchy_snapshot",
    "matrix_space_children_snapshot",
    "matrix_space_child_set",
    "matrix_space_child_remove",
    "matrix_restricted_join_reparent",
    "matrix_mdirect_snapshot",
    "matrix_mdirect_add",
    "matrix_mdirect_remove",
    "matrix_get_user_image_pack",
    "matrix_set_user_image_pack",
    "matrix_set_global_image_packs",
    "matrix_set_room_image_pack",
    "matrix_upload_media",
    "matrix_media_config",
    "matrix_media_download",
    "matrix_set_own_avatar",
    "matrix_set_own_display_name",
    "matrix_get_own_profile",
    "matrix_ignored_users_ignore",
    "matrix_ignored_users_snapshot",
    "matrix_ignored_users_unignore",
    "matrix_user_directory_search",
    "matrix_message_search",
    "matrix_set_room_name",
    "matrix_set_room_topic",
    "matrix_set_room_avatar",
    "matrix_get_room_directory_visibility",
    "matrix_set_room_directory_visibility",
    "matrix_room_join_rule_snapshot",
    "matrix_room_set_join_rule",
    "matrix_get_room_image_packs",
    "matrix_get_global_image_packs",
    "matrix_later_snapshot",
    "matrix_later_upsert",
    "matrix_later_complete",
    "matrix_later_snooze",
    "matrix_later_clear_completed",
    "matrix_later_mark_reminded",
    "matrix_room_notes_snapshot",
    "matrix_room_notes_upsert",
    "matrix_room_notes_delete",
    "matrix_room_notes_complete_todo",
    "matrix_room_notes_move_todo",
    "matrix_send_text",
    "matrix_send_attachment",
    "matrix_send_poll",
    "matrix_edit_message",
    "matrix_poll_respond",
    "matrix_timeline_set_read_state",
    "matrix_timeline_close",
    "matrix_timeline_event_readback",
    "matrix_timeline_jump_latest",
    "matrix_timeline_edit_text",
    "matrix_timeline_redact",
    "matrix_timeline_forward_text",
    "matrix_timeline_forward_media",
    "matrix_timeline_report",
    "matrix_timeline_pin",
    "matrix_timeline_unpin",
    "matrix_timeline_poll_vote",
    "matrix_timeline_call_decline",
    "matrix_composer_set_reply_draft",
    "matrix_composer_clear_reply_draft",
    "matrix_composer_get_reply_draft",
];

fn git_output(repo: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn sync_release_hardening_capability(manifest_dir: &Path) {
    let path = manifest_dir.join("capabilities/release-hardening.json");
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());

    println!("cargo:rerun-if-env-changed=PROFILE");

    if profile == "release" {
        let content = r#"{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "release-hardening",
  "description": "Release-only hardening: deny DevTools IPC toggle on the main webview.",
  "windows": ["main"],
  "permissions": [
    "core:webview:deny-internal-toggle-devtools"
  ]
}
"#;
        std::fs::write(&path, content).expect("write release-hardening capability");
        println!("cargo:rerun-if-changed={}", path.display());
    } else if path.exists() {
        std::fs::remove_file(&path).expect("remove release-hardening capability");
    }
}

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set");
    let manifest_path = Path::new(&manifest_dir);
    sync_release_hardening_capability(manifest_path);

    let repo = manifest_path.parent().unwrap_or(Path::new(&manifest_dir));
    let head_path = repo.join(".git/HEAD");
    let revision = git_output(repo, &["rev-parse", "--short=12", "HEAD"])
        .unwrap_or_else(|| "unknown".to_owned());
    let branch =
        git_output(repo, &["branch", "--show-current"]).unwrap_or_else(|| "unknown".to_owned());

    println!("cargo:rerun-if-changed={}", head_path.display());
    if let Ok(head) = std::fs::read_to_string(&head_path) {
        if let Some(reference) = head.strip_prefix("ref: ").map(str::trim) {
            println!(
                "cargo:rerun-if-changed={}",
                repo.join(".git").join(reference).display()
            );
        }
    }
    println!("cargo:rustc-env=SYNARA_DESKTOP_BUILD_REVISION={revision}");
    println!("cargo:rustc-env=SYNARA_DESKTOP_BUILD_BRANCH={branch}");

    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(DESKTOP_COMMANDS)),
    )
    .expect("failed to run Tauri build script")
}
