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
mod desktop_window_drag;
mod desktop_x_embed;
// P1.2: compile-only Matrix Rust SDK linkage; no production client session.
// P1.3: Matrix IPC schema foundation (types/helpers only; no production commands).
mod matrix;
#[cfg(target_os = "macos")]
mod menu;

use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use synara_core::platform::Platform;
use tauri::{
    webview::{NewWindowResponse, PageLoadEvent, WebviewWindowBuilder},
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

/// Brands of an ISO base media file (`ftyp` box): the major brand at 8..12
/// and the compatible brands after the minor version, within the box.
fn iso_media_brands(bytes: &[u8]) -> Vec<&[u8]> {
    if bytes.len() < 12 || &bytes[4..8] != b"ftyp" {
        return Vec::new();
    }
    let box_len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    let end = box_len.clamp(12, bytes.len().min(256));
    let mut brands = vec![&bytes[8..12]];
    let mut offset = 16;
    while offset + 4 <= end {
        brands.push(&bytes[offset..offset + 4]);
        offset += 4;
    }
    brands
}

/// The types an `ftyp` file may be served as, most specific first.
fn iso_media_candidates(bytes: &[u8]) -> &'static [&'static str] {
    let brands = iso_media_brands(bytes);
    let major = brands.first().copied().unwrap_or_default();
    let has = |names: &[&[u8; 4]]| {
        brands
            .iter()
            .any(|brand| names.iter().any(|name| *brand == &name[..]))
    };
    // HEIF stills (iPhone camera photos). WebKit on macOS decodes them; a
    // webview that cannot shows the row's file fallback instead.
    if matches!(major, b"heic" | b"heix" | b"heim" | b"heis") {
        return &["image/heic", "image/heif"];
    }
    if matches!(major, b"mif1" | b"msf1" | b"hevc" | b"hevx") || has(&[b"heic", b"mif1"]) {
        return &["image/heif", "image/heic"];
    }
    // QuickTime movies (iPhone camera videos) share the MP4 container.
    if major == b"qt  " {
        return &["video/quicktime", "video/mp4"];
    }
    if matches!(major, b"M4A " | b"M4B ") {
        return &["audio/mp4", "video/mp4"];
    }
    &["video/mp4", "audio/mp4", "video/quicktime"]
}

/// Common non-standard names senders use for types the protocol serves.
fn canonical_media_type(essence: &str) -> &str {
    match essence {
        "image/jpg" | "image/pjpeg" => "image/jpeg",
        "image/x-png" => "image/png",
        "audio/x-m4a" | "audio/m4a" | "audio/x-mp4" => "audio/mp4",
        "video/x-m4v" | "video/m4v" => "video/mp4",
        "audio/x-wav" | "audio/wave" | "audio/vnd.wave" => "audio/wav",
        "audio/x-flac" => "audio/flac",
        "audio/mp3" | "audio/x-mp3" | "audio/mpeg3" => "audio/mpeg",
        "audio/x-aac" | "audio/aacp" => "audio/aac",
        "audio/opus" | "audio/x-opus+ogg" | "audio/x-ogg" => "audio/ogg",
        "image/heic-sequence" => "image/heic",
        "image/heif-sequence" => "image/heif",
        other => other,
    }
}

/// The type to serve a timeline file as, or `None` (415). The bytes must
/// sniff as an allowlisted format and the sender's declared type must name
/// that format; only its essence counts (`audio/ogg; codecs=opus`).
fn timeline_media_content_type(bytes: &[u8], declared: Option<&str>) -> Option<&'static str> {
    let candidates: &[&'static str] = if let Some(image) = image_content_type(bytes) {
        &[image]
    } else if bytes.starts_with(b"%PDF-") {
        &["application/pdf"]
    } else if matches!(bytes, [0xff, second, ..] if second & 0xf6 == 0xf0) {
        // ADTS AAC: MPEG sync with layer bits 00.
        &["audio/aac"]
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
        iso_media_candidates(bytes)
    } else if bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]) {
        &["video/webm", "audio/webm"]
    } else {
        return None;
    };
    let essence = declared
        .and_then(|value| value.split(';').next())
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty());
    match essence.as_deref() {
        None | Some("application/octet-stream") => candidates.first().copied(),
        Some(value) => {
            let value = canonical_media_type(value);
            candidates
                .iter()
                .copied()
                .find(|candidate| candidate.eq_ignore_ascii_case(value))
        }
    }
}

/// A single satisfiable byte range of a `total`-byte body: `(start, end)`
/// inclusive. `Err(())` is unsatisfiable (416); `Ok(None)` serves the whole
/// body (no header, a multi-range, or a malformed one, as RFC 9110 allows).
fn requested_byte_range(header: Option<&str>, total: usize) -> Result<Option<(usize, usize)>, ()> {
    let Some(spec) = header.and_then(|value| value.trim().strip_prefix("bytes=")) else {
        return Ok(None);
    };
    if spec.contains(',') {
        return Ok(None);
    }
    let Some((start, end)) = spec.trim().split_once('-') else {
        return Ok(None);
    };
    let (start, end) = (start.trim(), end.trim());
    let parse = |value: &str| value.parse::<usize>().ok();
    let range = match (start.is_empty(), end.is_empty()) {
        // `bytes=-N`: the last N bytes.
        (true, false) => {
            let Some(suffix) = parse(end) else {
                return Ok(None);
            };
            if suffix == 0 || total == 0 {
                return Err(());
            }
            (total.saturating_sub(suffix), total - 1)
        }
        (false, _) => {
            let Some(first) = parse(start) else {
                return Ok(None);
            };
            if first >= total {
                return Err(());
            }
            let last = if end.is_empty() {
                total - 1
            } else {
                let Some(last) = parse(end) else {
                    return Ok(None);
                };
                if last < first {
                    return Ok(None);
                }
                last.min(total - 1)
            };
            (first, last)
        }
        (true, true) => return Ok(None),
    };
    Ok(Some(range))
}

/// Answer a media request from a fully buffered body, honouring `Range` so
/// WebKit can seek audio and video (it asks for ranges and expects 206).
fn synara_media_body_response(
    bytes: Vec<u8>,
    content_type: &str,
    range: Option<&str>,
) -> tauri::http::Response<Vec<u8>> {
    let total = bytes.len();
    let (status, body, content_range) = match requested_byte_range(range, total) {
        Ok(None) => (tauri::http::StatusCode::OK, bytes, None),
        Ok(Some((start, end))) => (
            tauri::http::StatusCode::PARTIAL_CONTENT,
            bytes[start..=end].to_vec(),
            Some(format!("bytes {start}-{end}/{total}")),
        ),
        Err(()) => (
            tauri::http::StatusCode::RANGE_NOT_SATISFIABLE,
            Vec::new(),
            Some(format!("bytes */{total}")),
        ),
    };
    let mut response = synara_media_response(status, body, Some(content_type));
    let headers = response.headers_mut();
    headers.insert(
        tauri::http::header::ACCEPT_RANGES,
        tauri::http::HeaderValue::from_static("bytes"),
    );
    if let Some(value) = content_range.and_then(|value| value.parse().ok()) {
        headers.insert(tauri::http::header::CONTENT_RANGE, value);
    }
    response
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

/// Thumbnails are bounded well below original media.
const THUMBNAIL_MEDIA_MAX_BYTES: usize = 8 * 1_048_576;

/// Requested sizes snap to a small set so one thumbnail serves every avatar or
/// preview of roughly that size (one cache entry, one server thumbnail).
const THUMBNAIL_SIZE_BUCKETS: [u32; 12] = [32, 48, 64, 96, 128, 192, 256, 320, 480, 640, 800, 960];

/// Renderer size hint carried in the protocol path:
/// `thumbnail/<width>x<height>/<crop|scale>/<target>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SynaraThumbnailHint {
    width: u32,
    height: u32,
    crop: bool,
}

impl SynaraThumbnailHint {
    fn media_format(self) -> matrix_sdk::media::MediaFormat {
        let mut settings =
            matrix_sdk::media::MediaThumbnailSettings::new(self.width.into(), self.height.into());
        settings.method = if self.crop {
            matrix_sdk::ruma::api::client::media::get_content_thumbnail::v3::Method::Crop
        } else {
            matrix_sdk::ruma::api::client::media::get_content_thumbnail::v3::Method::Scale
        };
        matrix_sdk::media::MediaFormat::Thumbnail(settings)
    }
}

fn thumbnail_bucket(requested: u32) -> u32 {
    THUMBNAIL_SIZE_BUCKETS
        .iter()
        .copied()
        .find(|bucket| *bucket >= requested)
        .unwrap_or(THUMBNAIL_SIZE_BUCKETS[THUMBNAIL_SIZE_BUCKETS.len() - 1])
}

/// Split an optional thumbnail hint off a decoded protocol path. `None` means
/// a malformed hint, which is a 404 rather than a silent full download.
fn split_thumbnail_hint(path: &str) -> Option<(Option<SynaraThumbnailHint>, &str)> {
    let Some(rest) = path.strip_prefix("thumbnail/") else {
        return Some((None, path));
    };
    let (size, rest) = rest.split_once('/')?;
    let (method, target) = rest.split_once('/')?;
    let (width, height) = size.split_once('x')?;
    let parse = |value: &str| -> Option<u32> {
        if value.is_empty() || value.len() > 5 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        value.parse::<u32>().ok().filter(|value| *value > 0)
    };
    let crop = match method {
        "crop" => true,
        "scale" => false,
        _ => return None,
    };
    if target.is_empty() || target.starts_with("thumbnail/") {
        return None;
    }
    Some((
        Some(SynaraThumbnailHint {
            width: thumbnail_bucket(parse(width)?),
            height: thumbnail_bucket(parse(height)?),
            crop,
        }),
        target,
    ))
}

fn synara_media_status(status: tauri::http::StatusCode) -> tauri::http::Response<Vec<u8>> {
    synara_media_response(status, Vec::new(), None)
}

/// Serve a server thumbnail of a plain source. `None` falls back to the
/// original (no thumbnail, an encrypted source, or a non-image reply).
async fn synara_thumbnail_response(
    client: &matrix_sdk::Client,
    source: &matrix_sdk::ruma::events::room::MediaSource,
    hint: Option<SynaraThumbnailHint>,
) -> Option<tauri::http::Response<Vec<u8>>> {
    let hint = hint?;
    // The homeserver cannot thumbnail an encrypted attachment.
    let matrix_sdk::ruma::events::room::MediaSource::Plain(_) = source else {
        return None;
    };
    let request = matrix_sdk::media::MediaRequestParameters {
        source: source.clone(),
        format: hint.media_format(),
    };
    let bytes =
        synara_core::app::media::fetch_media_cached(client, &request, THUMBNAIL_MEDIA_MAX_BYTES)
            .await
            .ok()?;
    let content_type = image_content_type(&bytes)?;
    Some(synara_media_response(
        tauri::http::StatusCode::OK,
        bytes,
        Some(content_type),
    ))
}

async fn synara_original_response(
    client: &matrix_sdk::Client,
    source: matrix_sdk::ruma::events::room::MediaSource,
    declared_mime_type: Option<&str>,
    range: Option<&str>,
) -> tauri::http::Response<Vec<u8>> {
    let request = matrix_sdk::media::MediaRequestParameters {
        source,
        format: matrix_sdk::media::MediaFormat::File,
    };
    let Ok(bytes) =
        synara_core::app::media::fetch_media_cached(client, &request, TIMELINE_MEDIA_MAX_BYTES)
            .await
    else {
        return synara_media_status(tauri::http::StatusCode::NOT_FOUND);
    };
    let Some(content_type) = timeline_media_content_type(&bytes, declared_mime_type) else {
        return synara_media_status(tauri::http::StatusCode::UNSUPPORTED_MEDIA_TYPE);
    };
    synara_media_body_response(bytes, content_type, range)
}

async fn synara_media_for_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    path: &str,
    range: Option<&str>,
) -> tauri::http::Response<Vec<u8>> {
    let Some((hint, handle)) = split_thumbnail_hint(path) else {
        return synara_media_status(tauri::http::StatusCode::NOT_FOUND);
    };
    let state = app.state::<matrix::auth::MatrixAuthState>();
    if matrix::timeline::is_timeline_media_handle(handle) {
        let Some((client, source)) = state.resolve_timeline_media(handle).await else {
            return synara_media_status(tauri::http::StatusCode::NOT_FOUND);
        };
        if let Some(response) = synara_thumbnail_response(&client, &source.source, hint).await {
            return response;
        }
        return synara_original_response(
            &client,
            source.source,
            source.declared_mime_type.as_deref(),
            range,
        )
        .await;
    }
    if let Ok(content_uri) = matrix::auth::product::parse_media_download_uri(handle) {
        let Some(client) = state.media_client().await else {
            return synara_media_status(tauri::http::StatusCode::NOT_FOUND);
        };
        let source = matrix_sdk::ruma::events::room::MediaSource::Plain(content_uri);
        if let Some(response) = synara_thumbnail_response(&client, &source, hint).await {
            return response;
        }
        return synara_original_response(&client, source, None, range).await;
    }
    if handle.len() != 64 || !handle.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return synara_media_status(tauri::http::StatusCode::NOT_FOUND);
    }
    let Some((client, source)) = state.resolve_invite_avatar(handle).await else {
        return synara_media_status(tauri::http::StatusCode::NOT_FOUND);
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
    let Ok(bytes) =
        synara_core::app::media::fetch_media_cached(&client, &request, INVITE_AVATAR_MAX_BYTES)
            .await
    else {
        return synara_media_status(tauri::http::StatusCode::NOT_FOUND);
    };
    let Some(content_type) = image_content_type(&bytes) else {
        return synara_media_status(tauri::http::StatusCode::UNSUPPORTED_MEDIA_TYPE);
    };
    synara_media_response(tauri::http::StatusCode::OK, bytes, Some(content_type))
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
            let range = request
                .headers()
                .get(tauri::http::header::RANGE)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            tauri::async_runtime::spawn(async move {
                let Some(handle) = decode_synara_media_path(&handle) else {
                    responder.respond(synara_media_response(
                        tauri::http::StatusCode::NOT_FOUND,
                        Vec::new(),
                        None,
                    ));
                    return;
                };
                let response = synara_media_for_path(&app, &handle, range.as_deref()).await;
                responder.respond(response);
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

const NATIVE_FILE_DROP_EVENT: &str = "synara-native-file-drop";

/// Drag position in CSS pixels: the event reports physical pixels and the
/// renderer hit-tests with `document.elementFromPoint`.
fn native_drop_position(position: tauri::PhysicalPosition<f64>, scale_factor: f64) -> (f64, f64) {
    let logical = position.to_logical::<f64>(scale_factor);
    (logical.x, logical.y)
}

/// One Tauri event per drag phase. The renderer routes it to the one
/// composer under the pointer; it is not also dispatched as a DOM event.
fn emit_native_file_drop(window: &tauri::Window, payload: NativeFileDropPayload) {
    let Some(webview) = window.get_webview_window(desktop::MAIN_WINDOW_LABEL) else {
        return;
    };
    let _ = webview.emit(NATIVE_FILE_DROP_EVENT, payload);
}

fn native_drop_payload(
    window: &tauri::Window,
    phase: &'static str,
    paths: Vec<String>,
    position: tauri::PhysicalPosition<f64>,
) -> NativeFileDropPayload {
    let (x, y) = native_drop_position(position, window.scale_factor().unwrap_or(1.0));
    NativeFileDropPayload { phase, paths, x, y }
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
        .manage(Arc::new(matrix::timeline::TimelineViewChannels::default()))
        .plugin({
            // macOS and Windows keep every default flag, including decorations.
            // Linux must not restore DECORATIONS: the saved file still says
            // decorated, and applying it puts the native chrome back.
            let window_state = tauri_plugin_window_state::Builder::default();
            #[cfg(target_os = "linux")]
            let window_state =
                window_state.with_state_flags(desktop_window_drag::linux_window_state_flags());
            window_state.build()
        })
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            desktop::desktop_show,
            desktop::desktop_hide,
            desktop::desktop_window_minimize,
            desktop::desktop_window_toggle_maximize,
            desktop::desktop_window_close,
            desktop::desktop_quit,
            desktop::desktop_render_markdown,
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
            desktop_notifications::desktop_dismiss_all_notifications,
            desktop_notifications::desktop_take_notification_responses,
            desktop::desktop_open_external_url,
            desktop_x_embed::desktop_x_post,
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
            matrix::auth::product::matrix_identity_reset_start,
            matrix::auth::product::matrix_identity_reset_password,
            matrix::auth::product::matrix_identity_reset_continue,
            matrix::auth::product::matrix_identity_reset_cancel,
            matrix::auth::product::matrix_device_verification_prompt,
            matrix::auth::product::matrix_device_verification_prompt_dismiss,
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
            matrix::auth::product::matrix_room_identity_warnings,
            matrix::auth::product::matrix_room_identity_warning_resolve,
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
            matrix::auth::product::matrix_account_data_types,
            matrix::auth::product::matrix_account_data_get,
            matrix::auth::product::matrix_account_data_set,
            matrix::auth::product::matrix_room_local_aliases,
            matrix::auth::product::matrix_room_alias_create,
            matrix::auth::product::matrix_room_alias_delete,
            matrix::auth::product::matrix_room_alias_check,
            matrix::auth::product::matrix_user_mutual_rooms,
            matrix::auth::product::matrix_room_upgrade,
            matrix::auth::product::matrix_room_bulk_redact,
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
            matrix::auth::product::desktop_timeline_subscribe,
            matrix::auth::product::desktop_timeline_unsubscribe,
            matrix::auth::product::matrix_timeline_open,
            matrix::auth::product::matrix_timeline_close,
            matrix::auth::product::matrix_timeline_jump_latest,
            matrix::auth::product::matrix_timeline_paginate,
            matrix::auth::product::matrix_timeline_snapshot,
            matrix::auth::product::matrix_timeline_retry_decryption,
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
            matrix::auth::product::matrix_local_echo_discard,
            matrix::auth::product::matrix_local_echo_retry,
            matrix::auth::product::matrix_edit_message,
            matrix::auth::product::matrix_send_attachment,
            matrix::auth::product::matrix_attachment_stage,
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
            bridge::agent_notification_preferences::matrix_agent_notification_preferences_snapshot,
            bridge::agent_notification_preferences::matrix_agent_notification_preferences_set,
            matrix::auth::product::matrix_room_notification_snapshot,
            matrix::auth::product::matrix_room_notification_set,
            matrix::auth::product::matrix_room_notifications_snapshot,
            matrix::auth::product::matrix_threepid_snapshot,
            matrix::auth::product::matrix_threepid_delete,
            matrix::auth::product::matrix_threepid_request_email_token,
            matrix::auth::product::matrix_threepid_add_email,
            matrix::auth::product::matrix_threepid_add_email_password,
            matrix::auth::product::matrix_change_own_password,
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
                    let _ = desktop::close_main_window(window.app_handle());
                }
                WindowEvent::DragDrop(DragDropEvent::Enter { paths, position }) => {
                    desktop_file_transfer::reset_drag_drop_session();
                    let payload =
                        native_drop_payload(window, "enter", native_drop_paths(paths), *position);
                    emit_native_file_drop(window, payload);
                }
                WindowEvent::DragDrop(DragDropEvent::Over { position }) => {
                    let payload = native_drop_payload(window, "over", Vec::new(), *position);
                    emit_native_file_drop(window, payload);
                }
                WindowEvent::DragDrop(DragDropEvent::Drop { paths, position }) => {
                    desktop_file_transfer::remember_dropped_paths(paths);
                    let payload =
                        native_drop_payload(window, "drop", native_drop_paths(paths), *position);
                    emit_native_file_drop(window, payload);
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
            desktop_tray::start_tray_host_watch();

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
            // The secure-store probe can take seconds on a slow or locked
            // keyring, so it must not delay the window. The bridge starts
            // conservative; the renderer reads the real status through
            // `desktop_secret_store_status`, which the warm-up below caches.
            desktop::warm_secret_store_status();
            let bridge_script = format!(
                "{}\nif (window.__SYNARA_DESKTOP__) {{ window.__SYNARA_DESKTOP__.supportsUpdater = {}; window.__SYNARA_DESKTOP__.supportsSecureSecretStore = false; window.__SYNARA_DESKTOP__.os = {}; }}",
                include_str!("desktop_bridge.js"),
                updater_configured,
                // WebKitGTK may present a macOS user agent; the renderer
                // reads the real OS from here instead.
                serde_json::to_string(std::env::consts::OS).unwrap_or_else(|_| "\"\"".to_owned()),
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
                // A reload drops the page's timeline views without unmounting
                // them: close their delta channels with the old page.
                .on_page_load(|window, payload| {
                    if payload.event() == PageLoadEvent::Started {
                        if let Some(channels) =
                            window.try_state::<Arc<matrix::timeline::TimelineViewChannels>>()
                        {
                            channels.close_webview(window.label());
                        }
                    }
                })
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
            if let Err(error) = desktop_window_drag::install(&window) {
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

            if let tauri::RunEvent::ExitRequested { api, code, .. } = &event {
                if begin_exit_sync_stop() {
                    // Stop sync so the SDK is not mid-request at exit, then
                    // exit for real. The stop is bounded: quit never hangs.
                    api.prevent_exit();
                    let app = app.clone();
                    let code = code.unwrap_or(0);
                    tauri::async_runtime::spawn(async move {
                        if let Some(state) = app.try_state::<matrix::auth::MatrixAuthState>() {
                            let _ = state.stop_sync_for_exit(EXIT_SYNC_STOP_BOUND).await;
                        }
                        app.exit(code);
                    });
                }
            }
        });
}

/// Upper bound on stopping sync during quit.
const EXIT_SYNC_STOP_BOUND: std::time::Duration = std::time::Duration::from_secs(2);

static EXIT_SYNC_STOP_STARTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// True only for the first exit request. The exit that follows the bounded
/// sync stop must pass straight through instead of being prevented again.
fn begin_exit_sync_stop() -> bool {
    !EXIT_SYNC_STOP_STARTED.swap(true, std::sync::atomic::Ordering::AcqRel)
}

#[cfg(test)]
mod exit_tests {
    use super::*;

    #[test]
    fn only_the_first_exit_request_is_held_for_the_sync_stop() {
        assert!(begin_exit_sync_stop());
        assert!(!begin_exit_sync_stop());
        assert!(!begin_exit_sync_stop());
    }

    #[tokio::test]
    async fn exit_sync_stop_without_a_session_returns_at_once() {
        let state = matrix::auth::MatrixAuthState::new();
        let started = std::time::Instant::now();
        assert!(!state.stop_sync_for_exit(EXIT_SYNC_STOP_BOUND).await);
        assert!(started.elapsed() < EXIT_SYNC_STOP_BOUND);
    }

    #[tokio::test]
    async fn exit_sync_stop_gives_up_when_the_session_lock_is_held() {
        let state = std::sync::Arc::new(matrix::auth::MatrixAuthState::new());
        let _held = state.hold_session_lock_for_test().await;
        let bound = std::time::Duration::from_millis(100);
        let started = std::time::Instant::now();
        assert!(!state.stop_sync_for_exit(bound).await);
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
    }

    #[test]
    fn window_setup_does_not_wait_for_the_secure_store_probe() {
        let source = include_str!("lib.rs");
        let setup = source
            .split("let bridge_script = format!(")
            .nth(1)
            .and_then(|rest| rest.split(");").next())
            .expect("bridge script");
        assert!(setup.contains("supportsSecureSecretStore = false"));
        assert!(!setup.contains("secret_store"));
        assert!(source.contains("desktop::warm_secret_store_status();"));
        let desktop = include_str!("desktop.rs");
        assert!(desktop.contains("pub async fn desktop_secret_store_status()"));
        assert!(desktop
            .contains("spawn_blocking(crate::desktop_secret_store::platform_secret_store_status)"));
    }
}

#[cfg(test)]
mod command_manifest_tests {
    use std::collections::BTreeSet;

    fn registered_commands() -> BTreeSet<String> {
        let source = include_str!("lib.rs");
        let handler = source
            .split("generate_handler![")
            .nth(1)
            .and_then(|rest| rest.split("])").next())
            .expect("generate_handler! list");
        handler
            .split(',')
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(|path| path.rsplit("::").next().unwrap_or(path).to_owned())
            .collect()
    }

    fn manifest_commands() -> BTreeSet<String> {
        let build = include_str!("../build.rs");
        let list = build
            .split("const DESKTOP_COMMANDS: &[&str] = &[")
            .nth(1)
            .and_then(|rest| rest.split("];").next())
            .expect("DESKTOP_COMMANDS list");
        list.split(',')
            .map(|entry| entry.trim().trim_matches('"'))
            .filter(|entry| !entry.is_empty())
            .map(str::to_owned)
            .collect()
    }

    fn granted_commands(capability: &str) -> BTreeSet<String> {
        let value: serde_json::Value = serde_json::from_str(capability).expect("capability json");
        value["permissions"]
            .as_array()
            .expect("permissions")
            .iter()
            .filter_map(|permission| permission.as_str())
            .filter(|permission| !permission.contains(':'))
            .filter_map(|permission| permission.strip_prefix("allow-"))
            .map(|permission| permission.replace('-', "_"))
            .collect()
    }

    #[test]
    fn the_build_manifest_lists_exactly_the_registered_commands() {
        let registered = registered_commands();
        let manifest = manifest_commands();
        assert!(
            registered.len() > 200,
            "parsed {} commands",
            registered.len()
        );
        let unlisted: Vec<_> = registered.difference(&manifest).collect();
        let stale: Vec<_> = manifest.difference(&registered).collect();
        assert!(unlisted.is_empty(), "missing from build.rs: {unlisted:?}");
        assert!(stale.is_empty(), "not registered: {stale:?}");
    }

    #[test]
    fn capabilities_grant_only_registered_commands_and_every_one_once() {
        let registered = registered_commands();
        let main = granted_commands(include_str!("../capabilities/main.json"));
        let widget = granted_commands(include_str!("../capabilities/widget.json"));
        assert_eq!(widget.iter().collect::<Vec<_>>(), ["widget_bridge_post"]);
        let granted: BTreeSet<String> = main.union(&widget).cloned().collect();
        let ungranted: Vec<_> = registered.difference(&granted).collect();
        let unknown: Vec<_> = granted.difference(&registered).collect();
        assert!(
            ungranted.is_empty(),
            "registered but not granted: {ungranted:?}"
        );
        assert!(
            unknown.is_empty(),
            "granted but not registered: {unknown:?}"
        );
        assert!(!main.contains("widget_bridge_post"));
    }

    #[test]
    fn every_registered_command_has_its_generated_permission_file() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("permissions/autogenerated");
        let files: BTreeSet<String> = std::fs::read_dir(&dir)
            .expect("autogenerated permissions")
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .and_then(|name| name.strip_suffix(".toml"))
                    .map(str::to_owned)
            })
            .collect();
        let missing: Vec<_> = registered_commands().difference(&files).cloned().collect();
        assert!(missing.is_empty(), "no permission file: {missing:?}");
    }
}

#[cfg(test)]
mod file_drop_tests {
    use super::native_drop_position;

    #[test]
    fn drag_positions_reach_the_renderer_in_css_pixels() {
        assert_eq!(
            native_drop_position(tauri::PhysicalPosition::new(200.0, 100.0), 2.0),
            (100.0, 50.0)
        );
        assert_eq!(
            native_drop_position(tauri::PhysicalPosition::new(30.0, 40.0), 1.0),
            (30.0, 40.0)
        );
    }

    #[test]
    fn each_drag_phase_is_sent_once_on_one_channel() {
        let source = include_str!("lib.rs");
        let emit = source
            .split("fn emit_native_file_drop")
            .nth(1)
            .and_then(|rest| rest.split("\nfn ").next())
            .expect("emit_native_file_drop");
        assert_eq!(emit.matches(".emit(").count(), 1);
        assert!(!emit.contains(".eval("));
        assert!(!emit.contains("dispatchEvent"));
        let hook = include_str!("../../synara/src/app/hooks/useFileDrop.ts");
        assert!(!hook.contains("addEventListener('synara-native-file-drop'"));
    }
}

#[cfg(test)]
mod csp_tests;

#[cfg(test)]
mod protocol_path_tests {
    use super::{
        decode_synara_media_path, requested_byte_range, split_thumbnail_hint,
        synara_media_body_response, thumbnail_bucket, timeline_media_content_type,
        SynaraThumbnailHint,
    };

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

    fn ftyp(major: &[u8; 4], compatible: &[&[u8; 4]]) -> Vec<u8> {
        let len = 16 + 4 * compatible.len();
        let mut bytes = (len as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(b"ftyp");
        bytes.extend_from_slice(major);
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        for brand in compatible {
            bytes.extend_from_slice(*brand);
        }
        bytes.extend_from_slice(b"\0\0\0\x08free");
        bytes
    }

    #[test]
    fn timeline_media_accepts_iphone_and_bridge_formats() {
        // iPhone camera defaults: HEIC photos and QuickTime videos.
        let heic = ftyp(b"heic", &[b"mif1", b"heic"]);
        assert_eq!(
            timeline_media_content_type(&heic, Some("image/heic")),
            Some("image/heic")
        );
        assert_eq!(
            timeline_media_content_type(&heic, Some("image/heif")),
            Some("image/heif")
        );
        let mif1 = ftyp(b"mif1", &[b"heic"]);
        assert_eq!(
            timeline_media_content_type(&mif1, Some("image/heic")),
            Some("image/heic")
        );
        let mov = ftyp(b"qt  ", &[b"qt  "]);
        assert_eq!(
            timeline_media_content_type(&mov, Some("video/quicktime")),
            Some("video/quicktime")
        );
        assert_eq!(
            timeline_media_content_type(&mov, Some("video/mp4")),
            Some("video/mp4")
        );
        let mp4 = ftyp(b"isom", &[b"isom", b"mp41"]);
        assert_eq!(
            timeline_media_content_type(&mp4, Some("video/mp4")),
            Some("video/mp4")
        );
        let m4a = ftyp(b"M4A ", &[b"M4A ", b"mp42"]);
        assert_eq!(
            timeline_media_content_type(&m4a, Some("audio/x-m4a")),
            Some("audio/mp4")
        );
        // Parameters and case do not matter; only the essence does.
        assert_eq!(
            timeline_media_content_type(b"OggS\0\x02rest", Some("audio/ogg; codecs=opus")),
            Some("audio/ogg")
        );
        assert_eq!(
            timeline_media_content_type(b"OggS\0\x02rest", Some("Audio/OGG")),
            Some("audio/ogg")
        );
        assert_eq!(
            timeline_media_content_type(b"RIFF\0\0\0\0WAVEfmt ", Some("audio/x-wav")),
            Some("audio/wav")
        );
        // ADTS AAC is no longer mistaken for MP3.
        assert_eq!(
            timeline_media_content_type(&[0xff, 0xf1, 0x50, 0x80], Some("audio/aac")),
            Some("audio/aac")
        );
        assert_eq!(
            timeline_media_content_type(&[0xff, 0xfb, 0x90, 0x00], Some("audio/mpeg")),
            Some("audio/mpeg")
        );
        assert_eq!(
            timeline_media_content_type(b"\xff\xd8\xff\xe0rest", Some("image/jpg")),
            Some("image/jpeg")
        );
        // Still fail closed on a real mismatch.
        assert_eq!(timeline_media_content_type(&heic, Some("video/mp4")), None);
        assert_eq!(
            timeline_media_content_type(&mp4, Some("text/html; charset=utf-8")),
            None
        );
    }

    #[test]
    fn media_protocol_answers_byte_ranges() {
        assert_eq!(requested_byte_range(None, 10), Ok(None));
        assert_eq!(requested_byte_range(Some("bytes=0-"), 10), Ok(Some((0, 9))));
        assert_eq!(
            requested_byte_range(Some("bytes=0-1"), 10),
            Ok(Some((0, 1)))
        );
        assert_eq!(
            requested_byte_range(Some("bytes=4-99"), 10),
            Ok(Some((4, 9)))
        );
        assert_eq!(requested_byte_range(Some("bytes=-3"), 10), Ok(Some((7, 9))));
        assert_eq!(
            requested_byte_range(Some("bytes=-30"), 10),
            Ok(Some((0, 9)))
        );
        assert_eq!(requested_byte_range(Some("bytes=10-"), 10), Err(()));
        assert_eq!(requested_byte_range(Some("bytes=-0"), 10), Err(()));
        // Multi-range and malformed headers fall back to the whole body.
        assert_eq!(requested_byte_range(Some("bytes=0-1,4-5"), 10), Ok(None));
        assert_eq!(requested_byte_range(Some("bytes=5-2"), 10), Ok(None));
        assert_eq!(requested_byte_range(Some("items=0-1"), 10), Ok(None));

        let body: Vec<u8> = (0..10).collect();
        let full = synara_media_body_response(body.clone(), "video/mp4", None);
        assert_eq!(full.status(), tauri::http::StatusCode::OK);
        assert_eq!(full.body().len(), 10);
        assert_eq!(
            full.headers()
                .get(tauri::http::header::ACCEPT_RANGES)
                .unwrap(),
            "bytes"
        );
        let partial = synara_media_body_response(body.clone(), "video/mp4", Some("bytes=2-4"));
        assert_eq!(partial.status(), tauri::http::StatusCode::PARTIAL_CONTENT);
        assert_eq!(partial.body(), &vec![2, 3, 4]);
        assert_eq!(
            partial
                .headers()
                .get(tauri::http::header::CONTENT_RANGE)
                .unwrap(),
            "bytes 2-4/10"
        );
        assert_eq!(
            partial
                .headers()
                .get(tauri::http::header::CONTENT_TYPE)
                .unwrap(),
            "video/mp4"
        );
        let unsatisfiable = synara_media_body_response(body, "video/mp4", Some("bytes=20-"));
        assert_eq!(
            unsatisfiable.status(),
            tauri::http::StatusCode::RANGE_NOT_SATISFIABLE
        );
        assert_eq!(
            unsatisfiable
                .headers()
                .get(tauri::http::header::CONTENT_RANGE)
                .unwrap(),
            "bytes */10"
        );
    }

    #[test]
    fn media_protocol_thumbnail_hints_bucket_and_fail_closed() {
        assert_eq!(
            split_thumbnail_hint("timeline-media-ab"),
            Some((None, "timeline-media-ab"))
        );
        assert_eq!(
            split_thumbnail_hint("thumbnail/96x96/crop/mxc://example.org/avatar"),
            Some((
                Some(SynaraThumbnailHint {
                    width: 96,
                    height: 96,
                    crop: true,
                }),
                "mxc://example.org/avatar"
            ))
        );
        assert_eq!(
            split_thumbnail_hint("thumbnail/100x70/scale/timeline-media-ab"),
            Some((
                Some(SynaraThumbnailHint {
                    width: 128,
                    height: 96,
                    crop: false,
                }),
                "timeline-media-ab"
            ))
        );
        assert_eq!(thumbnail_bucket(1), 32);
        assert_eq!(thumbnail_bucket(96), 96);
        assert_eq!(thumbnail_bucket(5000), 960);
        for invalid in [
            "thumbnail/",
            "thumbnail/96x96/crop/",
            "thumbnail/0x96/crop/mxc://example.org/a",
            "thumbnail/96/crop/mxc://example.org/a",
            "thumbnail/96x96/stretch/mxc://example.org/a",
            "thumbnail/-1x96/crop/mxc://example.org/a",
            "thumbnail/999999x96/crop/mxc://example.org/a",
            "thumbnail/96x96/crop/thumbnail/96x96/crop/mxc://example.org/a",
        ] {
            assert_eq!(split_thumbnail_hint(invalid), None, "{invalid}");
        }
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
