use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, Runtime, WebviewWindow};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

use crate::build_info;
use crate::desktop_file_transfer::{
    downloads_dir, dropped_file_read_mode, new_file_transfer_id, sanitize_download_filename,
    should_stream_file_ipc, unique_download_path, DroppedFileReadMode, DESKTOP_FILE_IPC_CHUNK_SIZE,
};
use crate::desktop_sanitize::sanitize_route;
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
use crate::desktop_tray::{self, DesktopTrayState};
use crate::desktop_url;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

pub const MAIN_WINDOW_LABEL: &str = "main";

pub(crate) const ROUTE_HOME: &str = "/";
pub(crate) const ROUTE_LATER: &str = "/inbox/later/";
pub(crate) const ROUTE_NOTIFICATIONS: &str = "/inbox/notifications/";
pub(crate) const ROUTE_SETTINGS: &str = "/settings/";

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

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopPerformanceCapabilities {
    platform: &'static str,
    app_version: &'static str,
    build_revision: &'static str,
    build_branch: &'static str,
    build_label: String,
}

const UNKNOWN_INTEGRATION_VALUE: &str = "unknown";

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

pub fn performance_capabilities() -> DesktopPerformanceCapabilities {
    DesktopPerformanceCapabilities {
        platform: std::env::consts::OS,
        app_version: build_info::app_version(),
        build_revision: build_info::revision(),
        build_branch: build_info::branch(),
        build_label: build_info::label(),
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
    desktop_tray::set_badge_count(&app, Some(count)).map_err(|error| error.to_string())
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
    let tray = if desktop_tray::tray_is_available(&app) {
        DesktopIntegrationCheck {
            name: "Tray".to_string(),
            ready: true,
            supported: true,
            message: "Tray is available.".to_string(),
        }
    } else {
        DesktopIntegrationCheck {
            name: "Tray".to_string(),
            ready: false,
            supported: false,
            message: "Tray is unavailable.".to_string(),
        }
    };

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
pub fn desktop_update_tray_state(app: AppHandle, state: DesktopTrayState) -> Result<bool, String> {
    desktop_tray::update_tray_state(app, state)
}

#[tauri::command]
pub fn desktop_get_performance_capabilities() -> DesktopPerformanceCapabilities {
    performance_capabilities()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());
    static DROPPED_FILE_ALLOWLIST_TEST_LOCK: Mutex<()> = Mutex::new(());

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
}
