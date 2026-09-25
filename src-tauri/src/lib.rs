#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
// Desktop `cargo clippy --all-targets` overflows rustc's default query-depth
// limit while laying out `{async fn body of matrix_send_attachment}` (and peers).
// NSE/Core already pin 256; keep-both dropped this crate-level attribute.
#![recursion_limit = "256"]

mod bridge;
mod build_info;
mod desktop;
mod desktop_agent_actions;
mod desktop_file_transfer;
mod desktop_integration;
mod desktop_logging;
mod desktop_navigation;
mod desktop_notification_sound;
mod desktop_notifications;
mod desktop_platform;
mod desktop_sanitize;
mod desktop_secret_store;
mod desktop_shortcuts;
mod desktop_spellcheck;
mod desktop_tray;
mod desktop_unread_badge;
mod desktop_url;
mod desktop_webview_performance;
// P1.2: compile-only Matrix Rust SDK linkage; no production client session.
mod matrix_sdk_link_smoke;
// P1.3: Matrix IPC schema foundation (types/helpers only; no production commands).
mod matrix;
#[cfg(target_os = "macos")]
mod menu;

use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use synara_core::platform::Platform;
use tauri::{
    webview::{NewWindowResponse, WebviewWindowBuilder},
    DragDropEvent, Emitter, LogicalSize, Manager, Size, WebviewUrl, WindowEvent,
};
use tauri_plugin_opener::OpenerExt;

const SYNARA_MEDIA_PROTOCOL: &str = "synara-media";
const INVITE_AVATAR_MAX_BYTES: usize = 1_048_576;
const TIMELINE_MEDIA_MAX_BYTES: usize = 64 * 1_048_576;

fn synara_media_response(
    status: tauri::http::StatusCode,
    body: Vec<u8>,
    content_type: Option<&str>,
) -> tauri::http::Response<Vec<u8>> {
    let mut builder = tauri::http::Response::builder()
        .status(status)
        .header(tauri::http::header::CACHE_CONTROL, "no-store")
        .header(tauri::http::header::X_CONTENT_TYPE_OPTIONS, "nosniff");
    if let Some(content_type) = content_type {
        builder = builder.header(tauri::http::header::CONTENT_TYPE, content_type);
    }
    builder
        .body(body)
        .expect("fixed invite avatar response headers are valid")
}

fn image_content_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" && &bytes[8..12] == b"avif" {
        Some("image/avif")
    } else {
        None
    }
}

fn timeline_media_content_type(bytes: &[u8], declared: Option<&str>) -> Option<&'static str> {
    let candidates: &[&'static str] = if let Some(image) = image_content_type(bytes) {
        &[image]
    } else if bytes.starts_with(b"%PDF-") {
        &["application/pdf"]
    } else if bytes.starts_with(b"ID3")
        || matches!(bytes, [0xff, second, ..] if second & 0xe0 == 0xe0)
    {
        &["audio/mpeg"]
    } else if bytes.starts_with(b"fLaC") {
        &["audio/flac"]
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        &["audio/wav"]
    } else if bytes.starts_with(b"OggS") {
        &["audio/ogg", "video/ogg", "application/ogg"]
    } else if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        &["video/mp4", "audio/mp4"]
    } else if bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]) {
        &["video/webm", "audio/webm"]
    } else {
        return None;
    };
    let declared = declared.map(str::trim).filter(|value| !value.is_empty());
    match declared {
        None | Some("application/octet-stream") => candidates.first().copied(),
        Some("image/jpg") if candidates.contains(&"image/jpeg") => Some("image/jpeg"),
        Some(value) => candidates
            .iter()
            .copied()
            .find(|candidate| candidate.eq_ignore_ascii_case(value)),
    }
}

fn decode_synara_media_path(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).ok()?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    let decoded = String::from_utf8(decoded).ok()?;
    if decoded.is_empty()
        || decoded.contains(['\0', '\\'])
        || decoded.split('/').any(|segment| segment == "..")
    {
        return None;
    }
    Some(decoded)
}

fn register_synara_media_protocol<R: tauri::Runtime>(
    builder: tauri::Builder<R>,
) -> tauri::Builder<R> {
    builder.register_asynchronous_uri_scheme_protocol(
        SYNARA_MEDIA_PROTOCOL,
        |context, request, responder| {
            if context.webview_label() != desktop::MAIN_WINDOW_LABEL
                || request.method() != tauri::http::Method::GET
                || request.uri().query().is_some()
            {
                responder.respond(synara_media_response(
                    tauri::http::StatusCode::NOT_FOUND,
                    Vec::new(),
                    None,
                ));
                return;
            }
            let Some(handle) = request.uri().path().strip_prefix('/') else {
                responder.respond(synara_media_response(
                    tauri::http::StatusCode::NOT_FOUND,
                    Vec::new(),
                    None,
                ));
                return;
            };
            let app = context.app_handle().clone();
            let handle = handle.to_owned();
            tauri::async_runtime::spawn(async move {
                let Some(handle) = decode_synara_media_path(&handle) else {
                    responder.respond(synara_media_response(
                        tauri::http::StatusCode::NOT_FOUND,
                        Vec::new(),
                        None,
                    ));
                    return;
                };
                let state = app.state::<matrix::auth::MatrixAuthState>();
                if matrix::timeline::is_timeline_media_handle(&handle) {
                    let Some((client, source)) = state.resolve_timeline_media(&handle).await else {
                        responder.respond(synara_media_response(
                            tauri::http::StatusCode::NOT_FOUND,
                            Vec::new(),
                            None,
                        ));
                        return;
                    };
                    let request = matrix_sdk::media::MediaRequestParameters {
                        source: source.source,
                        format: matrix_sdk::media::MediaFormat::File,
                    };
                    let Ok(bytes) = synara_core::app::media::download_media_bounded(
                        &client,
                        &request,
                        TIMELINE_MEDIA_MAX_BYTES,
                    )
                    .await
                    else {
                        responder.respond(synara_media_response(
                            tauri::http::StatusCode::NOT_FOUND,
                            Vec::new(),
                            None,
                        ));
                        return;
                    };
                    let Some(content_type) =
                        timeline_media_content_type(&bytes, source.declared_mime_type.as_deref())
                    else {
                        responder.respond(synara_media_response(
                            tauri::http::StatusCode::UNSUPPORTED_MEDIA_TYPE,
                            Vec::new(),
                            None,
                        ));
                        return;
                    };
                    responder.respond(synara_media_response(
                        tauri::http::StatusCode::OK,
                        bytes,
                        Some(content_type),
                    ));
                    return;
                }
                if let Ok(content_uri) = matrix::auth::product::parse_media_download_uri(&handle) {
                    let Some(client) = state.media_client().await else {
                        responder.respond(synara_media_response(
                            tauri::http::StatusCode::NOT_FOUND,
                            Vec::new(),
                            None,
                        ));
                        return;
                    };
                    let request = matrix_sdk::media::MediaRequestParameters {
                        source: matrix_sdk::ruma::events::room::MediaSource::Plain(content_uri),
                        format: matrix_sdk::media::MediaFormat::File,
                    };
                    let Ok(bytes) = synara_core::app::media::download_media_bounded(
                        &client,
                        &request,
                        TIMELINE_MEDIA_MAX_BYTES,
                    )
                    .await
                    else {
                        responder.respond(synara_media_response(
                            tauri::http::StatusCode::NOT_FOUND,
                            Vec::new(),
                            None,
                        ));
                        return;
                    };
                    let Some(content_type) = timeline_media_content_type(&bytes, None) else {
                        responder.respond(synara_media_response(
                            tauri::http::StatusCode::UNSUPPORTED_MEDIA_TYPE,
                            Vec::new(),
                            None,
                        ));
                        return;
                    };
                    responder.respond(synara_media_response(
                        tauri::http::StatusCode::OK,
                        bytes,
                        Some(content_type),
                    ));
                    return;
                }
                if handle.len() != 64 || !handle.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    responder.respond(synara_media_response(
                        tauri::http::StatusCode::NOT_FOUND,
                        Vec::new(),
                        None,
                    ));
                    return;
                }
                let Some((client, source)) = state.resolve_invite_avatar(&handle).await else {
                    responder.respond(synara_media_response(
                        tauri::http::StatusCode::NOT_FOUND,
                        Vec::new(),
                        None,
                    ));
                    return;
                };
                let request = matrix_sdk::media::MediaRequestParameters {
                    source: matrix_sdk::ruma::events::room::MediaSource::Plain(source.mxc_uri),
                    format: matrix_sdk::media::MediaFormat::Thumbnail(
                        matrix_sdk::media::MediaThumbnailSettings::new(
                            matrix_sdk::ruma::UInt::from(96_u8),
                            matrix_sdk::ruma::UInt::from(96_u8),
                        ),
                    ),
                };
                let Ok(bytes) = synara_core::app::media::download_media_bounded(
                    &client,
                    &request,
                    INVITE_AVATAR_MAX_BYTES,
                )
                .await
                else {
                    responder.respond(synara_media_response(
                        tauri::http::StatusCode::NOT_FOUND,
                        Vec::new(),
                        None,
                    ));
                    return;
                };
                let Some(content_type) = image_content_type(&bytes) else {
                    responder.respond(synara_media_response(
                        tauri::http::StatusCode::UNSUPPORTED_MEDIA_TYPE,
                        Vec::new(),
                        None,
                    ));
                    return;
                };
                responder.respond(synara_media_response(
                    tauri::http::StatusCode::OK,
                    bytes,
                    Some(content_type),
                ));
            });
        },
    )
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeFileDropPayload {
    phase: &'static str,
    paths: Vec<String>,
    x: f64,
    y: f64,
}

fn native_drop_paths(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

fn emit_native_file_drop(window: &tauri::Window, payload: NativeFileDropPayload) {
    let Some(webview) = window.get_webview_window(desktop::MAIN_WINDOW_LABEL) else {
        return;
    };
    let _ = webview.emit("synara-native-file-drop", payload.clone());
    let Ok(payload_json) = serde_json::to_string(&payload) else {
        return;
    };
    let script = format!(
        "window.dispatchEvent(new CustomEvent('synara-native-file-drop', {{ detail: {} }}));",
        payload_json
    );
    let _ = webview.eval(script);
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn updater_plugin_configured<R: tauri::Runtime>(context: &tauri::Context<R>) -> bool {
    context
        .config()
        .plugins
        .0
        .get("updater")
        .is_some_and(|config| !config.is_null())
}

pub fn run() {
    let context = tauri::generate_context!();
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let updater_configured = updater_plugin_configured(&context);
    #[cfg(any(target_os = "android", target_os = "ios"))]
    let updater_configured = false;
    let mut builder = register_synara_media_protocol(tauri::Builder::default());
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Err(error) = desktop::show_main_window(app) {
                eprintln!("[synara] failed to focus the running window: {error}");
            }
        }));
    }
    builder = builder
        .manage(matrix::auth::MatrixAuthState::new())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            desktop::desktop_show,
            desktop::desktop_hide,
            desktop::desktop_window_minimize,
            desktop::desktop_window_toggle_maximize,
            desktop::desktop_window_close,
            desktop::desktop_navigate,
            desktop::desktop_set_badge_count,
            desktop::desktop_play_notification_sound,
            desktop::desktop_set_shortcuts,
            desktop::desktop_secret_store_status,
            desktop_integration::desktop_get_integration_status,
            desktop::desktop_update_tray_state,
            desktop_notifications::desktop_get_notification_permission,
            desktop_notifications::desktop_request_notification_permission,
            desktop_notifications::desktop_notify,
            desktop_notifications::desktop_dismiss_notifications,
            desktop::desktop_open_external_url,
            desktop_file_transfer::desktop_save_file,
            desktop_file_transfer::desktop_save_file_begin,
            desktop_file_transfer::desktop_save_file_chunk,
            desktop_file_transfer::desktop_save_file_end,
            desktop_file_transfer::desktop_save_file_abort,
            desktop_file_transfer::desktop_read_dropped_files,
            desktop_file_transfer::desktop_read_dropped_file_chunk,
            desktop_file_transfer::desktop_read_dropped_file_end,
            desktop::desktop_get_performance_capabilities,
            desktop_logging::desktop_append_log,
            desktop_logging::desktop_log_path,
            desktop_logging::desktop_record_diagnostic,
            desktop_logging::desktop_diagnostics_status,
            desktop_logging::desktop_read_diagnostics,
            desktop_logging::desktop_clear_diagnostics,
            desktop_spellcheck::desktop_enable_spellcheck,
            desktop_agent_actions::desktop_agent_action,
            matrix::auth::product::matrix_login_password,
            matrix::auth::product::matrix_store_recovery_prepare,
            matrix::auth::product::matrix_store_recovery_confirm,
            matrix::auth::product::matrix_login_flows,
            matrix::auth::product::matrix_password_reset_request_email_token,
            matrix::auth::product::matrix_password_reset_complete,
            matrix::auth::product::matrix_register_flows,
            matrix::auth::product::matrix_register_request_email_token,
            matrix::auth::product::matrix_register,
            matrix::auth::product::matrix_session_identity,
            matrix::auth::product::matrix_session_snapshot,
            matrix::auth::product::matrix_sync_status,
            matrix::auth::product::matrix_sync_recover,
            matrix::auth::product::matrix_crypto_status,
            matrix::auth::product::matrix_cross_signing_status,
            matrix::auth::product::matrix_cross_signing_setup,
            matrix::auth::product::matrix_cross_signing_setup_password,
            matrix::auth::product::matrix_backup_status,
            matrix::auth::product::matrix_backup_setup,
            matrix::auth::product::matrix_backup_restore,
            matrix::auth::product::matrix_backup_repair,
            matrix::auth::product::matrix_secret_storage_status,
            matrix::auth::product::matrix_secret_storage_bootstrap,
            matrix::auth::product::matrix_secret_storage_unlock,
            matrix::auth::product::matrix_secret_storage_reset,
            matrix::auth::product::matrix_room_key_transfer_status,
            matrix::auth::product::matrix_room_key_export,
            matrix::auth::product::matrix_room_key_import_select,
            matrix::auth::product::matrix_room_key_import,
            matrix::auth::product::matrix_verification_list,
            matrix::auth::product::matrix_verification_start,
            matrix::auth::product::matrix_verification_accept,
            matrix::auth::product::matrix_verification_begin_sas,
            matrix::auth::product::matrix_verification_confirm,
            matrix::auth::product::matrix_verification_mismatch,
            matrix::auth::product::matrix_verification_cancel,
            matrix::auth::product::matrix_verification_dismiss,
            matrix::auth::product::matrix_device_snapshot,
            matrix::auth::product::matrix_device_rename,
            matrix::auth::product::matrix_device_delete_start,
            matrix::auth::product::matrix_device_delete_password,
            matrix::auth::product::matrix_device_delete_cancel,
            matrix::auth::product::matrix_x509_identity_status,
            matrix::auth::product::matrix_x509_identity_set_enabled,
            matrix::auth::product::matrix_x509_identity_import_ca,
            matrix::auth::product::matrix_x509_identity_remove_ca,
            matrix::auth::product::matrix_x509_identity_import_signer,
            matrix::auth::product::matrix_room_list_snapshot,
            matrix::auth::product::matrix_room_members_snapshot,
            matrix::auth::product::matrix_room_power_levels_snapshot,
            matrix::auth::product::matrix_room_retention,
            matrix::auth::product::matrix_room_creators_snapshot,
            matrix::auth::product::matrix_room_power_level_tags_snapshot,
            matrix::auth::product::matrix_room_directory_protocols,
            matrix::auth::product::matrix_room_directory_search,
            matrix::auth::product::matrix_room_directory_cancel,
            matrix::auth::product::matrix_inbox_notifications,
            matrix::auth::product::matrix_invites_snapshot,
            matrix::auth::product::matrix_invites_accept,
            matrix::auth::product::matrix_invites_decline,
            matrix::auth::product::matrix_room_create,
            matrix::auth::product::matrix_room_leave,
            matrix::auth::product::matrix_room_join,
            matrix::auth::product::matrix_room_set_favorite,
            matrix::auth::product::matrix_room_set_read_state,
            matrix::auth::product::matrix_room_invite,
            matrix::auth::product::matrix_room_kick,
            matrix::auth::product::matrix_room_ban,
            matrix::auth::product::matrix_room_unban,
            matrix::auth::product::matrix_room_set_power_level,
            matrix::auth::product::matrix_room_set_power_levels,
            matrix::auth::product::matrix_room_set_power_level_tags,
            matrix::auth::product::matrix_invites_report_spam,
            matrix::auth::product::matrix_invites_block_sender,
            matrix::auth::product::matrix_space_parents_snapshot,
            matrix::auth::product::matrix_space_hierarchy_snapshot,
            matrix::auth::product::matrix_space_children_snapshot,
            matrix::auth::product::matrix_space_child_set,
            matrix::auth::product::matrix_space_child_remove,
            matrix::auth::product::matrix_restricted_join_reparent,
            matrix::auth::product::matrix_mdirect_snapshot,
            matrix::auth::product::matrix_mdirect_add,
            matrix::auth::product::matrix_mdirect_remove,
            matrix::auth::product::matrix_get_user_image_pack,
            matrix::auth::product::matrix_get_room_image_packs,
            matrix::auth::product::matrix_get_global_image_packs,
            matrix::auth::product::matrix_set_user_image_pack,
            matrix::auth::product::matrix_set_global_image_packs,
            matrix::auth::product::matrix_set_room_image_pack,
            matrix::auth::product::matrix_media_config,
            matrix::auth::product::matrix_media_save,
            matrix::auth::product::matrix_media_text_preview,
            matrix::auth::product::matrix_media_preview,
            matrix::auth::product::matrix_later_snapshot,
            matrix::auth::product::matrix_later_upsert,
            matrix::auth::product::matrix_later_complete,
            matrix::auth::product::matrix_later_snooze,
            matrix::auth::product::matrix_later_clear_completed,
            matrix::auth::product::matrix_later_mark_reminded,
            matrix::auth::product::matrix_room_notes_snapshot,
            matrix::auth::product::matrix_room_notes_upsert,
            matrix::auth::product::matrix_room_notes_delete,
            matrix::auth::product::matrix_room_notes_complete_todo,
            matrix::auth::product::matrix_room_notes_move_todo,
            matrix::auth::product::matrix_typing_snapshot,
            matrix::auth::product::matrix_typing_set,
            matrix::auth::product::matrix_presence_set,
            matrix::auth::product::matrix_presence_snapshot,
            matrix::auth::product::matrix_presence_subscribe,
            matrix::auth::product::matrix_presence_unsubscribe,
            matrix::auth::product::matrix_rtc_transports_snapshot,
            matrix::auth::product::matrix_rtc_transports_refresh,
            matrix::auth::product::matrix_widgets_list,
            matrix::auth::product::matrix_widget_open,
            matrix::auth::product::matrix_widget_close,
            matrix::auth::product::matrix_widget_post,
            matrix::auth::product::matrix_widget_subscribe,
            matrix::auth::product::widget_bridge_post,
            matrix::auth::product::matrix_timeline_open,
            matrix::auth::product::matrix_timeline_close,
            matrix::auth::product::matrix_timeline_jump_latest,
            matrix::auth::product::matrix_timeline_paginate,
            matrix::auth::product::matrix_timeline_snapshot,
            matrix::auth::product::matrix_timeline_set_read_state,
            matrix::auth::product::matrix_timeline_event_readback,
            matrix::auth::product::matrix_timeline_timestamp_to_event,
            matrix::auth::product::matrix_timeline_follow_live,
            matrix::auth::product::matrix_timeline_reaction_toggle,
            matrix::auth::product::matrix_reaction_ensure,
            matrix::auth::product::matrix_agent_approval_decide,
            matrix::auth::product::matrix_agent_approval_history_snapshot,
            matrix::auth::product::matrix_agent_approvals_list,
            matrix::auth::product::matrix_reaction_redact,
            matrix::auth::product::matrix_timeline_edit_text,
            matrix::auth::product::matrix_timeline_redact,
            matrix::auth::product::matrix_timeline_forward_text,
            matrix::auth::product::matrix_timeline_report,
            matrix::auth::product::matrix_timeline_pin,
            matrix::auth::product::matrix_timeline_unpin,
            matrix::auth::product::matrix_pinned_events,
            matrix::auth::product::matrix_composer_set_reply_draft,
            matrix::auth::product::matrix_composer_clear_reply_draft,
            matrix::auth::product::matrix_composer_get_reply_draft,
            matrix::auth::product::matrix_thread_list,
            matrix::auth::product::matrix_timeline_forward_media,
            matrix::auth::product::matrix_timeline_poll_vote,
            matrix::auth::product::matrix_timeline_call_decline,
            matrix::auth::product::matrix_send_text,
            matrix::auth::product::matrix_edit_message,
            matrix::auth::product::matrix_send_attachment,
            matrix::auth::product::matrix_send_poll,
            matrix::auth::product::matrix_poll_respond,
            matrix::auth::product::matrix_set_own_display_name,
            matrix::auth::product::matrix_set_own_avatar,
            matrix::auth::product::matrix_get_own_profile,
            matrix::auth::product::matrix_ignored_users_snapshot,
            matrix::auth::product::matrix_ignored_users_ignore,
            matrix::auth::product::matrix_ignored_users_unignore,
            matrix::auth::product::matrix_user_directory_search,
            matrix::auth::product::matrix_user_status_clear,
            matrix::auth::product::matrix_user_status_set,
            matrix::auth::product::matrix_user_status_snapshot,
            matrix::auth::product::matrix_message_search,
            matrix::auth::product::matrix_notification_decide,
            matrix::auth::product::matrix_notification_dismiss,
            matrix::auth::product::matrix_notification_focus_set,
            matrix::auth::product::matrix_notification_pending_snapshot,
            matrix::auth::product::matrix_push_rules_snapshot,
            matrix::auth::product::matrix_push_rules_set_default,
            matrix::auth::product::matrix_push_rules_set_mention,
            matrix::auth::product::matrix_push_rules_add_keyword,
            matrix::auth::product::matrix_push_rules_remove_keyword,
            matrix::auth::product::matrix_room_notification_snapshot,
            matrix::auth::product::matrix_room_notification_set,
            matrix::auth::product::matrix_room_notifications_snapshot,
            matrix::auth::product::matrix_threepid_snapshot,
            matrix::auth::product::matrix_threepid_delete,
            matrix::auth::product::matrix_threepid_request_email_token,
            matrix::auth::product::matrix_threepid_add_email,
            matrix::auth::product::matrix_threepid_add_email_password,
            matrix::auth::product::matrix_set_room_name,
            matrix::auth::product::matrix_set_room_topic,
            matrix::auth::product::matrix_set_room_avatar,
            matrix::auth::product::matrix_send_state_event,
            matrix::auth::product::matrix_enable_room_encrypted_state,
            matrix::auth::product::matrix_set_encrypted_state_events_setting,
            matrix::auth::product::matrix_get_room_directory_visibility,
            matrix::auth::product::matrix_set_room_directory_visibility,
            matrix::auth::product::matrix_room_join_rule_snapshot,
            matrix::auth::product::matrix_room_set_join_rule,
            matrix::auth::product::matrix_upload_media,
            matrix::auth::product::matrix_logout,
            matrix::auth::product::matrix_restore_session
        ])
        .on_window_event(|window, event| {
            if let Some(session_id) =
                crate::matrix::widgets::host::session_id_from_widget_label(window.label())
            {
                if matches!(
                    event,
                    WindowEvent::Destroyed | WindowEvent::CloseRequested { .. }
                ) {
                    if let Some(core) = window.try_state::<Arc<synara_core::Core>>() {
                        let core = Arc::clone(core.inner());
                        let session_id = session_id.to_owned();
                        tauri::async_runtime::spawn(async move {
                            let _ = crate::bridge::widgets::widget_close(
                                core.as_ref(),
                                Some(session_id),
                            )
                            .await;
                        });
                    }
                }
                return;
            }

            if window.label() != desktop::MAIN_WINDOW_LABEL {
                return;
            }

            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = desktop::hide_main_window(window.app_handle());
                }
                WindowEvent::DragDrop(DragDropEvent::Enter { paths, position }) => {
                    desktop_file_transfer::reset_drag_drop_session();
                    emit_native_file_drop(
                        window,
                        NativeFileDropPayload {
                            phase: "enter",
                            paths: native_drop_paths(paths),
                            x: position.x,
                            y: position.y,
                        },
                    );
                }
                WindowEvent::DragDrop(DragDropEvent::Over { position }) => {
                    emit_native_file_drop(
                        window,
                        NativeFileDropPayload {
                            phase: "over",
                            paths: Vec::new(),
                            x: position.x,
                            y: position.y,
                        },
                    );
                }
                WindowEvent::DragDrop(DragDropEvent::Drop { paths, position }) => {
                    desktop_file_transfer::remember_dropped_paths(paths);
                    emit_native_file_drop(
                        window,
                        NativeFileDropPayload {
                            phase: "drop",
                            paths: native_drop_paths(paths),
                            x: position.x,
                            y: position.y,
                        },
                    );
                }
                WindowEvent::DragDrop(DragDropEvent::Leave) => {
                    desktop_file_transfer::clear_dropped_file_allowlist_on_drag_leave();
                    emit_native_file_drop(
                        window,
                        NativeFileDropPayload {
                            phase: "leave",
                            paths: Vec::new(),
                            x: 0.0,
                            y: 0.0,
                        },
                    );
                }
                _ => {}
            }
        });

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder.plugin(desktop_shortcuts::global_shortcut_plugin());
        builder = builder.plugin(tauri_plugin_process::init());

        if updater_configured {
            builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
        } else {
            eprintln!(
                "[synara] Tauri updater plugin is disabled because plugins.updater is not configured."
            );
        }
    }

    builder
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            {
                app.set_menu(menu::menu(app.handle())?)?;
                app.on_menu_event(menu::handle_menu_event);
            }

            desktop_tray::create_tray(app.handle())?;

            // WebviewUrl::App follows Tauri's dev cfg: http://localhost:8080 while
            // `tauri dev` is running, and the private asset origin tauri://localhost
            // for every packaged build. Navigation is pinned to that same origin.
            let window_url = WebviewUrl::App(Default::default());

            #[cfg(dev)]
            let navigation_url = app
                .config()
                .build
                .dev_url
                .clone()
                .ok_or("Desktop development requires a configured devUrl")?;
            #[cfg(not(dev))]
            let navigation_url = url::Url::parse(desktop_navigation::PACKAGED_ASSET_ORIGIN)
                .map_err(|error| format!("Invalid packaged asset origin: {error}"))?;

            let app_handle = app.handle().clone();
            let bridge_script = format!(
                "{}\nif (window.__SYNARA_DESKTOP__) {{ window.__SYNARA_DESKTOP__.supportsUpdater = {}; window.__SYNARA_DESKTOP__.supportsSecureSecretStore = {}; }}",
                include_str!("desktop_bridge.js"),
                updater_configured,
                desktop::desktop_bridge_supports_secure_secret_store()
            );
            let window_builder = WebviewWindowBuilder::new(app, "main".to_string(), window_url)
                .title("Synara")
                .inner_size(1280.0, 900.0)
                .min_inner_size(960.0, 720.0);
            // Keep the native macOS titlebar: the OS owns window movement and
            // traffic lights without requiring an asynchronous WebView drag.
            // Linux uses the in-app strip and its explicit window controls.
            #[cfg(target_os = "linux")]
            let window_builder = window_builder.decorations(false);
            let window = window_builder
                .initialization_script(bridge_script)
                .on_navigation(move |url| desktop_navigation::is_app_navigation(&navigation_url, url))
                .on_new_window(move |url, _features| {
                    if desktop::is_safe_external_url(url.as_str()) {
                        if let Err(error) = app_handle.opener().open_url(url.as_str(), None::<&str>)
                        {
                            eprintln!("[synara] Failed to open external URL: {error}");
                        }
                    }
                    NewWindowResponse::Deny
                })
                .build()?;

            if let Err(error) = desktop_spellcheck::configure_webview_spellcheck(&window) {
                eprintln!("[synara] {error}");
            }
            if let Err(error) = desktop_webview_performance::inspect(&window) {
                eprintln!("[synara] {error}");
            }

            if let Ok(size) = window.inner_size() {
                if size.width < 960 || size.height < 720 {
                    window.set_size(Size::Logical(LogicalSize::new(1280.0, 900.0)))?;
                    let _ = window.center();
                }
            }

            window.show()?;
            window.unminimize()?;
            window.set_focus()?;

            #[cfg(target_os = "macos")]
            desktop_notifications::initialize_macos_notifications(app.handle());

            // One desktop platform allocation is shared by the shell state and
            // the managed Core. P3.1 auth probes therefore retain this
            // platform's established HTTP user-agent injection without giving
            // Core a Tauri type or creating a fallback platform.
            let platform: Arc<dyn Platform> =
                Arc::new(desktop_platform::TauriPlatform::new(app.handle().clone()));
            app.manage(Arc::clone(&platform));
            app.manage(Arc::new(synara_core::Core::new(platform)));
            matrix::auth::product::spawn_suspend_resume_watch(app.handle().clone());
            Ok(())
        })
        .build(context)
        .expect("error while building tauri application")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                let _ = desktop::show_main_window(app);
            }

            #[cfg(not(target_os = "macos"))]
            let _ = (app, event);
        });
}

#[cfg(test)]
mod protocol_path_tests {
    use super::{decode_synara_media_path, timeline_media_content_type};

    #[test]
    fn timeline_media_requires_allowlisted_bytes_and_matching_mime() {
        let png = b"\x89PNG\r\n\x1a\nrest";
        assert_eq!(
            timeline_media_content_type(png, Some("image/png")),
            Some("image/png")
        );
        assert_eq!(timeline_media_content_type(png, Some("image/jpeg")), None);
        assert_eq!(
            timeline_media_content_type(b"arbitrary file bytes", None),
            None
        );
        assert_eq!(
            timeline_media_content_type(b"# heading\n", Some("text/markdown")),
            None
        );
    }

    #[test]
    fn media_protocol_path_decodes_mxc_and_rejects_traversal() {
        assert_eq!(
            decode_synara_media_path("timeline-media-ab").as_deref(),
            Some("timeline-media-ab")
        );
        assert_eq!(
            decode_synara_media_path("mxc%3A%2F%2Fexample.org%2Fmedia").as_deref(),
            Some("mxc://example.org/media")
        );
        assert_eq!(decode_synara_media_path("mxc%2F..%2Fsecret"), None);
        assert_eq!(decode_synara_media_path("%00"), None);
    }
}
