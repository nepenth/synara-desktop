use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
#[cfg(target_os = "macos")]
use tauri::image::Image;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindow};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
#[cfg(debug_assertions)]
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::build_info;
use crate::desktop_file_transfer::{
    downloads_dir, dropped_file_read_mode, new_file_transfer_id, sanitize_download_filename,
    should_stream_file_ipc, unique_download_path, DroppedFileReadMode, DESKTOP_FILE_IPC_CHUNK_SIZE,
};
use crate::desktop_sanitize::{
    sanitize_action_text, sanitize_notification_route, sanitize_route, truncate_text,
};
#[cfg(any(target_os = "windows", test))]
use crate::desktop_secret_store::DESKTOP_SECRET_STORE_WINDOWS_UNSUPPORTED;
use crate::desktop_secret_store::{bridge_supports_secure_secret_store, DesktopSecretStoreStatus};
#[cfg(test)]
use crate::desktop_secret_store::{
    unavailable_secret_store_status, DESKTOP_SECRET_STORE_BACKEND_NONE,
};
use crate::desktop_session::DesktopSessionEnvelope;
use crate::desktop_session_store::{
    desktop_get_session_from_store, desktop_remove_session_from_store,
    desktop_set_session_in_store, DesktopSessionSecretStore, KeyringDesktopSessionSecretStore,
};
use crate::desktop_shortcuts::{
    desktop_set_shortcuts as apply_desktop_shortcuts_command, desktop_shortcuts_integration_status,
    DesktopShortcutApplyResult, DesktopShortcutConfig,
};
use crate::desktop_url;

pub const MAIN_WINDOW_LABEL: &str = "main";

const MENU_SHOW: &str = "desktop.show";
const MENU_LATER: &str = "desktop.later";
const MENU_NOTIFICATIONS: &str = "desktop.notifications";
const MENU_UNREAD_SUMMARY: &str = "desktop.unread-summary";
const MENU_DESKTOP_INTEGRATION: &str = "desktop.integration";
const MENU_DND_TOGGLE: &str = "desktop.dnd";
const MENU_BUILD_INFO: &str = "desktop.build-info";
const MENU_QUIT: &str = "desktop.quit";

pub const DESKTOP_TRAY_DND_TOGGLE_EVENT: &str = "synara-tray-dnd-toggle";

const ROUTE_HOME: &str = "/";
const ROUTE_LATER: &str = "/inbox/later/";
const ROUTE_NOTIFICATIONS: &str = "/inbox/notifications/";
const ROUTE_SETTINGS: &str = "/settings/";

const TRAY_ICON_ID: &str = "synara-tray";
const TRAY_STATE_APPLY_MIN_INTERVAL_MS: u64 = 500;

#[cfg(debug_assertions)]
static TRAY_MENU_REBUILD_COUNT: AtomicU64 = AtomicU64::new(0);
const MAX_DROPPED_FILES: usize = 32;
const MAX_DROPPED_FILE_ALLOWLIST: usize = 256;
const MAX_DROPPED_FILE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_DROPPED_TOTAL_BYTES: u64 = 512 * 1024 * 1024;

#[cfg(not(test))]
const DROPPED_FILE_ALLOWLIST_TTL: Option<Duration> = Some(Duration::from_secs(60));
#[cfg(test)]
const DROPPED_FILE_ALLOWLIST_TTL: Option<Duration> = Some(Duration::from_millis(5));
const MAX_ACTIVE_FILE_TRANSFERS: usize = 16;
const FILE_TRANSFER_SESSION_TTL: Duration = Duration::from_secs(300);

#[derive(Clone, Serialize, serde::Deserialize)]
pub struct DesktopAgentActionPayload {
    id: String,
    title: String,
    kind: Option<String>,
    prompt: Option<String>,
    url: Option<String>,
    markdown: Option<String>,
}

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopNotificationPayload {
    pub title: String,
    pub body: Option<String>,
    pub route: Option<String>,
}

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSaveFilePayload {
    pub filename: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDroppedFilePayload {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSaveFileBeginResult {
    pub session_id: String,
}

struct SaveFileSession {
    temp_path: PathBuf,
    filename: String,
    expected_size: u64,
    bytes_received: u64,
    created_at: Instant,
}

struct DroppedReadSession {
    path: PathBuf,
    size: u64,
    created_at: Instant,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopIntegrationCheck {
    pub name: String,
    pub ready: bool,
    pub supported: bool,
    pub message: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopIntegrationStatus {
    pub platform: &'static str,
    pub desktop_environment: String,
    pub session_type: String,
    pub distro_id: String,
    pub distro_name: String,
    pub distro_version: String,
    pub build_identity: String,
    pub tray: DesktopIntegrationCheck,
    pub notifications: DesktopIntegrationCheck,
    pub global_shortcuts: DesktopIntegrationCheck,
    pub file_portal: DesktopIntegrationCheck,
    pub media_portal: DesktopIntegrationCheck,
}

#[derive(Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopTrayState {
    pub unread_count: i64,
    pub highlight_count: i64,
    pub later_count: i64,
    pub notification_inbox_count: i64,
    pub do_not_disturb: bool,
}

struct TrayMenuItems<R: Runtime> {
    later: MenuItem<R>,
    notifications: MenuItem<R>,
    #[cfg(target_os = "linux")]
    unread_summary: MenuItem<R>,
    #[cfg(target_os = "linux")]
    dnd: MenuItem<R>,
}

struct TrayStateCoalescer {
    pending: Mutex<Option<DesktopTrayState>>,
    last_applied_at: Mutex<Option<Instant>>,
    flush_scheduled: AtomicBool,
}

#[cfg(debug_assertions)]
#[allow(dead_code)]
pub fn debug_tray_menu_rebuild_count() -> u64 {
    TRAY_MENU_REBUILD_COUNT.load(Ordering::Relaxed)
}

impl TrayStateCoalescer {
    fn new() -> Self {
        Self {
            pending: Mutex::new(None),
            last_applied_at: Mutex::new(None),
            flush_scheduled: AtomicBool::new(false),
        }
    }
}

fn tray_state_apply_min_interval() -> Duration {
    Duration::from_millis(TRAY_STATE_APPLY_MIN_INTERVAL_MS)
}

fn should_apply_tray_state_now(last_applied_at: Option<Instant>, now: Instant) -> bool {
    last_applied_at
        .map(|applied_at| now.duration_since(applied_at) >= tray_state_apply_min_interval())
        .unwrap_or(true)
}

fn normalize_tray_state(state: DesktopTrayState) -> DesktopTrayState {
    DesktopTrayState {
        unread_count: clamp_count(state.unread_count),
        highlight_count: clamp_count(state.highlight_count),
        later_count: clamp_count(state.later_count),
        notification_inbox_count: clamp_count(state.notification_inbox_count),
        do_not_disturb: state.do_not_disturb,
    }
}

#[derive(Clone, Serialize)]
struct DesktopAgentActionEvent {
    action: DesktopAgentActionPayload,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopPerformanceCapabilities {
    platform: &'static str,
    app_version: &'static str,
    build_revision: &'static str,
    build_branch: &'static str,
    build_label: String,
}

const DESKTOP_AGENT_ACTION_MAX_TEXT_CHARS: usize = 1024;
const DESKTOP_AGENT_ACTION_MAX_URL_CHARS: usize = 2048;
const DESKTOP_AGENT_ACTION_MAX_MARKDOWN_CHARS: usize = 16_384;
const DESKTOP_NOTIFICATION_MAX_TITLE_CHARS: usize = 120;
const DESKTOP_NOTIFICATION_MAX_BODY_CHARS: usize = 500;
const UNKNOWN_INTEGRATION_VALUE: &str = "unknown";
const MAX_TRAY_COUNT: i64 = 9_999;
const ALLOWED_AGENT_ACTION_KIND: &[&str] = &[
    "agent",
    "copy",
    "continue",
    "export",
    "prompt",
    "regenerate",
    "run",
    "open",
    "open_url",
];

fn sanitize_notification_payload(
    notification: DesktopNotificationPayload,
) -> Result<DesktopNotificationPayload, String> {
    let title = sanitize_action_text(notification.title, DESKTOP_NOTIFICATION_MAX_TITLE_CHARS);
    if title.is_empty() {
        return Err("Notification title cannot be empty".to_owned());
    }

    let body = notification
        .body
        .map(|value| sanitize_action_text(value, DESKTOP_NOTIFICATION_MAX_BODY_CHARS))
        .filter(|value| !value.is_empty());

    let route = match notification.route {
        Some(value) => Some(sanitize_notification_route(value)?),
        None => None,
    };

    Ok(DesktopNotificationPayload { title, body, route })
}

/// User-clicked external URLs are handed to the OS browser, not fetched by Synara.
/// Block local file/code schemes and embedded credentials, but allow ordinary
/// http(s) links including LAN/internal hosts users intentionally click.
pub fn is_safe_external_url(value: &str) -> bool {
    desktop_url::is_safe_external_url(value)
}

#[tauri::command]
pub fn desktop_open_external_url<R: Runtime>(app: AppHandle<R>, url: String) -> bool {
    if !is_safe_external_url(&url) {
        return false;
    }

    match app.opener().open_url(url, None::<&str>) {
        Ok(()) => true,
        Err(error) => {
            eprintln!("[synara] Failed to open external URL: {error}");
            false
        }
    }
}

fn purge_stale_file_transfer_sessions(now: Instant) {
    if let Ok(mut sessions) = save_file_sessions().lock() {
        let stale_ids: Vec<String> = sessions
            .iter()
            .filter_map(|(session_id, session)| {
                (now.duration_since(session.created_at) > FILE_TRANSFER_SESSION_TTL)
                    .then_some(session_id.clone())
            })
            .collect();
        for session_id in stale_ids {
            if let Some(session) = sessions.remove(&session_id) {
                let _ = fs::remove_file(session.temp_path);
            }
        }
    }

    if let Ok(mut sessions) = dropped_read_sessions().lock() {
        sessions.retain(|_, session| {
            now.duration_since(session.created_at) <= FILE_TRANSFER_SESSION_TTL
        });
    }
}

fn save_file_sessions() -> &'static Mutex<HashMap<String, SaveFileSession>> {
    static SAVE_FILE_SESSIONS: OnceLock<Mutex<HashMap<String, SaveFileSession>>> = OnceLock::new();
    SAVE_FILE_SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn dropped_read_sessions() -> &'static Mutex<HashMap<String, DroppedReadSession>> {
    static DROPPED_READ_SESSIONS: OnceLock<Mutex<HashMap<String, DroppedReadSession>>> =
        OnceLock::new();
    DROPPED_READ_SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn register_save_session(session: SaveFileSession) -> Result<String, String> {
    purge_stale_file_transfer_sessions(Instant::now());
    let session_id = new_file_transfer_id("save");
    let mut sessions = save_file_sessions()
        .lock()
        .map_err(|_| "Unable to access save file sessions".to_owned())?;
    if sessions.len() >= MAX_ACTIVE_FILE_TRANSFERS {
        return Err("Too many active file save transfers".to_owned());
    }
    sessions.insert(session_id.clone(), session);
    Ok(session_id)
}

fn save_session_is_stale(session: &SaveFileSession, now: Instant) -> bool {
    now.duration_since(session.created_at) > FILE_TRANSFER_SESSION_TTL
}

fn dropped_read_session_is_stale(session: &DroppedReadSession, now: Instant) -> bool {
    now.duration_since(session.created_at) > FILE_TRANSFER_SESSION_TTL
}

fn take_save_session(session_id: &str) -> Result<SaveFileSession, String> {
    purge_stale_file_transfer_sessions(Instant::now());
    let now = Instant::now();
    let mut sessions = save_file_sessions()
        .lock()
        .map_err(|_| "Unable to access save file sessions".to_owned())?;
    let session = sessions
        .remove(session_id)
        .ok_or_else(|| "Save file session is not available".to_owned())?;
    if save_session_is_stale(&session, now) {
        let _ = fs::remove_file(session.temp_path);
        return Err("Save file session has expired".to_owned());
    }
    Ok(session)
}

fn register_dropped_read_session(session: DroppedReadSession) -> Result<String, String> {
    purge_stale_file_transfer_sessions(Instant::now());
    let transfer_id = new_file_transfer_id("drop");
    let mut sessions = dropped_read_sessions()
        .lock()
        .map_err(|_| "Unable to access dropped file read sessions".to_owned())?;
    if sessions.len() >= MAX_ACTIVE_FILE_TRANSFERS {
        return Err("Too many active dropped file read transfers".to_owned());
    }
    sessions.insert(transfer_id.clone(), session);
    Ok(transfer_id)
}

fn remove_dropped_read_session(transfer_id: &str) {
    if let Ok(mut sessions) = dropped_read_sessions().lock() {
        sessions.remove(transfer_id);
    }
}

fn remove_save_session(session_id: &str) {
    if let Ok(mut sessions) = save_file_sessions().lock() {
        if let Some(session) = sessions.remove(session_id) {
            let _ = fs::remove_file(session.temp_path);
        }
    }
}

fn finalize_save_session(session: SaveFileSession) -> Result<String, String> {
    if session.bytes_received != session.expected_size {
        let _ = fs::remove_file(&session.temp_path);
        return Err("Save file transfer is incomplete".to_owned());
    }

    let downloads = downloads_dir()?;
    fs::create_dir_all(&downloads).map_err(|err| format!("Unable to create Downloads: {err}"))?;
    let filename = sanitize_download_filename(&session.filename);
    let destination = unique_download_path(&downloads, &filename);
    fs::rename(&session.temp_path, &destination)
        .map_err(|err| format!("Unable to finalize saved file: {err}"))?;

    Ok(destination.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn desktop_save_file(payload: DesktopSaveFilePayload) -> Result<String, String> {
    if payload.bytes.is_empty() {
        return Err("File is empty".to_owned());
    }
    if should_stream_file_ipc(payload.bytes.len() as u64) {
        return Err("File is too large for inline save; use streaming save commands".to_owned());
    }

    let downloads = downloads_dir()?;
    fs::create_dir_all(&downloads).map_err(|err| format!("Unable to create Downloads: {err}"))?;
    let filename = sanitize_download_filename(&payload.filename);
    let path = unique_download_path(&downloads, &filename);
    fs::write(&path, payload.bytes).map_err(|err| format!("Unable to write file: {err}"))?;

    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn desktop_save_file_begin(
    filename: String,
    total_size: u64,
) -> Result<DesktopSaveFileBeginResult, String> {
    if total_size == 0 {
        return Err("File is empty".to_owned());
    }
    if !should_stream_file_ipc(total_size) {
        return Err("File is small enough for inline save".to_owned());
    }

    let safe_filename = sanitize_download_filename(&filename);
    let temp_path = std::env::temp_dir().join(new_file_transfer_id("save-temp"));
    File::create(&temp_path).map_err(|err| format!("Unable to create temp save file: {err}"))?;

    let session = SaveFileSession {
        temp_path: temp_path.clone(),
        filename: safe_filename,
        expected_size: total_size,
        bytes_received: 0,
        created_at: Instant::now(),
    };
    let session_id = register_save_session(session).map_err(|err| {
        let _ = fs::remove_file(temp_path);
        err
    })?;

    Ok(DesktopSaveFileBeginResult { session_id })
}

#[tauri::command]
pub fn desktop_save_file_chunk(
    session_id: String,
    offset: u64,
    bytes: Vec<u8>,
) -> Result<bool, String> {
    if bytes.is_empty() {
        return Err("Save file chunk is empty".to_owned());
    }
    if bytes.len() > DESKTOP_FILE_IPC_CHUNK_SIZE {
        return Err(format!(
            "Save file chunk exceeds maximum size of {} bytes",
            DESKTOP_FILE_IPC_CHUNK_SIZE
        ));
    }

    purge_stale_file_transfer_sessions(Instant::now());
    let now = Instant::now();
    let mut sessions = save_file_sessions()
        .lock()
        .map_err(|_| "Unable to access save file sessions".to_owned())?;
    if let Some(session) = sessions.get(&session_id) {
        if save_session_is_stale(session, now) {
            if let Some(stale_session) = sessions.remove(&session_id) {
                let _ = fs::remove_file(stale_session.temp_path);
            }
            return Err("Save file session has expired".to_owned());
        }
    }
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| "Save file session is not available".to_owned())?;

    if offset != session.bytes_received {
        return Err("Save file chunk offset is out of order".to_owned());
    }
    if session.bytes_received.saturating_add(bytes.len() as u64) > session.expected_size {
        return Err("Save file transfer exceeds declared size".to_owned());
    }

    let mut file = OpenOptions::new()
        .write(true)
        .open(&session.temp_path)
        .map_err(|err| format!("Unable to open temp save file: {err}"))?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|err| format!("Unable to seek temp save file: {err}"))?;
    file.write_all(&bytes)
        .map_err(|err| format!("Unable to write save file chunk: {err}"))?;
    session.bytes_received = session.bytes_received.saturating_add(bytes.len() as u64);

    Ok(true)
}

#[tauri::command]
pub fn desktop_save_file_end(session_id: String) -> Result<String, String> {
    let session = take_save_session(&session_id)?;
    finalize_save_session(session)
}

#[tauri::command]
pub fn desktop_save_file_abort(session_id: String) -> Result<bool, String> {
    purge_stale_file_transfer_sessions(Instant::now());
    remove_save_session(&session_id);
    Ok(true)
}

#[derive(Default)]
struct DragDropSession {
    dropped_this_drag: bool,
}

struct DroppedFileAllowlist {
    order: VecDeque<PathBuf>,
    entries: HashMap<PathBuf, Instant>,
}

impl DroppedFileAllowlist {
    fn new() -> Self {
        Self {
            order: VecDeque::new(),
            entries: HashMap::new(),
        }
    }

    fn purge_expired(&mut self, now: Instant) {
        let Some(ttl) = DROPPED_FILE_ALLOWLIST_TTL else {
            return;
        };

        while let Some(front) = self.order.front().cloned() {
            let Some(added_at) = self.entries.get(&front).copied() else {
                self.order.pop_front();
                continue;
            };
            if now.duration_since(added_at) <= ttl {
                break;
            }
            self.remove_entry(&front);
        }
    }

    fn evict_to_cap(&mut self) {
        while self.entries.len() > MAX_DROPPED_FILE_ALLOWLIST {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            self.entries.remove(&oldest);
        }
    }

    fn insert(&mut self, path: PathBuf, now: Instant) {
        if self.entries.contains_key(&path) {
            self.touch(&path, now);
            return;
        }

        self.entries.insert(path.clone(), now);
        self.order.push_back(path);
        self.evict_to_cap();
    }

    fn touch(&mut self, path: &PathBuf, now: Instant) {
        self.entries.insert(path.clone(), now);
        if let Some(position) = self.order.iter().position(|entry| entry == path) {
            self.order.remove(position);
        }
        self.order.push_back(path.clone());
    }

    fn remove_entry(&mut self, path: &PathBuf) {
        self.entries.remove(path);
        if let Some(position) = self.order.iter().position(|entry| entry == path) {
            self.order.remove(position);
        }
    }

    fn remove(&mut self, path: &PathBuf) -> bool {
        if self.entries.remove(path).is_some() {
            if let Some(position) = self.order.iter().position(|entry| entry == path) {
                self.order.remove(position);
            }
            true
        } else {
            false
        }
    }

    fn clear(&mut self) {
        self.order.clear();
        self.entries.clear();
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }
}

fn dropped_file_allowlist() -> &'static Mutex<DroppedFileAllowlist> {
    static DROPPED_FILE_ALLOWLIST_STATE: OnceLock<Mutex<DroppedFileAllowlist>> = OnceLock::new();
    DROPPED_FILE_ALLOWLIST_STATE.get_or_init(|| Mutex::new(DroppedFileAllowlist::new()))
}

fn drag_drop_session() -> &'static Mutex<DragDropSession> {
    static DRAG_DROP_SESSION: OnceLock<Mutex<DragDropSession>> = OnceLock::new();
    DRAG_DROP_SESSION.get_or_init(|| Mutex::new(DragDropSession::default()))
}

pub fn reset_drag_drop_session() {
    let Ok(mut session) = drag_drop_session().lock() else {
        return;
    };
    session.dropped_this_drag = false;
}

pub fn clear_dropped_file_allowlist() {
    let Ok(mut allowlist) = dropped_file_allowlist().lock() else {
        return;
    };
    allowlist.clear();
}

pub fn clear_dropped_file_allowlist_on_drag_leave() {
    let Ok(mut session) = drag_drop_session().lock() else {
        return;
    };

    if !session.dropped_this_drag {
        if let Ok(mut allowlist) = dropped_file_allowlist().lock() {
            allowlist.clear();
        }
    }

    session.dropped_this_drag = false;
}

pub fn remember_dropped_paths(paths: &[PathBuf]) {
    let Ok(mut session) = drag_drop_session().lock() else {
        return;
    };
    session.dropped_this_drag = true;
    drop(session);

    let Ok(mut allowlist) = dropped_file_allowlist().lock() else {
        return;
    };
    let now = Instant::now();
    allowlist.purge_expired(now);

    for path in paths {
        if let Ok(canonical) = fs::canonicalize(path) {
            allowlist.insert(canonical, now);
        }
    }
}

#[cfg(test)]
pub fn clear_dropped_file_registry_for_tests() {
    clear_dropped_file_allowlist();
    reset_drag_drop_session();
}

#[cfg(test)]
fn dropped_file_allowlist_len_for_tests() -> usize {
    dropped_file_allowlist()
        .lock()
        .map(|allowlist| allowlist.len())
        .unwrap_or_default()
}

fn take_allowed_dropped_path(path: &str) -> Result<PathBuf, String> {
    let canonical =
        fs::canonicalize(path).map_err(|err| format!("Unable to read dropped path: {err}"))?;
    let mut allowlist = dropped_file_allowlist()
        .lock()
        .map_err(|_| "Unable to access dropped file registry".to_owned())?;
    allowlist.purge_expired(Instant::now());

    if allowlist.remove(&canonical) {
        Ok(canonical)
    } else {
        Err("Dropped file path is not available to this window".to_owned())
    }
}

struct ClearDroppedFileAllowlistGuard;

impl Drop for ClearDroppedFileAllowlistGuard {
    fn drop(&mut self) {
        clear_dropped_file_allowlist();
    }
}

#[tauri::command]
pub fn desktop_read_dropped_files(
    paths: Vec<String>,
) -> Result<Vec<DesktopDroppedFilePayload>, String> {
    let _clear_allowlist_guard = ClearDroppedFileAllowlistGuard;

    if paths.len() > MAX_DROPPED_FILES {
        return Err(format!(
            "Too many dropped files. Maximum is {MAX_DROPPED_FILES}."
        ));
    }

    let mut total_bytes = 0_u64;
    let mut files = Vec::with_capacity(paths.len());

    for path in paths {
        let canonical = take_allowed_dropped_path(&path)?;
        let metadata = fs::metadata(&canonical)
            .map_err(|err| format!("Unable to inspect dropped file: {err}"))?;
        if !metadata.is_file() {
            return Err("Only files can be dropped into the composer".to_owned());
        }

        let file_size = metadata.len();
        if file_size > MAX_DROPPED_FILE_BYTES {
            return Err("Dropped file is too large to attach from drag and drop".to_owned());
        }
        total_bytes = total_bytes.saturating_add(file_size);
        if total_bytes > MAX_DROPPED_TOTAL_BYTES {
            return Err("Dropped files are too large to attach together".to_owned());
        }

        let name = canonical
            .file_name()
            .and_then(|value| value.to_str())
            .map(sanitize_download_filename)
            .unwrap_or_else(|| "attachment".to_owned());

        match dropped_file_read_mode(file_size) {
            DroppedFileReadMode::Inline => {
                let bytes = fs::read(&canonical)
                    .map_err(|err| format!("Unable to read dropped file: {err}"))?;
                files.push(DesktopDroppedFilePayload {
                    name,
                    bytes: Some(bytes),
                    transfer_id: None,
                    size: None,
                });
            }
            DroppedFileReadMode::Streamed => {
                let transfer_id = register_dropped_read_session(DroppedReadSession {
                    path: canonical,
                    size: file_size,
                    created_at: Instant::now(),
                })?;
                files.push(DesktopDroppedFilePayload {
                    name,
                    bytes: None,
                    transfer_id: Some(transfer_id),
                    size: Some(file_size),
                });
            }
        }
    }

    Ok(files)
}

#[tauri::command]
pub fn desktop_read_dropped_file_chunk(
    transfer_id: String,
    offset: u64,
    length: usize,
) -> Result<Vec<u8>, String> {
    if length == 0 {
        return Err("Dropped file chunk length must be positive".to_owned());
    }
    let chunk_length = length.min(DESKTOP_FILE_IPC_CHUNK_SIZE);

    purge_stale_file_transfer_sessions(Instant::now());
    let now = Instant::now();
    let mut sessions = dropped_read_sessions()
        .lock()
        .map_err(|_| "Unable to access dropped file read sessions".to_owned())?;
    if let Some(session) = sessions.get(&transfer_id) {
        if dropped_read_session_is_stale(session, now) {
            sessions.remove(&transfer_id);
            return Err("Dropped file read transfer has expired".to_owned());
        }
    }
    let session = sessions
        .get(&transfer_id)
        .ok_or_else(|| "Dropped file read transfer is not available".to_owned())?;

    if offset >= session.size {
        return Ok(Vec::new());
    }

    let remaining = session.size.saturating_sub(offset) as usize;
    let read_length = chunk_length.min(remaining);

    let mut file = File::open(&session.path)
        .map_err(|err| format!("Unable to open dropped file for streaming: {err}"))?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|err| format!("Unable to seek dropped file for streaming: {err}"))?;

    let mut buffer = vec![0_u8; read_length];
    let read_bytes = file
        .read(&mut buffer)
        .map_err(|err| format!("Unable to read dropped file chunk: {err}"))?;
    buffer.truncate(read_bytes);

    Ok(buffer)
}

#[tauri::command]
pub fn desktop_read_dropped_file_end(transfer_id: String) -> Result<bool, String> {
    purge_stale_file_transfer_sessions(Instant::now());
    remove_dropped_read_session(&transfer_id);
    Ok(true)
}

fn show_notification_without_route_click_handler<R: Runtime>(
    app: &AppHandle<R>,
    title: &str,
    body: Option<&str>,
) -> Result<(), String> {
    let mut builder = app.notification().builder().title(title.to_owned());
    if let Some(body) = body {
        builder = builder.body(body.to_owned());
    }
    builder.show().map_err(|error| error.to_string())
}

#[cfg(target_os = "linux")]
fn show_notification_with_route_click_handler<R: Runtime>(
    app: &AppHandle<R>,
    title: &str,
    body: Option<&str>,
    route: &str,
) -> Result<(), String> {
    use notify_rust::Notification;

    let mut notification = Notification::new();
    notification.summary(title);
    if let Some(body) = body {
        notification.body(body);
    }
    notification.auto_icon();
    notification.action("default", "Open Synara");

    let handle = notification.show().map_err(|error| error.to_string())?;
    let app = app.clone();
    let route = route.to_owned();

    tauri::async_runtime::spawn(async move {
        let _ = tauri::async_runtime::spawn_blocking(move || {
            handle.wait_for_action(move |action| {
                if action == "default" {
                    if let Err(error) = navigate_main_window(&app, &route) {
                        eprintln!("failed to navigate from notification click: {error}");
                    }
                }
            });
        })
        .await;
    });

    Ok(())
}

#[cfg(target_os = "macos")]
fn configure_macos_notification_application() {
    use mac_notification_sys::set_application;

    let bundle_identifier = if tauri::is_dev() {
        "com.apple.Terminal"
    } else {
        "com.whylandcreative.synara.desktop"
    };

    if let Err(error) = set_application(bundle_identifier) {
        eprintln!("failed to configure macOS notification application: {error}");
    }
}

#[cfg(target_os = "macos")]
fn show_notification_with_route_click_handler<R: Runtime>(
    app: &AppHandle<R>,
    title: &str,
    body: Option<&str>,
    route: &str,
) -> Result<(), String> {
    use mac_notification_sys::{Notification, NotificationResponse};

    configure_macos_notification_application();

    let title = title.to_owned();
    let body = body.map(str::to_owned);
    let app = app.clone();
    let route = route.to_owned();

    tauri::async_runtime::spawn(async move {
        let app = app.clone();
        let route = route.clone();
        let response = tauri::async_runtime::spawn_blocking(move || {
            let mut notification = Notification::new();
            notification.title(&title);
            if let Some(ref body) = body {
                notification.message(body);
            }
            notification.wait_for_click(true);
            notification.send()
        })
        .await;

        if let Ok(Ok(response)) = response {
            match response {
                NotificationResponse::Click | NotificationResponse::ActionButton(_) => {
                    if let Err(error) = navigate_main_window(&app, &route) {
                        eprintln!("failed to navigate from notification click: {error}");
                    }
                }
                _ => {}
            }
        }
    });

    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn show_notification_with_route_click_handler<R: Runtime>(
    app: &AppHandle<R>,
    title: &str,
    body: Option<&str>,
    _route: &str,
) -> Result<(), String> {
    show_notification_without_route_click_handler(app, title, body)
}

fn clamp_count(value: i64) -> i64 {
    match value {
        value if value < 0 => 0,
        value if value > MAX_TRAY_COUNT => MAX_TRAY_COUNT,
        value => value,
    }
}

fn is_kde() -> bool {
    env::var("XDG_CURRENT_DESKTOP")
        .map(|value| value.to_ascii_lowercase().contains("kde"))
        .unwrap_or(false)
}

fn is_wayland() -> bool {
    if env::var("WAYLAND_DISPLAY").is_ok() {
        return true;
    }

    env::var("XDG_SESSION_TYPE")
        .map(|value| value.eq_ignore_ascii_case("wayland"))
        .unwrap_or(false)
}

fn is_kde_wayland_session() -> bool {
    is_kde() && is_wayland()
}

fn detect_session_type() -> String {
    if is_wayland() {
        return "wayland".to_owned();
    }
    if env::var("DISPLAY").is_ok() {
        return "x11".to_owned();
    }
    env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| UNKNOWN_INTEGRATION_VALUE.to_owned())
}

fn desktop_environment_label() -> String {
    if is_kde_wayland_session() {
        return "KDE Plasma Wayland".to_owned();
    }
    if is_kde() {
        return "KDE".to_owned();
    }
    env::var("XDG_CURRENT_DESKTOP")
        .map(|value| value.trim().to_owned())
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| UNKNOWN_INTEGRATION_VALUE.to_owned())
}

fn parse_os_release_field(contents: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if !line.starts_with(&prefix) {
            continue;
        }
        let value = line.trim_start_matches(&prefix).trim();
        return Some(unquote_os_release_value(value));
    }
    None
}

fn unquote_os_release_value(value: &str) -> String {
    let stripped = value.trim().trim_matches('"');
    stripped.to_owned()
}

fn detect_os_release() -> (String, String, String) {
    let default = UNKNOWN_INTEGRATION_VALUE.to_owned();
    let path = Path::new("/etc/os-release");
    if !path.exists() {
        return (default.clone(), default.clone(), default);
    }

    let Ok(contents) = fs::read_to_string(path) else {
        return (default.clone(), default.clone(), default);
    };

    let distro_id = parse_os_release_field(&contents, "ID").unwrap_or_else(|| default.clone());
    let distro_name =
        parse_os_release_field(&contents, "NAME").unwrap_or_else(|| distro_id.clone());
    let distro_version =
        parse_os_release_field(&contents, "VERSION_ID").unwrap_or_else(|| default.clone());
    (distro_id, distro_name, distro_version)
}

fn dir_has_fragment(path: &str, fragment: &str) -> bool {
    let mut entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return false,
    };

    let fragment = fragment.to_ascii_lowercase();
    entries.any(|entry| {
        let Ok(entry) = entry else {
            return false;
        };
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        name.contains(&fragment)
    })
}

fn has_media_portal_backend() -> bool {
    dir_has_fragment("/usr/share/xdg-desktop-portal/portals", "screencast")
        || dir_has_fragment("/usr/share/dbus-1/services", "screencast")
        || dir_has_fragment("/usr/share/dbus-1/services", "camera")
        || dir_has_fragment("/usr/share/xdg-desktop-portal/portals", "screenshot")
}

fn has_file_portal_backend() -> bool {
    dir_has_fragment("/usr/share/xdg-desktop-portal/portals", "file")
        || dir_has_fragment("/usr/share/dbus-1/services", "org.freedesktop.portal.files")
        || dir_has_fragment("/usr/share/dbus-1/services", "filechooser")
}

fn tray_route_labels(state: &DesktopTrayState) -> [String; 5] {
    let unread = clamp_count(state.unread_count);
    let highlights = clamp_count(state.highlight_count);
    let later = clamp_count(state.later_count);
    let notifications = clamp_count(state.notification_inbox_count);
    let do_not_disturb = state.do_not_disturb;
    let summary = format!(
        "Unread: {unread} | Highlights: {highlights} | Later: {later} | Notifications: {notifications}"
    );
    let later_label = format!("Later ({later})");
    let notifications_label = format!("Notifications ({notifications})");
    let dnd_label = if do_not_disturb {
        "Do Not Disturb: On"
    } else {
        "Do Not Disturb: Off"
    };
    let integration_label = "Desktop Integration";
    [
        summary,
        later_label,
        notifications_label,
        dnd_label.to_owned(),
        integration_label.to_owned(),
    ]
}

fn apply_tray_menu_labels<R: Runtime>(
    items: &TrayMenuItems<R>,
    state: &DesktopTrayState,
) -> tauri::Result<()> {
    let route_labels = tray_route_labels(state);
    items.later.set_text(route_labels[1].as_str())?;
    items.notifications.set_text(route_labels[2].as_str())?;
    #[cfg(target_os = "linux")]
    {
        items.unread_summary.set_text(route_labels[0].as_str())?;
        items.dnd.set_text(route_labels[3].as_str())?;
    }
    Ok(())
}

fn apply_tray_state_in_place<R: Runtime>(
    app: &AppHandle<R>,
    items: &TrayMenuItems<R>,
    state: &DesktopTrayState,
) -> Result<(), String> {
    apply_tray_menu_labels(items, state).map_err(|error| error.to_string())?;
    if let Some(tray) = app.tray_by_id(TRAY_ICON_ID) {
        tray.set_tooltip(Some(tray_tooltip(state)))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn rebuild_tray_menu<R: Runtime>(
    app: &AppHandle<R>,
    state: &DesktopTrayState,
) -> Result<(), String> {
    let Some(tray) = app.tray_by_id(TRAY_ICON_ID) else {
        return Ok(());
    };

    #[cfg(debug_assertions)]
    TRAY_MENU_REBUILD_COUNT.fetch_add(1, Ordering::Relaxed);

    let built_menu = build_tray_menu(app, state).map_err(|error| error.to_string())?;
    tray.set_menu(Some(built_menu.0))
        .map_err(|error| error.to_string())?;
    tray.set_tooltip(Some(tray_tooltip(state)))
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn apply_pending_tray_state<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let coalescer = app.state::<TrayStateCoalescer>();
    let state = {
        let mut pending = coalescer
            .pending
            .lock()
            .map_err(|error| error.to_string())?;
        pending.take()
    };
    let Some(state) = state else {
        return Ok(());
    };

    let normalized = normalize_tray_state(state);
    if let Some(items) = app.try_state::<TrayMenuItems<R>>() {
        apply_tray_state_in_place(app, &items, &normalized)?;
    } else {
        rebuild_tray_menu(app, &normalized)?;
    }

    let mut last_applied_at = coalescer
        .last_applied_at
        .lock()
        .map_err(|error| error.to_string())?;
    *last_applied_at = Some(Instant::now());
    Ok(())
}

fn schedule_tray_state_flush<R: Runtime>(app: AppHandle<R>) {
    let coalescer = app.state::<TrayStateCoalescer>();
    if coalescer.flush_scheduled.swap(true, Ordering::AcqRel) {
        return;
    }

    let delay = {
        let last_applied_at = coalescer
            .last_applied_at
            .lock()
            .ok()
            .and_then(|guard| *guard);
        let elapsed = last_applied_at
            .map(|applied_at| Instant::now().duration_since(applied_at))
            .unwrap_or(tray_state_apply_min_interval());
        tray_state_apply_min_interval()
            .checked_sub(elapsed)
            .unwrap_or(Duration::ZERO)
    };

    tauri::async_runtime::spawn(async move {
        if !delay.is_zero() {
            let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(delay)).await;
        }

        let coalescer = app.state::<TrayStateCoalescer>();
        coalescer.flush_scheduled.store(false, Ordering::Release);
        if let Err(error) = apply_pending_tray_state(&app) {
            eprintln!("failed to apply coalesced tray state: {error}");
        }
    });
}

fn queue_tray_state_update<R: Runtime>(
    app: AppHandle<R>,
    state: DesktopTrayState,
) -> Result<(), String> {
    let coalescer = app.state::<TrayStateCoalescer>();
    {
        let mut pending = coalescer
            .pending
            .lock()
            .map_err(|error| error.to_string())?;
        *pending = Some(state);
    }

    let now = Instant::now();
    let apply_now = {
        let last_applied_at = coalescer
            .last_applied_at
            .lock()
            .map_err(|error| error.to_string())?;
        should_apply_tray_state_now(*last_applied_at, now)
    };

    if apply_now {
        apply_pending_tray_state(&app)
    } else {
        schedule_tray_state_flush(app);
        Ok(())
    }
}

fn build_tray_menu<R: Runtime>(
    app: &AppHandle<R>,
    state: &DesktopTrayState,
) -> tauri::Result<(Menu<R>, TrayMenuItems<R>)> {
    let route_labels = tray_route_labels(state);

    let show = MenuItem::with_id(
        app,
        MENU_SHOW,
        "Show Synara",
        true,
        Some("CmdOrCtrl+Shift+C"),
    )?;
    let later = MenuItem::with_id(
        app,
        MENU_LATER,
        route_labels[1].as_str(),
        true,
        Some("CmdOrCtrl+Shift+L"),
    )?;
    let notifications = MenuItem::with_id(
        app,
        MENU_NOTIFICATIONS,
        route_labels[2].as_str(),
        true,
        Some("CmdOrCtrl+Shift+N"),
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let build_item = MenuItem::with_id(
        app,
        MENU_BUILD_INFO,
        build_info::menu_label(),
        false,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit Synara", true, Some("CmdOrCtrl+Q"))?;

    #[cfg(not(target_os = "linux"))]
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &later,
            &notifications,
            &separator,
            &build_item,
            &quit,
        ],
    )?;
    #[cfg(not(target_os = "linux"))]
    let items = TrayMenuItems {
        later,
        notifications,
    };

    #[cfg(target_os = "linux")]
    let unread_summary = MenuItem::with_id(
        app,
        MENU_UNREAD_SUMMARY,
        route_labels[0].as_str(),
        false,
        None::<&str>,
    )?;
    #[cfg(target_os = "linux")]
    let desktop_integration = MenuItem::with_id(
        app,
        MENU_DESKTOP_INTEGRATION,
        route_labels[4].as_str(),
        true,
        None::<&str>,
    )?;
    #[cfg(target_os = "linux")]
    let dnd = MenuItem::with_id(
        app,
        MENU_DND_TOGGLE,
        route_labels[3].as_str(),
        true,
        None::<&str>,
    )?;
    #[cfg(target_os = "linux")]
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &unread_summary,
            &later,
            &notifications,
            &desktop_integration,
            &dnd,
            &separator,
            &build_item,
            &quit,
        ],
    )?;
    #[cfg(target_os = "linux")]
    let items = TrayMenuItems {
        later,
        notifications,
        unread_summary,
        dnd,
    };

    Ok((menu, items))
}

fn tray_tooltip(state: &DesktopTrayState) -> String {
    let unread = clamp_count(state.unread_count);
    let highlights = clamp_count(state.highlight_count);
    let later = clamp_count(state.later_count);
    format!("Synara — {unread} unread ({highlights} highlights), {later} later")
}

fn sanitize_agent_action_payload(
    mut action: DesktopAgentActionPayload,
) -> Result<DesktopAgentActionPayload, String> {
    action.id = sanitize_action_text(action.id, DESKTOP_AGENT_ACTION_MAX_TEXT_CHARS);
    if action.id.is_empty() {
        return Err("Agent action payload missing action id".to_owned());
    }

    action.title = sanitize_action_text(action.title, DESKTOP_AGENT_ACTION_MAX_TEXT_CHARS);
    if action.title.is_empty() {
        return Err("Agent action payload missing title".to_owned());
    }

    if let Some(kind) = action.kind.take() {
        let normalized = kind.trim().to_lowercase();
        if !ALLOWED_AGENT_ACTION_KIND.contains(&normalized.as_str()) {
            return Err("Agent action kind is not allowed".to_owned());
        }
        action.kind = Some(normalized);
    }

    if let Some(url) = action.url.take() {
        if !desktop_url::is_safe_agent_url(&url) {
            return Err("Agent action URL must use https".to_owned());
        }
        action.url = Some(sanitize_action_text(
            url,
            DESKTOP_AGENT_ACTION_MAX_URL_CHARS,
        ));
    }

    if let Some(prompt) = action.prompt.take() {
        let sanitized = sanitize_action_text(prompt, DESKTOP_AGENT_ACTION_MAX_TEXT_CHARS);
        if !sanitized.is_empty() {
            action.prompt = Some(sanitized);
        }
    }

    if let Some(markdown) = action.markdown.take() {
        let sanitized = truncate_text(markdown, DESKTOP_AGENT_ACTION_MAX_MARKDOWN_CHARS);
        if !sanitized.is_empty() {
            action.markdown = Some(sanitized);
        }
    }

    if action.url.is_none() && action.prompt.is_none() && action.markdown.is_none() {
        return Err("Agent action payload missing runnable payload".to_owned());
    }

    Ok(action)
}

fn extract_agent_action_copy_text(action: &DesktopAgentActionPayload) -> Option<String> {
    if let Some(markdown) = action.markdown.as_deref() {
        return Some(markdown.to_owned());
    }

    if let Some(prompt) = action.prompt.as_deref() {
        return Some(prompt.to_owned());
    }

    if !action.title.is_empty() {
        return Some(action.title.clone());
    }

    None
}

fn handle_agent_action_locally<R: Runtime>(
    app: &AppHandle<R>,
    action: &DesktopAgentActionPayload,
) -> bool {
    match action.kind.as_deref() {
        Some("copy") => {
            let Some(copy_text) = extract_agent_action_copy_text(action) else {
                return false;
            };
            app.clipboard().write_text(copy_text).is_ok()
        }
        Some("open") | Some("open_url") => action
            .url
            .as_ref()
            .is_some_and(|url| app.opener().open_url(url.as_str(), None::<&str>).is_ok()),
        None => action
            .url
            .as_ref()
            .is_some_and(|url| app.opener().open_url(url.as_str(), None::<&str>).is_ok()),
        _ => false,
    }
}

fn is_supported_agent_action(action: &DesktopAgentActionPayload) -> bool {
    match &action.kind {
        Some(kind) => ALLOWED_AGENT_ACTION_KIND.contains(&kind.as_str()),
        None => action.url.is_some() || action.prompt.is_some() || action.markdown.is_some(),
    }
}

fn main_window<R: Runtime>(app: &AppHandle<R>) -> Option<WebviewWindow<R>> {
    app.get_webview_window(MAIN_WINDOW_LABEL)
}

pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = main_window(app) {
        window.show()?;
        window.unminimize()?;
        window.set_focus()?;
    }
    Ok(())
}

pub fn hide_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = main_window(app) {
        window.hide()?;
    }
    Ok(())
}

pub fn navigate_main_window<R: Runtime>(app: &AppHandle<R>, route: &str) -> tauri::Result<()> {
    show_main_window(app)?;

    if let Some(window) = main_window(app) {
        let hash = format!("#{}", route.trim_start_matches('#'));
        let hash_json = serde_json::to_string(&hash).unwrap_or_else(|_| "\"#/\"".to_owned());
        window.eval(format!("window.location.hash = {};", hash_json))?;
    }

    Ok(())
}

pub fn tray_dnd_toggle_dispatch_script() -> String {
    format!("window.dispatchEvent(new CustomEvent('{DESKTOP_TRAY_DND_TOGGLE_EVENT}'));")
}

fn emit_tray_dnd_toggle<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = main_window(app) {
        window.eval(tray_dnd_toggle_dispatch_script())?;
    }
    Ok(())
}

pub fn set_badge_count<R: Runtime>(app: &AppHandle<R>, count: Option<i64>) -> tauri::Result<()> {
    if let Some(window) = main_window(app) {
        let normalized_count = count.map(clamp_count).filter(|value| *value > 0);
        window.set_badge_count(normalized_count)?;

        #[cfg(target_os = "macos")]
        {
            window.set_badge_label(normalized_count.map(|value| value.to_string()))?;
        }
    }
    Ok(())
}

pub fn performance_capabilities() -> DesktopPerformanceCapabilities {
    DesktopPerformanceCapabilities {
        platform: std::env::consts::OS,
        app_version: build_info::app_version(),
        build_revision: build_info::revision(),
        build_branch: build_info::branch(),
        build_label: build_info::label(),
    }
}

pub fn create_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    app.manage(TrayStateCoalescer::new());

    let initial_state = DesktopTrayState {
        unread_count: 0,
        highlight_count: 0,
        later_count: 0,
        notification_inbox_count: 0,
        do_not_disturb: false,
    };
    let (menu, tray_items) = build_tray_menu(app, &initial_state)?;
    app.manage(tray_items);

    let mut builder = TrayIconBuilder::with_id("synara-tray")
        .tooltip(&tray_tooltip(&initial_state))
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(handle_menu_event);

    #[cfg(target_os = "macos")]
    {
        let tray_icon = Image::from_bytes(include_bytes!("../icons/tray-template.png"))?;
        builder = builder.icon(tray_icon).icon_as_template(true);
    }

    #[cfg(not(target_os = "macos"))]
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone()).icon_as_template(false);
    }

    builder.build(app)?;
    Ok(())
}

pub fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    let result = match event.id().as_ref() {
        MENU_SHOW => show_main_window(app),
        MENU_LATER => navigate_main_window(app, ROUTE_LATER),
        MENU_NOTIFICATIONS => navigate_main_window(app, ROUTE_NOTIFICATIONS),
        MENU_UNREAD_SUMMARY => navigate_main_window(app, ROUTE_HOME),
        MENU_DESKTOP_INTEGRATION => navigate_main_window(app, ROUTE_SETTINGS),
        MENU_DND_TOGGLE => emit_tray_dnd_toggle(app),
        MENU_BUILD_INFO => Ok(()),
        MENU_QUIT => {
            app.exit(0);
            Ok(())
        }
        _ => Ok(()),
    };

    if let Err(error) = result {
        eprintln!("failed to handle desktop menu event: {error}");
    }
}

#[tauri::command]
pub fn desktop_show(app: AppHandle) -> Result<(), String> {
    show_main_window(&app).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn desktop_hide(app: AppHandle) -> Result<(), String> {
    hide_main_window(&app).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn desktop_navigate(app: AppHandle, route: String) -> Result<(), String> {
    let route = sanitize_route(route)?;
    navigate_main_window(&app, &route).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn desktop_set_badge_count(app: AppHandle, count: i64) -> Result<(), String> {
    set_badge_count(&app, Some(clamp_count(count))).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn desktop_set_shortcuts(
    app: AppHandle,
    shortcuts: DesktopShortcutConfig,
) -> DesktopShortcutApplyResult {
    apply_desktop_shortcuts_command(&app, shortcuts)
}

pub fn desktop_bridge_supports_secure_secret_store() -> bool {
    bridge_supports_secure_secret_store(&KeyringDesktopSessionSecretStore.status())
}

#[tauri::command]
pub fn desktop_secret_store_status() -> DesktopSecretStoreStatus {
    KeyringDesktopSessionSecretStore.status()
}

#[tauri::command]
pub fn desktop_get_session() -> Result<Option<DesktopSessionEnvelope>, String> {
    desktop_get_session_from_store(&KeyringDesktopSessionSecretStore)
}

#[tauri::command]
pub fn desktop_set_session(session: DesktopSessionEnvelope) -> Result<bool, String> {
    desktop_set_session_in_store(&KeyringDesktopSessionSecretStore, session)
}

#[tauri::command]
pub fn desktop_remove_session() -> Result<bool, String> {
    desktop_remove_session_from_store(&KeyringDesktopSessionSecretStore)
}

#[tauri::command]
pub fn desktop_get_integration_status(app: AppHandle) -> DesktopIntegrationStatus {
    let (distro_id, distro_name, distro_version) = detect_os_release();
    let desktop_environment = desktop_environment_label();
    let session_type = detect_session_type();
    let tray = app
        .tray_by_id(TRAY_ICON_ID)
        .map(|_| DesktopIntegrationCheck {
            name: "Tray".to_string(),
            ready: true,
            supported: true,
            message: "Tray is available.".to_string(),
        })
        .unwrap_or_else(|| DesktopIntegrationCheck {
            name: "Tray".to_string(),
            ready: false,
            supported: false,
            message: "Tray is unavailable.".to_string(),
        });

    let notifications = app
        .notification()
        .permission_state()
        .map(|permission| permission.to_string().to_ascii_lowercase())
        .map(|permission| {
            let supported = !permission.is_empty();
            let ready = permission != "denied";
            let message = if ready {
                "Notification permission is active."
            } else {
                "Notifications are blocked by platform permission."
            };
            DesktopIntegrationCheck {
                name: "Notifications".to_string(),
                supported,
                ready,
                message: message.to_string(),
            }
        })
        .unwrap_or_else(|_| DesktopIntegrationCheck {
            name: "Notifications".to_string(),
            ready: false,
            supported: false,
            message: "Notification state could not be read.".to_string(),
        });

    let shortcut_status = desktop_shortcuts_integration_status();
    let global_shortcuts = DesktopIntegrationCheck {
        name: "Global Shortcuts".to_string(),
        supported: shortcut_status.supported,
        ready: shortcut_status.ready,
        message: shortcut_status.message,
    };

    let file_portal_available = has_file_portal_backend();
    let media_portal_available = has_media_portal_backend();
    let file_portal = DesktopIntegrationCheck {
        name: "File Portal".to_string(),
        supported: true,
        ready: file_portal_available,
        message: if file_portal_available {
            "File portal backend detected."
        } else {
            "File portal backend not detected."
        }
        .to_string(),
    };
    let media_portal = DesktopIntegrationCheck {
        name: "Media Portal".to_string(),
        supported: true,
        ready: media_portal_available,
        message: if media_portal_available {
            "Media portal backend detected."
        } else {
            "Media portal backend not detected."
        }
        .to_string(),
    };

    DesktopIntegrationStatus {
        platform: std::env::consts::OS,
        desktop_environment,
        session_type,
        distro_id,
        distro_name,
        distro_version,
        build_identity: build_info::menu_label(),
        tray,
        notifications,
        global_shortcuts,
        file_portal,
        media_portal,
    }
}

#[tauri::command]
pub fn desktop_update_tray_state(app: AppHandle, state: DesktopTrayState) -> Result<(), String> {
    if app.tray_by_id(TRAY_ICON_ID).is_some() {
        queue_tray_state_update(app, state)?;
    }
    Ok(())
}

#[tauri::command]
pub fn desktop_get_notification_permission(app: AppHandle) -> Result<String, String> {
    app.notification()
        .permission_state()
        .map(|permission| permission.to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn desktop_request_notification_permission(app: AppHandle) -> Result<String, String> {
    app.notification()
        .request_permission()
        .map(|permission| permission.to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn desktop_notify(
    app: AppHandle,
    notification: DesktopNotificationPayload,
) -> Result<bool, String> {
    let notification = sanitize_notification_payload(notification)?;

    if let Some(route) = notification.route.as_deref() {
        show_notification_with_route_click_handler(
            &app,
            &notification.title,
            notification.body.as_deref(),
            route,
        )?;
        return Ok(true);
    }

    show_notification_without_route_click_handler(
        &app,
        &notification.title,
        notification.body.as_deref(),
    )?;
    Ok(true)
}

#[tauri::command]
pub fn desktop_get_performance_capabilities() -> DesktopPerformanceCapabilities {
    performance_capabilities()
}

#[tauri::command]
pub fn desktop_agent_action(
    app: AppHandle,
    action: DesktopAgentActionPayload,
) -> Result<bool, String> {
    let action = sanitize_agent_action_payload(action).map_err(|error| error.to_string())?;
    if !is_supported_agent_action(&action) {
        return Ok(false);
    }

    if handle_agent_action_locally(&app, &action) {
        return Ok(true);
    }

    app.emit("synara://agent-action", DesktopAgentActionEvent { action })
        .map_err(|error| error.to_string())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());
    static DROPPED_FILE_ALLOWLIST_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn sanitize_action_payload_allows_https_urls() {
        let payload = sanitize_agent_action_payload(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("agent".to_owned()),
            prompt: None,
            url: Some("https://example.org/action".to_owned()),
            markdown: None,
        })
        .expect("action payload should pass");

        assert_eq!(payload.id, "abc");
        assert_eq!(payload.url.as_deref(), Some("https://example.org/action"));
    }

    #[test]
    fn sanitize_action_payload_rejects_plain_http_urls() {
        let result = sanitize_agent_action_payload(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("agent".to_owned()),
            prompt: None,
            url: Some("http://example.org/action".to_owned()),
            markdown: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn sanitize_action_payload_rejects_credentialed_urls() {
        let result = sanitize_agent_action_payload(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("agent".to_owned()),
            prompt: None,
            url: Some("https://user:pass@example.org/action".to_owned()),
            markdown: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn sanitize_action_payload_rejects_disallowed_scheme() {
        let result = sanitize_agent_action_payload(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("agent".to_owned()),
            prompt: None,
            url: Some("file:///tmp/test".to_owned()),
            markdown: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn sanitize_action_payload_rejects_unsupported_kind() {
        let result = sanitize_agent_action_payload(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("malicious".to_owned()),
            prompt: Some("Run local tool".to_owned()),
            url: None,
            markdown: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn sanitize_action_payload_requires_payload() {
        let result = sanitize_agent_action_payload(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("agent".to_owned()),
            prompt: None,
            url: None,
            markdown: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn supported_agent_action_detects_no_kind_with_url() {
        let payload = sanitize_action_payload_with_no_kind(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: None,
            prompt: None,
            url: Some("https://example.org/action".to_owned()),
            markdown: None,
        });

        assert!(is_supported_agent_action(&payload));
    }

    #[test]
    fn supported_agent_action_detects_no_kind_with_prompt() {
        let payload = sanitize_agent_action_payload(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: None,
            prompt: Some("Run the workflow".to_owned()),
            url: None,
            markdown: None,
        })
        .expect("prompt-only action should sanitize");

        assert!(is_supported_agent_action(&payload));
    }

    #[test]
    fn sanitize_action_payload_allows_urls_up_to_desktop_max_url_chars() {
        let long_path =
            "a".repeat(DESKTOP_AGENT_ACTION_MAX_URL_CHARS - "https://example.org/".len());
        let payload = sanitize_agent_action_payload(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("open".to_owned()),
            prompt: None,
            url: Some(format!("https://example.org/{long_path}")),
            markdown: None,
        })
        .expect("long https url should sanitize");

        assert_eq!(
            payload.url.as_deref().map(str::len),
            Some(DESKTOP_AGENT_ACTION_MAX_URL_CHARS)
        );
    }

    #[test]
    fn save_session_is_stale_after_transfer_ttl() {
        let session = SaveFileSession {
            temp_path: PathBuf::from("/tmp/synara-save-test"),
            filename: "test.bin".to_owned(),
            expected_size: 1,
            bytes_received: 0,
            created_at: Instant::now()
                .checked_sub(FILE_TRANSFER_SESSION_TTL + Duration::from_secs(1))
                .unwrap_or_else(Instant::now),
        };

        assert!(save_session_is_stale(&session, Instant::now()));
    }

    #[test]
    fn desktop_save_file_chunk_rejects_expired_session() {
        let temp_path = std::env::temp_dir().join(new_file_transfer_id("save-expired-test"));
        File::create(&temp_path).expect("temp save file should be created");
        let session_id = new_file_transfer_id("save");
        let mut sessions = save_file_sessions()
            .lock()
            .expect("save sessions lock should succeed");
        sessions.insert(
            session_id.clone(),
            SaveFileSession {
                temp_path: temp_path.clone(),
                filename: "expired.bin".to_owned(),
                expected_size: 1,
                bytes_received: 0,
                created_at: Instant::now()
                    .checked_sub(FILE_TRANSFER_SESSION_TTL + Duration::from_secs(1))
                    .unwrap_or_else(Instant::now),
            },
        );
        drop(sessions);

        let result = desktop_save_file_chunk(session_id.clone(), 0, vec![1]);
        assert!(result.is_err());
        let sessions = save_file_sessions()
            .lock()
            .expect("save sessions lock should succeed");
        assert!(!sessions.contains_key(&session_id));
        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn extract_copy_text_prefers_markdown() {
        let payload = sanitize_action_payload_with_no_kind(DesktopAgentActionPayload {
            id: "abc".to_owned(),
            title: "Action".to_owned(),
            kind: Some("copy".to_owned()),
            prompt: Some("Prompt".to_owned()),
            url: None,
            markdown: Some("```\nBlock\n```".to_owned()),
        });

        assert_eq!(
            extract_agent_action_copy_text(&payload),
            Some("```\nBlock\n```".to_owned())
        );
    }

    #[test]
    fn performance_capabilities_reflect_platform_support() {
        let capabilities = performance_capabilities();
        assert_eq!(capabilities.platform, std::env::consts::OS);
        assert_eq!(capabilities.app_version, env!("CARGO_PKG_VERSION"));
        assert!(!capabilities.build_revision.is_empty());
        assert!(!capabilities.build_branch.is_empty());
        assert!(capabilities.build_label.contains(capabilities.app_version));
    }

    #[test]
    fn external_url_filter_allows_user_clicked_http_https_links() {
        assert!(is_safe_external_url("https://example.org/path"));
        assert!(is_safe_external_url("http://example.org/path"));
        assert!(is_safe_external_url("http://127.0.0.1:8080"));
        assert!(is_safe_external_url("http://localhost:8080"));
        assert!(is_safe_external_url("https://192.168.1.1/"));
        assert!(is_safe_external_url(
            "https://169.254.169.254/latest/meta-data/"
        ));
        assert!(is_safe_external_url("https://metadata.google.internal/"));
        assert!(is_safe_external_url("https://app.local/"));
        assert!(is_safe_external_url("mailto:test@example.org"));
        assert!(is_safe_external_url("matrix:r/#room:example.org"));
        assert!(!is_safe_external_url("javascript:alert(1)"));
        assert!(!is_safe_external_url("file:///Users/example/.ssh/id_rsa"));
        assert!(!is_safe_external_url("https://user:pass@example.org/"));
        assert!(!is_safe_external_url("mailto:not-an-email"));
        assert!(!is_safe_external_url("matrix:"));
        assert!(!desktop_url::is_safe_agent_url("https://10.0.0.5/run"));
        assert!(desktop_url::is_safe_agent_url(
            "https://agent.example.org/run"
        ));
    }

    #[test]
    fn windows_secret_store_status_mapping_is_explicit_and_non_persistent() {
        let status = unavailable_secret_store_status(DESKTOP_SECRET_STORE_WINDOWS_UNSUPPORTED);

        assert_eq!(status.available, false);
        assert_eq!(status.backend, DESKTOP_SECRET_STORE_BACKEND_NONE);
        assert_eq!(status.can_persist_session, false);
        assert_eq!(
            status.reason,
            Some(DESKTOP_SECRET_STORE_WINDOWS_UNSUPPORTED)
        );
        assert!(!bridge_supports_secure_secret_store(&status));
    }

    #[test]
    fn sanitize_route_allows_only_internal_routes() {
        assert_eq!(
            sanitize_route("/inbox/later/".to_owned()).unwrap(),
            "/inbox/later/"
        );
        assert_eq!(
            sanitize_route("#/room/abc".to_owned()).unwrap(),
            "#/room/abc"
        );
        assert!(sanitize_route("https://example.org".to_owned()).is_err());
        assert!(sanitize_route("room/abc".to_owned()).is_err());
    }

    #[test]
    fn sanitize_notification_payload_rejects_empty_title() {
        let result = sanitize_notification_payload(DesktopNotificationPayload {
            title: "  ".to_owned(),
            body: Some("Body".to_owned()),
            route: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn sanitize_notification_payload_truncates_body() {
        let payload = sanitize_notification_payload(DesktopNotificationPayload {
            title: "Reminder".to_owned(),
            body: Some("a".repeat(DESKTOP_NOTIFICATION_MAX_BODY_CHARS + 10)),
            route: Some("/inbox/".to_owned()),
        })
        .expect("notification payload should pass");

        assert_eq!(
            payload.body.unwrap().chars().count(),
            DESKTOP_NOTIFICATION_MAX_BODY_CHARS
        );
    }

    #[test]
    fn sanitize_notification_route_allows_only_internal_routes() {
        assert_eq!(
            sanitize_notification_route("/inbox/later/".to_owned()).unwrap(),
            "/inbox/later/"
        );
        assert_eq!(
            sanitize_notification_route("#/room/abc".to_owned()).unwrap(),
            "#/room/abc"
        );
        let notification = sanitize_notification_payload(DesktopNotificationPayload {
            title: "Later".to_owned(),
            body: Some("Reminder".to_owned()),
            route: Some("/inbox/later/".to_owned()),
        })
        .expect("notification payload should sanitize");
        let route = notification.route.expect("route should be present");
        assert_eq!(sanitize_route(route.clone()).unwrap(), route);
        assert!(sanitize_notification_route("https://example.org".to_owned()).is_err());
        assert!(sanitize_notification_route("room/abc".to_owned()).is_err());
    }

    #[test]
    fn sanitize_notification_payload_accepts_safe_route() {
        let payload = sanitize_notification_payload(DesktopNotificationPayload {
            title: "Reminder".to_owned(),
            body: Some("body".to_owned()),
            route: Some("/inbox/notifications/".to_owned()),
        })
        .expect("notification payload should pass");
        assert_eq!(payload.route, Some("/inbox/notifications/".to_string()));
    }

    #[test]
    fn sanitize_notification_payload_rejects_unsafe_route() {
        let result = sanitize_notification_payload(DesktopNotificationPayload {
            title: "Reminder".to_owned(),
            body: Some("body".to_owned()),
            route: Some("https://evil.example.com".to_owned()),
        });

        assert!(result.is_err());
    }

    #[test]
    fn parse_os_release_detects_cachyos_metadata() {
        let data = r#"
ID=cachyos
NAME="CachyOS"
VERSION_ID=24
"#;

        assert_eq!(
            parse_os_release_field(data, "ID").unwrap_or_else(|| "".to_owned()),
            "cachyos"
        );
        assert_eq!(
            parse_os_release_field(data, "NAME").unwrap_or_else(|| "".to_owned()),
            "CachyOS"
        );
        assert_eq!(
            parse_os_release_field(data, "VERSION_ID").unwrap_or_else(|| "".to_owned()),
            "24"
        );
    }

    #[test]
    fn detect_integration_environment_falls_back_for_absent_values() {
        let _guard = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let original_desktop = std::env::var("XDG_CURRENT_DESKTOP").ok();
        let original_session_type = std::env::var("XDG_SESSION_TYPE").ok();
        let original_display = std::env::var("DISPLAY").ok();
        let original_wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
        std::env::remove_var("XDG_CURRENT_DESKTOP");
        std::env::remove_var("XDG_SESSION_TYPE");
        std::env::remove_var("DISPLAY");
        std::env::remove_var("WAYLAND_DISPLAY");

        assert_eq!(
            desktop_environment_label(),
            UNKNOWN_INTEGRATION_VALUE.to_owned()
        );
        assert_eq!(detect_session_type(), UNKNOWN_INTEGRATION_VALUE.to_owned());

        if let Some(value) = original_desktop {
            std::env::set_var("XDG_CURRENT_DESKTOP", value);
        }
        if let Some(value) = original_session_type {
            std::env::set_var("XDG_SESSION_TYPE", value);
        }
        if let Some(value) = original_display {
            std::env::set_var("DISPLAY", value);
        }
        if let Some(value) = original_wayland_display {
            std::env::set_var("WAYLAND_DISPLAY", value);
        }
    }

    #[test]
    fn detect_cachyos_like_desktop_is_kde_wayland_when_flags_match() {
        let _guard = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let original_desktop = std::env::var("XDG_CURRENT_DESKTOP").ok();
        let original_wayland = std::env::var("WAYLAND_DISPLAY").ok();
        std::env::set_var("XDG_CURRENT_DESKTOP", "KDE");
        std::env::set_var("WAYLAND_DISPLAY", "wayland-0");

        assert!(is_kde_wayland_session());
        assert_eq!(desktop_environment_label(), "KDE Plasma Wayland");
        assert_eq!(detect_session_type(), "wayland");

        if let Some(value) = original_desktop {
            std::env::set_var("XDG_CURRENT_DESKTOP", value);
        } else {
            std::env::remove_var("XDG_CURRENT_DESKTOP");
        }
        if let Some(value) = original_wayland {
            std::env::set_var("WAYLAND_DISPLAY", value);
        } else {
            std::env::remove_var("WAYLAND_DISPLAY");
        }
    }

    #[test]
    fn tray_state_apply_interval_allows_first_update_and_blocks_rapid_rebuilds() {
        let interval = tray_state_apply_min_interval();
        let started_at = Instant::now();
        assert!(should_apply_tray_state_now(None, started_at));
        assert!(!should_apply_tray_state_now(
            Some(started_at),
            started_at + Duration::from_millis(100)
        ));
        assert!(should_apply_tray_state_now(
            Some(started_at),
            started_at + interval
        ));
    }

    #[test]
    fn normalize_tray_state_clamps_all_count_fields() {
        let normalized = normalize_tray_state(DesktopTrayState {
            unread_count: -3,
            highlight_count: 12_345,
            later_count: 4,
            notification_inbox_count: -1,
            do_not_disturb: true,
        });

        assert_eq!(normalized.unread_count, 0);
        assert_eq!(normalized.highlight_count, 9_999);
        assert_eq!(normalized.later_count, 4);
        assert_eq!(normalized.notification_inbox_count, 0);
        assert!(normalized.do_not_disturb);
    }

    #[test]
    fn badge_count_uses_same_clamp_as_tray_state() {
        assert_eq!(clamp_count(50_000), 9_999);
        assert_eq!(clamp_count(-3), 0);
    }

    #[test]
    fn tray_state_fields_are_clamped() {
        assert_eq!(clamp_count(-1), 0);
        assert_eq!(clamp_count(15_000), 9_999);
        assert_eq!(clamp_count(23), 23);

        let labels = tray_route_labels(&DesktopTrayState {
            unread_count: -5,
            highlight_count: 12_000,
            later_count: 3,
            notification_inbox_count: -9,
            do_not_disturb: true,
        });
        assert!(labels[0].contains("Unread: 0"));
        assert!(labels[0].contains("Highlights: 9999"));
        assert!(labels[0].contains("Later: 3"));
        assert!(labels[0].contains("Notifications: 0"));
    }

    #[test]
    fn tray_route_labels_reflect_do_not_disturb_state() {
        let on = tray_route_labels(&DesktopTrayState {
            unread_count: 0,
            highlight_count: 0,
            later_count: 0,
            notification_inbox_count: 0,
            do_not_disturb: true,
        });
        let off = tray_route_labels(&DesktopTrayState {
            unread_count: 0,
            highlight_count: 0,
            later_count: 0,
            notification_inbox_count: 0,
            do_not_disturb: false,
        });

        assert_eq!(on[3], "Do Not Disturb: On");
        assert_eq!(off[3], "Do Not Disturb: Off");
    }

    #[test]
    fn tray_dnd_toggle_dispatch_script_emits_custom_event() {
        assert_eq!(
            tray_dnd_toggle_dispatch_script(),
            "window.dispatchEvent(new CustomEvent('synara-tray-dnd-toggle'));"
        );
    }

    #[test]
    fn should_stream_file_ipc_uses_eight_mebibyte_threshold() {
        assert!(!should_stream_file_ipc(
            crate::desktop_file_transfer::DESKTOP_FILE_IPC_INLINE_THRESHOLD as u64
        ));
        assert!(should_stream_file_ipc(
            crate::desktop_file_transfer::DESKTOP_FILE_IPC_INLINE_THRESHOLD as u64 + 1
        ));
    }

    #[test]
    fn dropped_file_read_mode_selects_inline_or_streamed_transfer() {
        assert_eq!(
            dropped_file_read_mode(
                crate::desktop_file_transfer::DESKTOP_FILE_IPC_INLINE_THRESHOLD as u64
            ),
            DroppedFileReadMode::Inline
        );
        assert_eq!(
            dropped_file_read_mode(
                crate::desktop_file_transfer::DESKTOP_FILE_IPC_INLINE_THRESHOLD as u64 + 1
            ),
            DroppedFileReadMode::Streamed
        );
    }

    #[test]
    fn desktop_save_file_rejects_inline_payload_over_threshold() {
        let oversized =
            vec![0_u8; crate::desktop_file_transfer::DESKTOP_FILE_IPC_INLINE_THRESHOLD + 1];
        let result = desktop_save_file(DesktopSaveFilePayload {
            filename: "large.bin".to_owned(),
            bytes: oversized,
        });

        assert!(result.is_err());
        assert!(result
            .expect_err("inline save should fail")
            .contains("streaming save commands"));
    }

    #[test]
    fn desktop_read_dropped_files_rejects_unauthorized_path() {
        let _guard = DROPPED_FILE_ALLOWLIST_TEST_LOCK
            .lock()
            .expect("drop allowlist test lock should not be poisoned");
        clear_dropped_file_registry_for_tests();
        let result = desktop_read_dropped_files(vec!["/etc/passwd".to_owned()]);

        assert!(result.is_err());
        assert!(result
            .expect_err("unauthorized path should fail")
            .contains("not available"));
    }

    #[test]
    fn streaming_save_round_trip_writes_expected_bytes_without_inline_buffer() {
        let _guard = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let original_home = std::env::var("HOME").ok();
        let temp_home = std::env::temp_dir().join(new_file_transfer_id("save-home-test"));
        fs::create_dir_all(&temp_home).expect("temp home should be created");
        std::env::set_var("HOME", &temp_home);

        let total_size =
            (crate::desktop_file_transfer::DESKTOP_FILE_IPC_INLINE_THRESHOLD + 1) as u64;
        let begin = desktop_save_file_begin("streamed.bin".to_owned(), total_size)
            .expect("streaming save should begin");
        let chunk_size = DESKTOP_FILE_IPC_CHUNK_SIZE as u64;
        let mut offset = 0_u64;
        while offset < total_size {
            let remaining = total_size - offset;
            let length = chunk_size.min(remaining) as usize;
            let bytes = vec![((offset % 251) + 1) as u8; length];
            desktop_save_file_chunk(begin.session_id.clone(), offset, bytes)
                .expect("chunk should be accepted");
            offset += length as u64;
        }

        let saved_path =
            desktop_save_file_end(begin.session_id).expect("streaming save should finalize");
        let saved_bytes = fs::read(&saved_path).expect("saved file should be readable");
        assert_eq!(saved_bytes.len(), total_size as usize);
        assert_eq!(saved_bytes[0], 1);
        assert_eq!(
            saved_bytes[crate::desktop_file_transfer::DESKTOP_FILE_IPC_INLINE_THRESHOLD],
            ((crate::desktop_file_transfer::DESKTOP_FILE_IPC_INLINE_THRESHOLD as u64 % 251) + 1)
                as u8
        );

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
        let _ = fs::remove_dir_all(temp_home);
    }

    #[test]
    fn streaming_dropped_file_read_returns_chunks_without_loading_entire_file() {
        let _guard = DROPPED_FILE_ALLOWLIST_TEST_LOCK
            .lock()
            .expect("drop allowlist test lock should not be poisoned");
        clear_dropped_file_registry_for_tests();
        let temp_dir = std::env::temp_dir().join(new_file_transfer_id("drop-read-test"));
        fs::create_dir_all(&temp_dir).expect("temp drop dir should be created");
        let file_path = temp_dir.join("streamed-drop.bin");
        let total_size =
            (crate::desktop_file_transfer::DESKTOP_FILE_IPC_INLINE_THRESHOLD + 512) as u64;
        let payload = vec![9_u8; total_size as usize];
        fs::write(&file_path, &payload).expect("dropped file fixture should be written");

        remember_dropped_paths(&[file_path.clone()]);
        let descriptors =
            desktop_read_dropped_files(vec![file_path.to_string_lossy().into_owned()])
                .expect("dropped file metadata should be returned");
        assert_eq!(descriptors.len(), 1);
        assert!(descriptors[0].bytes.is_none());
        let transfer_id = descriptors[0]
            .transfer_id
            .clone()
            .expect("streamed transfer id should be present");
        assert_eq!(descriptors[0].size, Some(total_size));

        let first_chunk =
            desktop_read_dropped_file_chunk(transfer_id.clone(), 0, DESKTOP_FILE_IPC_CHUNK_SIZE)
                .expect("first chunk should be readable");
        assert_eq!(first_chunk.len(), DESKTOP_FILE_IPC_CHUNK_SIZE);
        assert!(first_chunk.iter().all(|byte| *byte == 9));

        let second_chunk = desktop_read_dropped_file_chunk(
            transfer_id.clone(),
            crate::desktop_file_transfer::DESKTOP_FILE_IPC_INLINE_THRESHOLD as u64,
            DESKTOP_FILE_IPC_CHUNK_SIZE,
        )
        .expect("second chunk should be readable");
        assert_eq!(second_chunk.len(), 512);
        assert!(second_chunk.iter().all(|byte| *byte == 9));

        assert_eq!(
            desktop_read_dropped_file_end(transfer_id).expect("transfer should end"),
            true
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    fn write_temp_drop_fixture(name: &str, contents: &[u8]) -> (PathBuf, PathBuf) {
        let temp_dir = std::env::temp_dir().join(new_file_transfer_id("drop-allowlist-test"));
        fs::create_dir_all(&temp_dir).expect("temp drop dir should be created");
        let file_path = temp_dir.join(name);
        fs::write(&file_path, contents).expect("drop fixture should be written");
        (temp_dir, file_path)
    }

    #[test]
    fn dropped_file_allowlist_clears_on_drag_leave_without_drop() {
        let _guard = DROPPED_FILE_ALLOWLIST_TEST_LOCK
            .lock()
            .expect("drop allowlist test lock should not be poisoned");
        clear_dropped_file_registry_for_tests();
        let (temp_dir, file_path) = write_temp_drop_fixture("leave-clear.txt", b"stale");

        remember_dropped_paths(&[file_path]);
        assert_eq!(dropped_file_allowlist_len_for_tests(), 1);

        reset_drag_drop_session();
        clear_dropped_file_allowlist_on_drag_leave();
        assert_eq!(dropped_file_allowlist_len_for_tests(), 0);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn dropped_file_allowlist_preserves_paths_after_drop_on_drag_leave() {
        let _guard = DROPPED_FILE_ALLOWLIST_TEST_LOCK
            .lock()
            .expect("drop allowlist test lock should not be poisoned");
        clear_dropped_file_registry_for_tests();
        let (temp_dir, file_path) = write_temp_drop_fixture("leave-keep.txt", b"fresh");

        reset_drag_drop_session();
        remember_dropped_paths(&[file_path.clone()]);
        clear_dropped_file_allowlist_on_drag_leave();
        assert_eq!(dropped_file_allowlist_len_for_tests(), 1);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn dropped_file_allowlist_caps_at_max_entries() {
        let _guard = DROPPED_FILE_ALLOWLIST_TEST_LOCK
            .lock()
            .expect("drop allowlist test lock should not be poisoned");
        clear_dropped_file_registry_for_tests();
        let temp_dir = std::env::temp_dir().join(new_file_transfer_id("drop-cap-test"));
        fs::create_dir_all(&temp_dir).expect("temp drop dir should be created");

        let mut paths = Vec::with_capacity(300);
        for index in 0..300 {
            let file_path = temp_dir.join(format!("drop-{index}.txt"));
            fs::write(&file_path, format!("payload-{index}"))
                .expect("drop fixture should be written");
            paths.push(file_path);
        }

        remember_dropped_paths(&paths);
        assert!(dropped_file_allowlist_len_for_tests() <= MAX_DROPPED_FILE_ALLOWLIST);
        assert_eq!(
            dropped_file_allowlist_len_for_tests(),
            MAX_DROPPED_FILE_ALLOWLIST
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn dropped_file_allowlist_expires_stale_entries() {
        use std::thread::sleep;

        let _guard = DROPPED_FILE_ALLOWLIST_TEST_LOCK
            .lock()
            .expect("drop allowlist test lock should not be poisoned");
        clear_dropped_file_registry_for_tests();
        let (temp_dir, file_path) = write_temp_drop_fixture("ttl-expire.txt", b"expires");

        remember_dropped_paths(&[file_path.clone()]);
        sleep(Duration::from_millis(10));

        let result = desktop_read_dropped_files(vec![file_path.to_string_lossy().into_owned()]);
        assert!(result.is_err());
        assert!(result
            .expect_err("expired allowlist entry should be rejected")
            .contains("not available"));
        assert_eq!(dropped_file_allowlist_len_for_tests(), 0);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn desktop_read_dropped_files_clears_allowlist_after_read() {
        let _guard = DROPPED_FILE_ALLOWLIST_TEST_LOCK
            .lock()
            .expect("drop allowlist test lock should not be poisoned");
        clear_dropped_file_registry_for_tests();
        let (temp_dir, file_path) = write_temp_drop_fixture("consume.txt", b"payload");

        remember_dropped_paths(&[file_path.clone()]);
        let files = desktop_read_dropped_files(vec![file_path.to_string_lossy().into_owned()])
            .expect("authorized dropped file should be readable");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "consume.txt");
        assert_eq!(files[0].bytes.as_deref(), Some(b"payload".as_slice()));
        assert_eq!(dropped_file_allowlist_len_for_tests(), 0);

        let _ = fs::remove_dir_all(temp_dir);
    }

    fn sanitize_action_payload_with_no_kind(
        action: DesktopAgentActionPayload,
    ) -> DesktopAgentActionPayload {
        sanitize_agent_action_payload(action).expect("action payload should pass")
    }
}
