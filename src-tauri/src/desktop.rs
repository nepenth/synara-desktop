use keyring::{Entry, Error as KeyringError};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
#[cfg(target_os = "macos")]
use tauri::image::Image;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindow};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;
use url::Url;

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::build_info;

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
pub const DESKTOP_FILE_IPC_INLINE_THRESHOLD: usize = 8 * 1024 * 1024;
pub const DESKTOP_FILE_IPC_CHUNK_SIZE: usize = 1024 * 1024;
const MAX_ACTIVE_FILE_TRANSFERS: usize = 16;

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
pub struct DesktopShortcutConfig {
    pub show: String,
    pub later: String,
    pub notifications: String,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DroppedFileReadMode {
    Inline,
    Streamed,
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
}

struct DroppedReadSession {
    path: PathBuf,
    size: u64,
}

#[derive(Clone, serde::Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopShortcutApplyResult {
    pub success: bool,
    pub state: DesktopShortcutApplyState,
    pub message: String,
    pub fallback_command: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum DesktopShortcutApplyState {
    Active,
    PermissionNeeded,
    Unsupported,
    Unknown,
    Failed,
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

static LAST_SHORTCUT_APPLY_STATE: OnceLock<Mutex<Option<DesktopShortcutApplyState>>> =
    OnceLock::new();
static LAST_ACTIVE_SHORTCUT_CONFIG: OnceLock<Mutex<Option<DesktopShortcutConfig>>> = OnceLock::new();

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

#[derive(Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSecretStoreStatus {
    pub available: bool,
    pub backend: &'static str,
    pub can_persist_session: bool,
    pub reason: Option<&'static str>,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSessionEnvelope {
    base_url: String,
    user_id: String,
    device_id: String,
    access_token: String,
    refresh_token: Option<String>,
    expires_in_ms: Option<u64>,
    stored_at_ms: Option<u64>,
}

const DESKTOP_AGENT_ACTION_MAX_TEXT_CHARS: usize = 1024;
const DESKTOP_AGENT_ACTION_MAX_MARKDOWN_CHARS: usize = 16_384;
const DESKTOP_NOTIFICATION_MAX_TITLE_CHARS: usize = 120;
const DESKTOP_NOTIFICATION_MAX_BODY_CHARS: usize = 500;
const DESKTOP_SESSION_MAX_BASE_URL_CHARS: usize = 2_048;
const DESKTOP_SESSION_MAX_ID_CHARS: usize = 512;
const DESKTOP_SESSION_MAX_TOKEN_CHARS: usize = 8_192;
const DESKTOP_SESSION_CREDENTIAL_SERVICE: &str = "com.whylandcreative.synara.desktop";
const DESKTOP_SESSION_LEGACY_CREDENTIAL_SERVICE: &str = "app.synara.desktop";
const DESKTOP_SESSION_CREDENTIAL_ACCOUNT: &str = "matrix-session";
#[cfg(target_os = "macos")]
const DESKTOP_SESSION_KEYCHAIN_PROBE_ACCOUNT: &str = "matrix-session-probe";
const DESKTOP_SECRET_STORE_BACKEND_NONE: &str = "none";
#[cfg(any(target_os = "macos", test))]
const DESKTOP_SECRET_STORE_BACKEND_MACOS_KEYCHAIN: &str = "macos-keychain";
#[allow(dead_code)]
const DESKTOP_SECRET_STORE_BACKEND_LINUX_SECRET_SERVICE: &str = "linux-secret-service";
#[allow(dead_code)]
const DESKTOP_SECRET_STORE_BACKEND_LINUX_KEYUTILS: &str = "linux-keyutils";
#[allow(dead_code)]
const DESKTOP_SECRET_STORE_NOT_CONFIGURED: &str = "secure-secret-store-not-configured";
#[allow(dead_code)]
const DESKTOP_SECRET_STORE_UNSUPPORTED_PLATFORM: &str = "secure-secret-store-unsupported-platform";
#[allow(dead_code)]
const DESKTOP_SECRET_STORE_WINDOWS_UNSUPPORTED: &str = "windows-native-session-store-unsupported";
#[allow(dead_code)]
const DESKTOP_SECRET_STORE_SESSION_SCOPED: &str = "linux-keyutils-session-scoped";
#[allow(dead_code)]
const DESKTOP_SECRET_STORE_LINUX_UNAVAILABLE: &str = "linux-secret-store-unavailable";
#[cfg(any(target_os = "macos", test))]
const DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_LOCKED: &str = "macos-keychain-locked";
#[cfg(any(target_os = "macos", test))]
const DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_ACCESS_DENIED: &str = "macos-keychain-access-denied";
#[cfg(any(target_os = "macos", test))]
const DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_UNAVAILABLE: &str = "macos-keychain-unavailable";
#[cfg(target_os = "linux")]
const DESKTOP_SECRET_STORE_SECRET_SERVICE_PROBE_SERVICE: &str =
    "com.whylandcreative.synara.desktop.secret-service-probe";
#[cfg(target_os = "linux")]
const DESKTOP_SECRET_STORE_SECRET_SERVICE_PROBE_ACCOUNT: &str = "availability-probe";
#[cfg(target_os = "linux")]
const DESKTOP_SECRET_STORE_SECRET_SERVICE_PROBE_SECRET: &str =
    "synara-secret-service-availability-probe";
#[cfg(target_os = "linux")]
const DESKTOP_SECRET_STORE_KEYUTILS_PROBE_SERVICE: &str =
    "com.whylandcreative.synara.desktop.keyutils-probe";
#[cfg(target_os = "linux")]
const DESKTOP_SECRET_STORE_KEYUTILS_PROBE_ACCOUNT: &str = "availability-probe";
#[cfg(target_os = "linux")]
const DESKTOP_SECRET_STORE_KEYUTILS_PROBE_SECRET: &str = "synara-keyutils-availability-probe";
const DESKTOP_SECRET_STORE_OPERATION_LOCKED: &str = "desktop-secret-store-locked";
const DESKTOP_SECRET_STORE_OPERATION_UNAVAILABLE: &str = "desktop-secret-store-unavailable";
const DESKTOP_SECRET_STORE_OPERATION_DENIED: &str = "desktop-secret-store-denied";
#[cfg(target_os = "linux")]
const DESKTOP_SECRET_STORE_PROBE_TIMEOUT: Duration = Duration::from_secs(2);
const DESKTOP_STORED_SESSION_INVALID: &str = "desktop-stored-session-invalid";
const MAX_DESKTOP_ROUTE_CHARS: usize = 2_048;
const ALLOWED_SHORTCUT_LEN: usize = 128;
const UNKNOWN_INTEGRATION_VALUE: &str = "unknown";
const MAX_TRAY_COUNT: i64 = 9_999;
const SESSION_EXPIRY_CLOCK_SKEW_TOLERANCE_MS: u64 = 60_000;
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

fn trim_shortcut(value: String) -> String {
    value.trim().replace(' ', "")
}

fn normalize_shortcut(shortcut: &str) -> String {
    trim_shortcut(shortcut.to_owned())
}

fn parse_shortcut(shortcut: &str) -> Result<Shortcut, String> {
    shortcut
        .parse::<Shortcut>()
        .map_err(|error| format!("Failed to parse shortcut '{shortcut}': {error}"))
}

fn validate_shortcuts(shortcuts: &DesktopShortcutConfig) -> Result<DesktopShortcutConfig, String> {
    let show = normalize_shortcut(&shortcuts.show);
    let later = normalize_shortcut(&shortcuts.later);
    let notifications = normalize_shortcut(&shortcuts.notifications);
    if show.is_empty() || later.is_empty() || notifications.is_empty() {
        return Err("Shortcut values cannot be empty".to_string());
    }
    if show.len() > ALLOWED_SHORTCUT_LEN
        || later.len() > ALLOWED_SHORTCUT_LEN
        || notifications.len() > ALLOWED_SHORTCUT_LEN
    {
        return Err("Shortcut values are too long".to_string());
    }

    let parsed_show = parse_shortcut(&show)?;
    let parsed_later = parse_shortcut(&later)?;
    let parsed_notifications = parse_shortcut(&notifications)?;
    if parsed_show == parsed_later
        || parsed_show == parsed_notifications
        || parsed_later == parsed_notifications
    {
        return Err("Shortcut values must be unique".to_string());
    }

    Ok(DesktopShortcutConfig {
        show,
        later,
        notifications,
    })
}

fn truncate_text(value: String, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn sanitize_action_text(value: String, max_chars: usize) -> String {
    truncate_text(value.trim().to_string(), max_chars)
}

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

fn sanitize_required_session_field(
    value: String,
    field_name: &'static str,
    max_chars: usize,
) -> Result<String, String> {
    let sanitized = value.trim().to_owned();
    if sanitized.is_empty() {
        return Err(format!("Session {field_name} cannot be empty"));
    }
    if sanitized.chars().count() > max_chars {
        return Err(format!("Session {field_name} is too long"));
    }
    Ok(sanitized)
}

fn sanitize_optional_session_field(
    value: Option<String>,
    field_name: &'static str,
    max_chars: usize,
) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let sanitized = value.trim().to_owned();
    if sanitized.is_empty() {
        return Ok(None);
    }
    if sanitized.chars().count() > max_chars {
        return Err(format!("Session {field_name} is too long"));
    }
    Ok(Some(sanitized))
}

fn is_loopback_session_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1"
}

fn is_allowed_session_base_url(value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };

    if url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() {
        return false;
    }

    match url.scheme() {
        "https" => true,
        "http" => url
            .host_str()
            .map(is_loopback_session_host)
            .unwrap_or(false),
        _ => false,
    }
}

fn sanitize_session_envelope(
    session: DesktopSessionEnvelope,
) -> Result<DesktopSessionEnvelope, String> {
    let base_url = sanitize_required_session_field(
        session.base_url,
        "baseUrl",
        DESKTOP_SESSION_MAX_BASE_URL_CHARS,
    )?;
    if !is_allowed_session_base_url(&base_url) {
        return Err(
            "Session baseUrl must be an HTTPS URL or a loopback development URL".to_owned(),
        );
    }

    let user_id =
        sanitize_required_session_field(session.user_id, "userId", DESKTOP_SESSION_MAX_ID_CHARS)?;
    let device_id = sanitize_required_session_field(
        session.device_id,
        "deviceId",
        DESKTOP_SESSION_MAX_ID_CHARS,
    )?;
    let access_token = sanitize_required_session_field(
        session.access_token,
        "accessToken",
        DESKTOP_SESSION_MAX_TOKEN_CHARS,
    )?;
    let refresh_token = sanitize_optional_session_field(
        session.refresh_token,
        "refreshToken",
        DESKTOP_SESSION_MAX_TOKEN_CHARS,
    )?;

    Ok(DesktopSessionEnvelope {
        base_url,
        user_id,
        device_id,
        access_token,
        refresh_token,
        expires_in_ms: session.expires_in_ms,
        stored_at_ms: session.stored_at_ms,
    })
}

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn session_envelope_is_expired(session: &DesktopSessionEnvelope, now_ms: u64) -> bool {
    let Some(expires_in_ms) = session.expires_in_ms else {
        return false;
    };
    let Some(stored_at_ms) = session.stored_at_ms else {
        return false;
    };

    now_ms
        > stored_at_ms
            .saturating_add(expires_in_ms)
            .saturating_add(SESSION_EXPIRY_CLOCK_SKEW_TOLERANCE_MS)
}

trait DesktopSessionSecretStore {
    fn status(&self) -> DesktopSecretStoreStatus;
    fn get_secret(&self) -> Result<Option<String>, String>;
    fn set_secret(&self, secret: &str) -> Result<bool, String>;
    fn remove_secret(&self) -> Result<bool, String>;
}

struct KeyringDesktopSessionSecretStore;

impl KeyringDesktopSessionSecretStore {
    fn session_entry(&self) -> Result<Entry, String> {
        self.session_entry_for_service(DESKTOP_SESSION_CREDENTIAL_SERVICE)
    }

    fn legacy_session_entry(&self) -> Result<Entry, String> {
        self.session_entry_for_service(DESKTOP_SESSION_LEGACY_CREDENTIAL_SERVICE)
    }

    fn session_entry_for_service(&self, service: &str) -> Result<Entry, String> {
        Entry::new(service, DESKTOP_SESSION_CREDENTIAL_ACCOUNT)
            .map_err(|error| map_keyring_error("create-entry", error))
    }
}

impl DesktopSessionSecretStore for KeyringDesktopSessionSecretStore {
    fn status(&self) -> DesktopSecretStoreStatus {
        platform_secret_store_status()
    }

    fn get_secret(&self) -> Result<Option<String>, String> {
        if !self.status().can_persist_session {
            return Ok(None);
        }

        match self.session_entry()?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(KeyringError::NoEntry) => match self.legacy_session_entry()?.get_password() {
                Ok(secret) => {
                    self.session_entry()?
                        .set_password(&secret)
                        .map_err(|error| map_keyring_error("migrate-session", error))?;
                    let _ = self.legacy_session_entry()?.delete_credential();
                    Ok(Some(secret))
                }
                Err(KeyringError::NoEntry) => Ok(None),
                Err(error) => Err(map_keyring_error("read-legacy-session", error)),
            },
            Err(error) => Err(map_keyring_error("read-session", error)),
        }
    }

    fn set_secret(&self, secret: &str) -> Result<bool, String> {
        if !self.status().can_persist_session {
            return Ok(false);
        }

        self.session_entry()?
            .set_password(secret)
            .map_err(|error| map_keyring_error("write-session", error))?;
        Ok(true)
    }

    fn remove_secret(&self) -> Result<bool, String> {
        if !self.status().can_persist_session {
            return Ok(false);
        }

        let legacy_removed = match self.legacy_session_entry()?.delete_credential() {
            Ok(()) => true,
            Err(KeyringError::NoEntry) => false,
            Err(error) => return Err(map_keyring_error("remove-legacy-session", error)),
        };

        match self.session_entry()?.delete_credential() {
            Ok(()) => Ok(true),
            Err(KeyringError::NoEntry) => Ok(legacy_removed),
            Err(error) => Err(map_keyring_error("remove-session", error)),
        }
    }
}

#[cfg(any(target_os = "macos", test))]
fn secret_store_error_indicates_access_denied(err: &(dyn std::error::Error + Send + Sync)) -> bool {
    if macos_keychain_error_indicates_access_denied(err) {
        return true;
    }

    #[cfg(test)]
    {
        let message = err.to_string();
        if message.starts_with("test secret store error denied") {
            return true;
        }
    }

    let message = err.to_string().to_lowercase();
    message.contains("access denied")
        || message.contains("permission denied")
        || message.contains("not authorized")
        || message.contains("auth denied")
}

#[cfg(not(any(target_os = "macos", test)))]
fn secret_store_error_indicates_access_denied(err: &(dyn std::error::Error + Send + Sync)) -> bool {
    let message = err.to_string().to_lowercase();
    message.contains("access denied")
        || message.contains("permission denied")
        || message.contains("not authorized")
        || message.contains("auth denied")
}

fn secret_store_operation_error_code(error: &KeyringError) -> &'static str {
    match error {
        KeyringError::NoStorageAccess(err) => {
            if secret_store_error_indicates_access_denied(err.as_ref()) {
                DESKTOP_SECRET_STORE_OPERATION_DENIED
            } else {
                DESKTOP_SECRET_STORE_OPERATION_LOCKED
            }
        }
        KeyringError::PlatformFailure(err) => {
            if secret_store_error_indicates_access_denied(err.as_ref()) {
                DESKTOP_SECRET_STORE_OPERATION_DENIED
            } else {
                DESKTOP_SECRET_STORE_OPERATION_UNAVAILABLE
            }
        }
        _ => DESKTOP_SECRET_STORE_OPERATION_UNAVAILABLE,
    }
}

fn map_keyring_error(operation: &'static str, error: KeyringError) -> String {
    let code = secret_store_operation_error_code(&error);
    eprintln!("desktop secret store {operation} failed: code={code} detail={error}");
    code.to_owned()
}

#[allow(dead_code)]
fn unavailable_secret_store_status(reason: &'static str) -> DesktopSecretStoreStatus {
    DesktopSecretStoreStatus {
        available: false,
        backend: DESKTOP_SECRET_STORE_BACKEND_NONE,
        can_persist_session: false,
        reason: Some(reason),
    }
}

#[cfg(target_os = "macos")]
static MACOS_SECRET_STORE_STATUS_CACHE: OnceLock<Mutex<Option<DesktopSecretStoreStatus>>> =
    OnceLock::new();

#[cfg(target_os = "macos")]
fn macos_secret_store_status_cached() -> DesktopSecretStoreStatus {
    let cache = MACOS_SECRET_STORE_STATUS_CACHE.get_or_init(|| Mutex::new(None));
    let mut guard = cache
        .lock()
        .expect("macos secret store status cache should not be poisoned");
    if let Some(status) = *guard {
        return status;
    }

    let status = macos_secret_store_status_from_probe(macos_keychain_probe());
    *guard = Some(status);
    status
}

#[cfg(target_os = "macos")]
fn macos_keychain_probe_entry() -> Result<Entry, KeyringError> {
    Entry::new(
        DESKTOP_SESSION_CREDENTIAL_SERVICE,
        DESKTOP_SESSION_KEYCHAIN_PROBE_ACCOUNT,
    )
}

#[cfg(all(test, target_os = "macos"))]
static MACOS_KEYCHAIN_PROBE_TEST_OVERRIDE: Mutex<Option<Result<(), KeyringError>>> =
    Mutex::new(None);

#[cfg(all(test, target_os = "macos"))]
fn set_macos_keychain_probe_test_override(result: Option<Result<(), KeyringError>>) {
    if let Ok(mut guard) = MACOS_KEYCHAIN_PROBE_TEST_OVERRIDE.lock() {
        *guard = result;
    }
}

#[cfg(all(test, target_os = "macos"))]
fn take_macos_keychain_probe_test_override() -> Option<Result<(), KeyringError>> {
    MACOS_KEYCHAIN_PROBE_TEST_OVERRIDE
        .lock()
        .ok()
        .and_then(|mut guard| guard.take())
}

#[cfg(all(test, target_os = "macos"))]
fn reset_macos_secret_store_status_cache_for_tests() {
    if let Some(cache) = MACOS_SECRET_STORE_STATUS_CACHE.get() {
        *cache
            .lock()
            .expect("macos secret store status cache should not be poisoned") = None;
    }
}

#[cfg(target_os = "macos")]
fn macos_keychain_probe() -> Result<(), KeyringError> {
    #[cfg(all(test, target_os = "macos"))]
    if let Some(override_result) = take_macos_keychain_probe_test_override() {
        return override_result;
    }

    let entry = macos_keychain_probe_entry()?;
    match entry.get_password() {
        Ok(_) | Err(KeyringError::NoEntry) => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(any(target_os = "macos", test))]
fn macos_unavailable_secret_store_status(reason: &'static str) -> DesktopSecretStoreStatus {
    DesktopSecretStoreStatus {
        available: false,
        backend: DESKTOP_SECRET_STORE_BACKEND_MACOS_KEYCHAIN,
        can_persist_session: false,
        reason: Some(reason),
    }
}

#[cfg(any(target_os = "macos", test))]
fn macos_keychain_error_indicates_access_denied(
    err: &(dyn std::error::Error + Send + Sync),
) -> bool {
    #[cfg(test)]
    {
        let message = err.to_string();
        if let Some(code) = message
            .strip_prefix("test keychain error ")
            .and_then(|value| value.parse::<i32>().ok())
        {
            return matches!(code, -25293 | -25308);
        }
    }

    #[cfg(target_os = "macos")]
    if let Some(code) = err
        .downcast_ref::<security_framework::base::Error>()
        .map(|error| error.code())
    {
        return matches!(code, -25293 | -25308);
    }

    false
}

#[cfg(any(target_os = "macos", test))]
fn macos_keychain_unavailable_reason(error: &KeyringError) -> &'static str {
    match error {
        KeyringError::NoStorageAccess(err) => {
            if macos_keychain_error_indicates_access_denied(err.as_ref()) {
                DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_ACCESS_DENIED
            } else {
                DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_LOCKED
            }
        }
        KeyringError::PlatformFailure(err) => {
            if macos_keychain_error_indicates_access_denied(err.as_ref()) {
                DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_ACCESS_DENIED
            } else {
                DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_UNAVAILABLE
            }
        }
        _ => DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_UNAVAILABLE,
    }
}

#[cfg(any(target_os = "macos", test))]
fn macos_secret_store_status_from_probe(probe: Result<(), KeyringError>) -> DesktopSecretStoreStatus {
    match probe {
        Ok(()) => DesktopSecretStoreStatus {
            available: true,
            backend: DESKTOP_SECRET_STORE_BACKEND_MACOS_KEYCHAIN,
            can_persist_session: true,
            reason: None,
        },
        Err(error) => {
            macos_unavailable_secret_store_status(macos_keychain_unavailable_reason(&error))
        }
    }
}

fn platform_secret_store_status() -> DesktopSecretStoreStatus {
    #[cfg(target_os = "macos")]
    {
        macos_secret_store_status_cached()
    }

    #[cfg(target_os = "linux")]
    {
        linux_secret_store_status_cached()
    }

    #[cfg(target_os = "windows")]
    {
        unavailable_secret_store_status(DESKTOP_SECRET_STORE_WINDOWS_UNSUPPORTED)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        unavailable_secret_store_status(DESKTOP_SECRET_STORE_UNSUPPORTED_PLATFORM)
    }
}

#[allow(dead_code)]
fn linux_secret_store_status_from_signals(
    has_secret_service: bool,
    has_keyutils: bool,
) -> DesktopSecretStoreStatus {
    linux_secret_store_status_from_signals_with_reason(has_secret_service, has_keyutils, None)
}

#[allow(dead_code)]
fn linux_secret_store_status_from_signals_with_reason(
    has_secret_service: bool,
    has_keyutils: bool,
    secret_service_unavailable_reason: Option<&'static str>,
) -> DesktopSecretStoreStatus {
    if has_secret_service {
        return DesktopSecretStoreStatus {
            available: true,
            backend: DESKTOP_SECRET_STORE_BACKEND_LINUX_SECRET_SERVICE,
            can_persist_session: true,
            reason: None,
        };
    }

    if has_keyutils {
        return DesktopSecretStoreStatus {
            available: true,
            backend: DESKTOP_SECRET_STORE_BACKEND_LINUX_KEYUTILS,
            can_persist_session: false,
            reason: Some(DESKTOP_SECRET_STORE_SESSION_SCOPED),
        };
    }

    unavailable_secret_store_status(
        secret_service_unavailable_reason.unwrap_or(DESKTOP_SECRET_STORE_LINUX_UNAVAILABLE),
    )
}

#[cfg(target_os = "linux")]
trait LinuxSecretServiceProbe: Send {
    fn round_trip(&self) -> Result<(), KeyringError>;
}

#[cfg(target_os = "linux")]
struct LiveLinuxSecretServiceProbe;

#[cfg(target_os = "linux")]
impl LinuxSecretServiceProbe for LiveLinuxSecretServiceProbe {
    fn round_trip(&self) -> Result<(), KeyringError> {
        #[cfg(test)]
        if let Some(override_result) = take_linux_secret_service_probe_test_override() {
            return override_result;
        }

        linux_secret_service_probe_round_trip()
    }
}

#[cfg(all(test, target_os = "linux"))]
static LINUX_SECRET_SERVICE_PROBE_TEST_OVERRIDE: Mutex<Option<Result<(), KeyringError>>> =
    Mutex::new(None);

#[cfg(all(test, target_os = "linux"))]
fn set_linux_secret_service_probe_test_override(result: Option<Result<(), KeyringError>>) {
    if let Ok(mut guard) = LINUX_SECRET_SERVICE_PROBE_TEST_OVERRIDE.lock() {
        *guard = result;
    }
}

#[cfg(all(test, target_os = "linux"))]
fn take_linux_secret_service_probe_test_override() -> Option<Result<(), KeyringError>> {
    LINUX_SECRET_SERVICE_PROBE_TEST_OVERRIDE
        .lock()
        .ok()
        .and_then(|mut guard| guard.take())
}

#[cfg(all(test, target_os = "linux"))]
fn reset_linux_secret_store_status_cache_for_tests() {
    if let Some(cache) = LINUX_SECRET_STORE_STATUS_CACHE.get() {
        *cache
            .lock()
            .expect("linux secret store status cache should not be poisoned") = None;
    }
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
enum LinuxSecretServiceProbeError {
    Timeout,
    #[allow(dead_code)]
    Keyring(KeyringError),
}

#[cfg(target_os = "linux")]
fn linux_secret_service_probe_with_timeout(
    probe: impl LinuxSecretServiceProbe + Send + 'static,
) -> Result<(), LinuxSecretServiceProbeError> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(probe.round_trip());
    });

    match rx.recv_timeout(DESKTOP_SECRET_STORE_PROBE_TIMEOUT) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(LinuxSecretServiceProbeError::Keyring(error)),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            Err(LinuxSecretServiceProbeError::Timeout)
        }
    }
}

#[cfg(target_os = "linux")]
fn linux_secret_service_unavailable_reason_from_probe_error(
    error: &LinuxSecretServiceProbeError,
) -> &'static str {
    match error {
        LinuxSecretServiceProbeError::Timeout | LinuxSecretServiceProbeError::Keyring(_) => {
            DESKTOP_SECRET_STORE_LINUX_UNAVAILABLE
        }
    }
}

#[cfg(target_os = "linux")]
fn linux_secret_service_probe_round_trip() -> Result<(), KeyringError> {
    let entry = Entry::new(
        DESKTOP_SECRET_STORE_SECRET_SERVICE_PROBE_SERVICE,
        DESKTOP_SECRET_STORE_SECRET_SERVICE_PROBE_ACCOUNT,
    )?;

    entry.set_password(DESKTOP_SECRET_STORE_SECRET_SERVICE_PROBE_SECRET)?;
    let stored = entry.get_password()?;
    if stored != DESKTOP_SECRET_STORE_SECRET_SERVICE_PROBE_SECRET {
        return Err(KeyringError::PlatformFailure(
            std::io::Error::other("linux secret service probe round-trip mismatch").into(),
        ));
    }

    let _ = entry.delete_credential();
    Ok(())
}

#[cfg(target_os = "linux")]
fn has_linux_dbus_session_bus() -> bool {
    env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some()
        || env::var_os("XDG_RUNTIME_DIR")
            .map(|runtime_dir| Path::new(&runtime_dir).join("bus").exists())
            .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn linux_secret_store_status_from_live_probes(
    secret_service_probe: impl LinuxSecretServiceProbe + Send + 'static,
) -> DesktopSecretStoreStatus {
    let mut unavailable_reason = None;
    let has_secret_service = if !has_linux_dbus_session_bus() {
        false
    } else {
        match linux_secret_service_probe_with_timeout(secret_service_probe) {
            Ok(()) => true,
            Err(error) => {
                unavailable_reason =
                    Some(linux_secret_service_unavailable_reason_from_probe_error(&error));
                false
            }
        }
    };

    linux_secret_store_status_from_signals_with_reason(
        has_secret_service,
        has_linux_keyutils_backend(),
        unavailable_reason,
    )
}

#[cfg(target_os = "linux")]
static LINUX_SECRET_STORE_STATUS_CACHE: OnceLock<Mutex<Option<DesktopSecretStoreStatus>>> =
    OnceLock::new();

#[cfg(target_os = "linux")]
fn linux_secret_store_status_cached() -> DesktopSecretStoreStatus {
    let cache = LINUX_SECRET_STORE_STATUS_CACHE.get_or_init(|| Mutex::new(None));
    let mut guard = cache
        .lock()
        .expect("linux secret store status cache should not be poisoned");
    if let Some(status) = *guard {
        return status;
    }

    let status = linux_secret_store_status_from_live_probes(LiveLinuxSecretServiceProbe);
    *guard = Some(status);
    status
}

#[cfg(all(target_os = "linux", test))]
fn has_secret_service_backend_service_files() -> bool {
    dir_has_fragment("/usr/share/dbus-1/services", "org.freedesktop.secrets")
        || dir_has_fragment("/usr/share/dbus-1/services", "gnome-keyring")
        || dir_has_fragment("/usr/share/dbus-1/services", "kwallet")
        || dir_has_fragment(
            "/usr/local/share/dbus-1/services",
            "org.freedesktop.secrets",
        )
        || dir_has_fragment("/usr/local/share/dbus-1/services", "gnome-keyring")
        || dir_has_fragment("/usr/local/share/dbus-1/services", "kwallet")
}

#[cfg(target_os = "linux")]
fn has_linux_keyutils_backend() -> bool {
    linux_keyutils_probe_passes()
}

#[cfg(target_os = "linux")]
fn linux_keyutils_probe_passes() -> bool {
    linux_keyutils_probe_round_trip().is_ok()
}

#[cfg(target_os = "linux")]
fn linux_keyutils_probe_round_trip() -> Result<(), KeyringError> {
    use keyring::credential::CredentialApi;
    use keyring::keyutils::KeyutilsCredential;

    let credential = KeyutilsCredential::new_with_target(
        None,
        DESKTOP_SECRET_STORE_KEYUTILS_PROBE_SERVICE,
        DESKTOP_SECRET_STORE_KEYUTILS_PROBE_ACCOUNT,
    )?;

    credential.set_password(DESKTOP_SECRET_STORE_KEYUTILS_PROBE_SECRET)?;
    let stored = credential.get_password()?;
    if stored != DESKTOP_SECRET_STORE_KEYUTILS_PROBE_SECRET {
        return Err(KeyringError::PlatformFailure(
            std::io::Error::other("linux keyutils probe round-trip mismatch").into(),
        ));
    }

    let _ = credential.delete_credential();
    Ok(())
}

fn desktop_get_session_from_store(
    store: &impl DesktopSessionSecretStore,
) -> Result<Option<DesktopSessionEnvelope>, String> {
    desktop_get_session_from_store_at(store, current_timestamp_ms())
}

fn desktop_get_session_from_store_at(
    store: &impl DesktopSessionSecretStore,
    now_ms: u64,
) -> Result<Option<DesktopSessionEnvelope>, String> {
    if !store.status().can_persist_session {
        return Ok(None);
    }

    let Some(secret) = store.get_secret()? else {
        return Ok(None);
    };
    let session = serde_json::from_str::<DesktopSessionEnvelope>(&secret)
        .map_err(|_| DESKTOP_STORED_SESSION_INVALID.to_owned())?;
    let session = sanitize_session_envelope(session)
        .map_err(|_| DESKTOP_STORED_SESSION_INVALID.to_owned())?;
    if session_envelope_is_expired(&session, now_ms) {
        let _ = desktop_remove_session_from_store(store);
        return Ok(None);
    }

    Ok(Some(session))
}

fn desktop_set_session_in_store(
    store: &impl DesktopSessionSecretStore,
    session: DesktopSessionEnvelope,
) -> Result<bool, String> {
    let mut session = sanitize_session_envelope(session)?;
    session.stored_at_ms = Some(current_timestamp_ms());
    if !store.status().can_persist_session {
        return Ok(false);
    }

    let secret =
        serde_json::to_string(&session).map_err(|_| DESKTOP_STORED_SESSION_INVALID.to_owned())?;
    store.set_secret(&secret)
}

fn desktop_remove_session_from_store(
    store: &impl DesktopSessionSecretStore,
) -> Result<bool, String> {
    if !store.status().can_persist_session {
        return Ok(false);
    }

    store.remove_secret()
}

/// External URLs must use HTTPS unless they target a loopback host (development).
pub fn is_safe_external_url(value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };

    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }

    match url.scheme() {
        "https" => url.host_str().is_some(),
        "http" => url
            .host_str()
            .map(is_loopback_session_host)
            .unwrap_or(false),
        "mailto" | "matrix" => true,
        _ => false,
    }
}

#[tauri::command]
pub fn desktop_open_external_url<R: Runtime>(app: AppHandle<R>, url: String) -> bool {
    if !is_safe_external_url(&url) {
        return false;
    }

    app.opener().open_url(url, None::<&str>).is_ok()
}

fn sanitize_download_filename(filename: &str) -> String {
    let safe_name = Path::new(filename)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("download")
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            {
                '_'
            } else {
                ch
            }
        })
        .collect::<String>();

    let trimmed = safe_name.trim().trim_matches('.').trim();
    if trimmed.is_empty() {
        "download".to_owned()
    } else {
        trimmed.chars().take(180).collect()
    }
}

fn downloads_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    let home = env::var_os("USERPROFILE");

    #[cfg(not(target_os = "windows"))]
    let home = env::var_os("HOME");

    let Some(home_dir) = home else {
        return Err("Unable to resolve home directory".to_owned());
    };

    Ok(PathBuf::from(home_dir).join("Downloads"))
}

fn unique_download_path(downloads: &Path, filename: &str) -> PathBuf {
    let initial = downloads.join(filename);
    if !initial.exists() {
        return initial;
    }

    let path = Path::new(filename);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("download");
    let extension = path.extension().and_then(|value| value.to_str());

    for index in 1..1000 {
        let candidate_name = match extension {
            Some(ext) if !ext.is_empty() => format!("{stem} ({index}).{ext}"),
            _ => format!("{stem} ({index})"),
        };
        let candidate = downloads.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
    }

    downloads.join(format!("{stem} ({})", chrono_like_timestamp()))
}

fn chrono_like_timestamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

pub fn should_stream_file_ipc(byte_count: u64) -> bool {
    byte_count > DESKTOP_FILE_IPC_INLINE_THRESHOLD as u64
}

pub fn dropped_file_read_mode(byte_count: u64) -> DroppedFileReadMode {
    if should_stream_file_ipc(byte_count) {
        DroppedFileReadMode::Streamed
    } else {
        DroppedFileReadMode::Inline
    }
}

fn new_file_transfer_id(prefix: &str) -> String {
    static TRANSFER_COUNTER: AtomicU64 = AtomicU64::new(0);
    let counter = TRANSFER_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{counter}-{}", chrono_like_timestamp())
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

fn take_save_session(session_id: &str) -> Result<SaveFileSession, String> {
    save_file_sessions()
        .lock()
        .map_err(|_| "Unable to access save file sessions".to_owned())?
        .remove(session_id)
        .ok_or_else(|| "Save file session is not available".to_owned())
}

fn register_dropped_read_session(session: DroppedReadSession) -> Result<String, String> {
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
        return Err(
            "File is too large for inline save; use streaming save commands".to_owned(),
        );
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

    let session_id = register_save_session(SaveFileSession {
        temp_path,
        filename: safe_filename,
        expected_size: total_size,
        bytes_received: 0,
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

    let mut sessions = save_file_sessions()
        .lock()
        .map_err(|_| "Unable to access save file sessions".to_owned())?;
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

    let sessions = dropped_read_sessions()
        .lock()
        .map_err(|_| "Unable to access dropped file read sessions".to_owned())?;
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
    remove_dropped_read_session(&transfer_id);
    Ok(true)
}

fn is_safe_agent_url(value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };

    url.scheme() == "https"
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
}

fn sanitize_route(route: String) -> Result<String, String> {
    let route = sanitize_action_text(route, MAX_DESKTOP_ROUTE_CHARS);
    if route.is_empty() {
        return Err("Route cannot be empty".to_owned());
    }
    if route.contains("://") {
        return Err("Route must be an internal app route".to_owned());
    }
    if !route.starts_with('/') && !route.starts_with('#') {
        return Err("Route must start with / or #".to_owned());
    }
    Ok(route)
}

fn sanitize_notification_route(route: String) -> Result<String, String> {
    sanitize_route(route)
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

fn last_shortcut_state() -> &'static Mutex<Option<DesktopShortcutApplyState>> {
    LAST_SHORTCUT_APPLY_STATE.get_or_init(|| Mutex::new(None))
}

fn set_last_shortcut_apply_state(state: DesktopShortcutApplyState) {
    if let Ok(mut guard) = last_shortcut_state().lock() {
        *guard = Some(state);
    }
}

fn read_last_shortcut_apply_state() -> Option<DesktopShortcutApplyState> {
    last_shortcut_state()
        .lock()
        .ok()
        .and_then(|state| state.clone())
}

fn last_active_shortcut_config() -> &'static Mutex<Option<DesktopShortcutConfig>> {
    LAST_ACTIVE_SHORTCUT_CONFIG.get_or_init(|| Mutex::new(None))
}

fn read_last_active_shortcut_config() -> Option<DesktopShortcutConfig> {
    last_active_shortcut_config()
        .lock()
        .ok()
        .and_then(|config| config.clone())
}

fn set_last_active_shortcut_config(config: DesktopShortcutConfig) {
    if let Ok(mut guard) = last_active_shortcut_config().lock() {
        *guard = Some(config);
    }
}

fn shortcut_route_for_slot(config: &DesktopShortcutConfig, shortcut: &str) -> Option<&'static str> {
    if config.show == shortcut {
        return Some(ROUTE_HOME);
    }
    if config.later == shortcut {
        return Some(ROUTE_LATER);
    }
    if config.notifications == shortcut {
        return Some(ROUTE_NOTIFICATIONS);
    }
    None
}

fn shortcut_strings_for_config(config: &DesktopShortcutConfig) -> [&str; 3] {
    [
        config.show.as_str(),
        config.later.as_str(),
        config.notifications.as_str(),
    ]
}

fn shortcuts_needing_registration(
    previous: Option<&DesktopShortcutConfig>,
    normalized: &DesktopShortcutConfig,
) -> Vec<String> {
    let Some(previous) = previous else {
        return vec![
            normalized.show.clone(),
            normalized.later.clone(),
            normalized.notifications.clone(),
        ];
    };

    let mut shortcuts = Vec::new();
    if previous.show != normalized.show {
        shortcuts.push(normalized.show.clone());
    }
    if previous.later != normalized.later {
        shortcuts.push(normalized.later.clone());
    }
    if previous.notifications != normalized.notifications {
        shortcuts.push(normalized.notifications.clone());
    }
    shortcuts
}

fn shortcuts_needing_handler_rebind(
    previous: &DesktopShortcutConfig,
    normalized: &DesktopShortcutConfig,
) -> Vec<String> {
    let mut shortcuts = Vec::new();
    for shortcut in shortcut_strings_for_config(normalized) {
        let Some(new_route) = shortcut_route_for_slot(normalized, shortcut) else {
            continue;
        };
        let Some(old_route) = shortcut_route_for_slot(previous, shortcut) else {
            continue;
        };
        if old_route != new_route {
            shortcuts.push(shortcut.to_owned());
        }
    }
    shortcuts
}

fn retired_shortcut_strings(
    previous: &DesktopShortcutConfig,
    normalized: &DesktopShortcutConfig,
) -> Vec<String> {
    let new_strings: HashSet<&str> = shortcut_strings_for_config(normalized).into_iter().collect();
    shortcut_strings_for_config(previous)
        .into_iter()
        .filter(|shortcut| !new_strings.contains(shortcut))
        .map(str::to_owned)
        .collect()
}

fn build_shortcut_route_map(
    normalized: &DesktopShortcutConfig,
    parsed_show: &Shortcut,
    parsed_later: &Shortcut,
    parsed_notifications: &Shortcut,
) -> HashMap<u32, &'static str> {
    let mut route_by_id = HashMap::new();
    route_by_id.insert(parsed_show.id(), ROUTE_HOME);
    route_by_id.insert(parsed_later.id(), ROUTE_LATER);
    route_by_id.insert(parsed_notifications.id(), ROUTE_NOTIFICATIONS);
    debug_assert_eq!(normalized.show, parsed_show.to_string());
    debug_assert_eq!(normalized.later, parsed_later.to_string());
    debug_assert_eq!(normalized.notifications, parsed_notifications.to_string());
    route_by_id
}

fn register_desktop_shortcut_batch(
    global_shortcut: &tauri_plugin_global_shortcut::GlobalShortcut<tauri::Wry>,
    shortcuts: &[String],
    route_by_id: HashMap<u32, &'static str>,
) -> Result<(), String> {
    if shortcuts.is_empty() {
        return Ok(());
    }

    let shortcut_refs = shortcuts.iter().map(String::as_str).collect::<Vec<_>>();
    global_shortcut
        .on_shortcuts(shortcut_refs, move |app, shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }

            let Some(route) = route_by_id.get(&shortcut.id()) else {
                return;
            };
            if let Err(error) = navigate_main_window(app, route) {
                eprintln!("failed to handle desktop shortcut: {error}");
            }
        })
        .map_err(|error| error.to_string())
}

fn unregister_desktop_shortcut_batch(
    global_shortcut: &tauri_plugin_global_shortcut::GlobalShortcut<tauri::Wry>,
    shortcuts: &[String],
) {
    if shortcuts.is_empty() {
        return;
    }

    let shortcut_refs = shortcuts.iter().map(String::as_str).collect::<Vec<_>>();
    let _ = global_shortcut.unregister_multiple(shortcut_refs);
}

fn rebind_desktop_shortcut_handlers(
    global_shortcut: &tauri_plugin_global_shortcut::GlobalShortcut<tauri::Wry>,
    shortcuts: &[String],
    route_by_id: HashMap<u32, &'static str>,
) -> Result<(), String> {
    for shortcut in shortcuts {
        let _ = global_shortcut.unregister(shortcut.as_str());
        register_desktop_shortcut_batch(
            global_shortcut,
            std::slice::from_ref(shortcut),
            route_by_id.clone(),
        )?;
    }
    Ok(())
}

fn apply_desktop_shortcuts(
    app: &AppHandle,
    normalized: DesktopShortcutConfig,
    parsed_show: Shortcut,
    parsed_later: Shortcut,
    parsed_notifications: Shortcut,
) -> DesktopShortcutApplyResult {
    let previous_config = read_last_active_shortcut_config();
    let global_shortcut = app.global_shortcut();
    let route_by_id = build_shortcut_route_map(
        &normalized,
        &parsed_show,
        &parsed_later,
        &parsed_notifications,
    );

    let brand_new_shortcuts = shortcuts_needing_registration(previous_config.as_ref(), &normalized);
    if let Err(error) =
        register_desktop_shortcut_batch(&global_shortcut, &brand_new_shortcuts, route_by_id.clone())
    {
        unregister_desktop_shortcut_batch(&global_shortcut, &brand_new_shortcuts);
        let state = shortcut_state_from_error(&error);
        let preserved_state = read_last_shortcut_apply_state().unwrap_or(state.clone());
        set_last_shortcut_apply_state(preserved_state);
        return shortcut_result(
            state,
            Some(format!("Failed to register desktop shortcuts: {error}")),
            desktop_shortcut_fallback_command(),
        );
    }

    if let Some(previous) = previous_config.as_ref() {
        let rebind_shortcuts = shortcuts_needing_handler_rebind(previous, &normalized);
        if let Err(error) = rebind_desktop_shortcut_handlers(
            &global_shortcut,
            &rebind_shortcuts,
            route_by_id.clone(),
        ) {
            unregister_desktop_shortcut_batch(&global_shortcut, &brand_new_shortcuts);
            let state = shortcut_state_from_error(&error);
            let preserved_state = read_last_shortcut_apply_state().unwrap_or(state.clone());
            set_last_shortcut_apply_state(preserved_state);
            return shortcut_result(
                state,
                Some(format!("Failed to update desktop shortcuts: {error}")),
                desktop_shortcut_fallback_command(),
            );
        }

        let retired_shortcuts = retired_shortcut_strings(previous, &normalized);
        unregister_desktop_shortcut_batch(&global_shortcut, &retired_shortcuts);
    }

    set_last_active_shortcut_config(normalized);
    set_last_shortcut_apply_state(DesktopShortcutApplyState::Active);
    shortcut_result(DesktopShortcutApplyState::Active, None, None)
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

fn shortcut_apply_state_message(state: DesktopShortcutApplyState) -> &'static str {
    match state {
        DesktopShortcutApplyState::Active => "Desktop shortcuts are active.",
        DesktopShortcutApplyState::PermissionNeeded => {
            "Shortcut registration needs permission on this desktop session."
        }
        DesktopShortcutApplyState::Unsupported => {
            "Desktop shortcuts are unsupported in this environment."
        }
        DesktopShortcutApplyState::Unknown => {
            "Desktop shortcut registration has not been attempted yet."
        }
        DesktopShortcutApplyState::Failed => "Desktop shortcut registration failed.",
    }
}

fn is_gnome_session() -> bool {
    env::var("XDG_CURRENT_DESKTOP")
        .map(|value| value.to_ascii_lowercase().contains("gnome"))
        .unwrap_or(false)
}

fn shortcut_permission_help_hint() -> Option<&'static str> {
    #[cfg(target_os = "macos")]
    {
        return Some(
            "On macOS, grant Synara Input Monitoring permission in System Settings > Privacy & Security.",
        );
    }

    #[cfg(target_os = "linux")]
    {
        if is_kde_wayland_session() {
            return Some(
                "On KDE Plasma Wayland, global shortcut capture can require manual registration in System Settings > Shortcuts.",
            );
        }
        if is_wayland() {
            if is_gnome_session() {
                return Some(
                    "On GNOME Wayland, global shortcuts may require portal or compositor permission. Check Settings > Keyboard > Keyboard Shortcuts.",
                );
            }
            return Some(
                "On Wayland sessions, global shortcuts may require portal or compositor permission. Check your desktop environment shortcut settings.",
            );
        }
        if is_kde() {
            return Some(
                "On KDE X11, verify shortcut bindings in System Settings > Shortcuts and ensure no other app has claimed the keys.",
            );
        }
        return Some(
            "On Linux X11, verify no other application has claimed the shortcut and check your desktop environment shortcut settings.",
        );
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Some("Check the session permissions for global shortcuts and try again.")
    }
}

fn desktop_shortcut_fallback_command() -> Option<String> {
    if is_kde_wayland_session() {
        return Some(
            "Open System Settings > Shortcuts and create a custom shortcut for Synara.".to_string(),
        );
    }
    None
}

fn unresolved_shortcut_apply_state() -> DesktopShortcutApplyState {
    if is_kde_wayland_session() {
        DesktopShortcutApplyState::Unknown
    } else {
        DesktopShortcutApplyState::Failed
    }
}

fn shortcut_result(
    state: DesktopShortcutApplyState,
    message: Option<String>,
    fallback_command: Option<String>,
) -> DesktopShortcutApplyResult {
    let fallback_command = if matches!(state, DesktopShortcutApplyState::PermissionNeeded) {
        desktop_shortcut_fallback_command()
    } else {
        fallback_command
    };
    let message = message.unwrap_or_else(|| {
        if matches!(state, DesktopShortcutApplyState::PermissionNeeded) {
            let mut parts = vec![shortcut_apply_state_message(state).to_owned()];
            if let Some(hint) = shortcut_permission_help_hint() {
                parts.push(hint.to_owned());
            }
            return parts.join(" ");
        }
        shortcut_apply_state_message(state).to_owned()
    });

    DesktopShortcutApplyResult {
        success: matches!(state, DesktopShortcutApplyState::Active),
        state,
        message,
        fallback_command,
    }
}

fn shortcut_state_from_error(error: &str) -> DesktopShortcutApplyState {
    let normalized = error.to_ascii_lowercase();
    if normalized.contains("permission") || normalized.contains("denied") {
        DesktopShortcutApplyState::PermissionNeeded
    } else if normalized.contains("not supported") || normalized.contains("unsupported") {
        DesktopShortcutApplyState::Unsupported
    } else {
        DesktopShortcutApplyState::Failed
    }
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
        let mut pending = coalescer.pending.lock().map_err(|error| error.to_string())?;
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
    if coalescer
        .flush_scheduled
        .swap(true, Ordering::AcqRel)
    {
        return;
    }

    let delay = {
        let last_applied_at = coalescer.last_applied_at.lock().ok().and_then(|guard| *guard);
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
        let mut pending = coalescer.pending.lock().map_err(|error| error.to_string())?;
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
        if !is_safe_agent_url(&url) {
            return Err("Agent action URL must use https".to_owned());
        }
        action.url = Some(sanitize_action_text(
            url,
            DESKTOP_AGENT_ACTION_MAX_TEXT_CHARS,
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
    match (&action.kind, &action.url) {
        (Some(kind), _) => ALLOWED_AGENT_ACTION_KIND.contains(&kind.as_str()),
        (None, Some(_)) => true,
        _ => false,
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
    format!(
        "window.dispatchEvent(new CustomEvent('{DESKTOP_TRAY_DND_TOGGLE_EVENT}'));"
    )
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
    let supported = cfg!(not(any(target_os = "android", target_os = "ios")));
    if !supported {
        return shortcut_result(
            DesktopShortcutApplyState::Unsupported,
            Some("Global shortcuts are not supported on this platform.".to_string()),
            None,
        );
    }

    let normalized = match validate_shortcuts(&shortcuts) {
        Ok(normalized) => normalized,
        Err(message) => {
            return shortcut_result(DesktopShortcutApplyState::Failed, Some(message), None);
        }
    };

    let parsed_show = match parse_shortcut(&normalized.show) {
        Ok(value) => value,
        Err(message) => {
            return shortcut_result(DesktopShortcutApplyState::Failed, Some(message), None);
        }
    };
    let parsed_later = match parse_shortcut(&normalized.later) {
        Ok(value) => value,
        Err(message) => {
            return shortcut_result(DesktopShortcutApplyState::Failed, Some(message), None);
        }
    };
    let parsed_notifications = match parse_shortcut(&normalized.notifications) {
        Ok(value) => value,
        Err(message) => {
            return shortcut_result(DesktopShortcutApplyState::Failed, Some(message), None);
        }
    };

    apply_desktop_shortcuts(
        &app,
        normalized,
        parsed_show,
        parsed_later,
        parsed_notifications,
    )
}

pub fn bridge_supports_secure_secret_store(status: &DesktopSecretStoreStatus) -> bool {
    status.available && status.can_persist_session
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

    let shortcut_state =
        read_last_shortcut_apply_state().unwrap_or_else(unresolved_shortcut_apply_state);
    let global_shortcuts = DesktopIntegrationCheck {
        name: "Global Shortcuts".to_string(),
        supported: cfg!(not(any(target_os = "android", target_os = "ios"))),
        ready: matches!(shortcut_state, DesktopShortcutApplyState::Active),
        message: match shortcut_state {
            DesktopShortcutApplyState::Active => "Global shortcuts are active.".to_string(),
            DesktopShortcutApplyState::PermissionNeeded => {
                let mut parts = vec![
                    "Global shortcuts require permission in this desktop session.".to_string(),
                ];
                if let Some(hint) = shortcut_permission_help_hint() {
                    parts.push(hint.to_owned());
                }
                parts.join(" ")
            }
            DesktopShortcutApplyState::Unsupported => {
                "Global shortcuts are unsupported in this build.".to_string()
            }
            DesktopShortcutApplyState::Unknown => {
                if read_last_active_shortcut_config().is_none() {
                    "Global shortcuts are configured after the client loads.".to_string()
                } else {
                    "Global shortcut registration has not been attempted yet.".to_string()
                }
            }
            DesktopShortcutApplyState::Failed => {
                if read_last_active_shortcut_config().is_none() {
                    "Global shortcuts are configured after the client loads.".to_string()
                } else {
                    "Global shortcuts not currently active.".to_string()
                }
            }
        },
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
#[derive(Debug)]
struct MacosKeychainTestError {
    code: i32,
}

#[cfg(test)]
impl std::fmt::Display for MacosKeychainTestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "test keychain error {}", self.code)
    }
}

#[cfg(test)]
impl std::error::Error for MacosKeychainTestError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());
    static DROPPED_FILE_ALLOWLIST_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn valid_session_envelope() -> DesktopSessionEnvelope {
        DesktopSessionEnvelope {
            base_url: "https://matrix.example.org".to_owned(),
            user_id: "@alice:example.org".to_owned(),
            device_id: "DEVICEID".to_owned(),
            access_token: "access-token".to_owned(),
            refresh_token: None,
            expires_in_ms: Some(3_600_000),
            stored_at_ms: None,
        }
    }

    fn available_test_secret_store_status() -> DesktopSecretStoreStatus {
        DesktopSecretStoreStatus {
            available: true,
            backend: DESKTOP_SECRET_STORE_BACKEND_MACOS_KEYCHAIN,
            can_persist_session: true,
            reason: None,
        }
    }

    struct TestSessionSecretStore {
        status: DesktopSecretStoreStatus,
        secret: Mutex<Option<String>>,
    }

    impl TestSessionSecretStore {
        fn available() -> Self {
            Self {
                status: available_test_secret_store_status(),
                secret: Mutex::new(None),
            }
        }

        fn unavailable() -> Self {
            Self {
                status: unavailable_secret_store_status(DESKTOP_SECRET_STORE_NOT_CONFIGURED),
                secret: Mutex::new(None),
            }
        }

        fn with_secret(secret: String) -> Self {
            Self {
                status: available_test_secret_store_status(),
                secret: Mutex::new(Some(secret)),
            }
        }

        fn stored_secret(&self) -> Option<String> {
            self.secret.lock().expect("secret lock").clone()
        }
    }

    impl DesktopSessionSecretStore for TestSessionSecretStore {
        fn status(&self) -> DesktopSecretStoreStatus {
            self.status
        }

        fn get_secret(&self) -> Result<Option<String>, String> {
            Ok(self.stored_secret())
        }

        fn set_secret(&self, secret: &str) -> Result<bool, String> {
            *self.secret.lock().expect("secret lock") = Some(secret.to_owned());
            Ok(true)
        }

        fn remove_secret(&self) -> Result<bool, String> {
            Ok(self.secret.lock().expect("secret lock").take().is_some())
        }
    }

    #[test]
    fn validate_shortcuts_accepts_valid_input() {
        let normalized = validate_shortcuts(&DesktopShortcutConfig {
            show: "cmd+shift+c".to_string(),
            later: "CmdOrCtrl+Shift+L".to_string(),
            notifications: " CmdOrCtrl+Shift+N ".to_string(),
        })
        .expect("shortcuts should validate");

        assert_eq!(normalized.show, "cmd+shift+c");
        assert_eq!(normalized.later, "CmdOrCtrl+Shift+L");
        assert_eq!(normalized.notifications, "CmdOrCtrl+Shift+N");
    }

    #[test]
    fn validate_shortcuts_rejects_duplicate_shortcuts() {
        let result = validate_shortcuts(&DesktopShortcutConfig {
            show: "CmdOrCtrl+Shift+C".to_string(),
            later: "CmdOrCtrl+Shift+C".to_string(),
            notifications: "CmdOrCtrl+Shift+N".to_string(),
        });

        assert!(result.is_err());
    }

    #[test]
    fn validate_shortcuts_rejects_invalid_shortcut() {
        let result = validate_shortcuts(&DesktopShortcutConfig {
            show: "Ctrl+".to_string(),
            later: "CmdOrCtrl+Shift+L".to_string(),
            notifications: "CmdOrCtrl+Shift+N".to_string(),
        });

        assert!(result.is_err());
    }

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
    fn external_url_filter_allows_https_and_loopback_http_only() {
        assert!(is_safe_external_url("https://example.org/path"));
        assert!(!is_safe_external_url("http://example.org/path"));
        assert!(is_safe_external_url("http://127.0.0.1:8080"));
        assert!(is_safe_external_url("http://localhost:8080"));
        assert!(is_safe_external_url("mailto:test@example.org"));
        assert!(!is_safe_external_url("javascript:alert(1)"));
        assert!(!is_safe_external_url("file:///Users/example/.ssh/id_rsa"));
        assert!(!is_safe_external_url("https://user:pass@example.org/"));
    }

    #[test]
    fn sanitize_session_envelope_accepts_https_session() {
        let session = sanitize_session_envelope(DesktopSessionEnvelope {
            base_url: " https://matrix.example.org ".to_owned(),
            user_id: " @alice:example.org ".to_owned(),
            device_id: " DEVICEID ".to_owned(),
            access_token: " access-token ".to_owned(),
            refresh_token: Some(" refresh-token ".to_owned()),
            expires_in_ms: Some(3_600_000),
            stored_at_ms: None,
        })
        .expect("session envelope should pass");

        assert_eq!(session.base_url, "https://matrix.example.org");
        assert_eq!(session.user_id, "@alice:example.org");
        assert_eq!(session.device_id, "DEVICEID");
        assert_eq!(session.access_token, "access-token");
        assert_eq!(session.refresh_token.as_deref(), Some("refresh-token"));
        assert_eq!(session.expires_in_ms, Some(3_600_000));
    }

    #[test]
    fn sanitize_session_envelope_allows_loopback_http_for_development() {
        let mut session = valid_session_envelope();
        session.base_url = "http://localhost:8008".to_owned();

        let sanitized = sanitize_session_envelope(session).expect("loopback session should pass");

        assert_eq!(sanitized.base_url, "http://localhost:8008");
    }

    #[test]
    fn sanitize_session_envelope_rejects_empty_access_token() {
        let mut session = valid_session_envelope();
        session.access_token = "   ".to_owned();

        assert!(sanitize_session_envelope(session).is_err());
    }

    #[test]
    fn sanitize_session_envelope_rejects_plain_http_remote_base_url() {
        let mut session = valid_session_envelope();
        session.base_url = "http://matrix.example.org".to_owned();

        let result = sanitize_session_envelope(session);

        assert!(result.is_err());
    }

    #[test]
    fn sanitize_session_envelope_does_not_echo_token_values_in_errors() {
        let secret_token = "super-secret-access-token";
        let mut session = valid_session_envelope();
        session.base_url = "http://matrix.example.org".to_owned();
        session.access_token = secret_token.to_owned();

        let error = sanitize_session_envelope(session)
            .err()
            .expect("session envelope should fail");

        assert!(!error.contains(secret_token));
    }

    #[test]
    fn credential_store_names_are_stable_and_scoped() {
        assert_eq!(
            DESKTOP_SESSION_CREDENTIAL_SERVICE,
            "com.whylandcreative.synara.desktop"
        );
        assert_eq!(
            DESKTOP_SESSION_LEGACY_CREDENTIAL_SERVICE,
            "app.synara.desktop"
        );
        assert_eq!(DESKTOP_SESSION_CREDENTIAL_ACCOUNT, "matrix-session");
    }

    #[test]
    fn macos_secret_store_status_from_probe_reports_available_when_keychain_accessible() {
        let status = macos_secret_store_status_from_probe(Ok(()));

        assert!(status.available);
        assert_eq!(status.backend, DESKTOP_SECRET_STORE_BACKEND_MACOS_KEYCHAIN);
        assert!(status.can_persist_session);
        assert_eq!(status.reason, None);
        assert!(bridge_supports_secure_secret_store(&status));
    }

    #[test]
    fn macos_secret_store_status_from_probe_reports_locked_when_keychain_unavailable() {
        let status = macos_secret_store_status_from_probe(Err(KeyringError::NoStorageAccess(
            Box::new(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "keychain locked",
            )),
        )));

        assert!(!status.available);
        assert_eq!(status.backend, DESKTOP_SECRET_STORE_BACKEND_MACOS_KEYCHAIN);
        assert!(!status.can_persist_session);
        assert_eq!(
            status.reason,
            Some(DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_LOCKED)
        );
        assert!(!bridge_supports_secure_secret_store(&status));
    }

    #[test]
    fn macos_secret_store_status_from_probe_reports_denied_when_acl_blocks_access() {
        let status = macos_secret_store_status_from_probe(Err(KeyringError::PlatformFailure(
            Box::new(MacosKeychainTestError { code: -25293 }),
        )));

        assert!(!status.available);
        assert_eq!(
            status.reason,
            Some(DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_ACCESS_DENIED)
        );
    }

    #[test]
    fn macos_secret_store_status_from_probe_reports_unavailable_on_other_failures() {
        let status = macos_secret_store_status_from_probe(Err(KeyringError::PlatformFailure(
            Box::new(std::io::Error::other("unexpected keychain failure")),
        )));

        assert!(!status.available);
        assert_eq!(
            status.reason,
            Some(DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_UNAVAILABLE)
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn platform_secret_store_status_probes_macos_keychain() {
        let status = platform_secret_store_status();

        assert!(status.available);
        assert_eq!(status.backend, DESKTOP_SECRET_STORE_BACKEND_MACOS_KEYCHAIN);
        assert!(status.can_persist_session);
        assert_eq!(status.reason, None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn platform_secret_store_status_honors_injected_macos_keychain_probe_failure() {
        reset_macos_secret_store_status_cache_for_tests();
        set_macos_keychain_probe_test_override(Some(Err(KeyringError::NoStorageAccess(
            Box::new(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "simulated locked keychain",
            )),
        ))));

        let status = platform_secret_store_status();

        assert!(!status.available);
        assert_eq!(
            status.reason,
            Some(DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_LOCKED)
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn platform_secret_store_status_reports_windows_unsupported() {
        let status = platform_secret_store_status();

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
    fn windows_secret_store_status_mapping_is_explicit_and_non_persistent() {
        let status =
            unavailable_secret_store_status(DESKTOP_SECRET_STORE_WINDOWS_UNSUPPORTED);

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
    fn linux_secret_store_status_prefers_secret_service_for_persistence() {
        let status = linux_secret_store_status_from_signals(true, true);

        assert_eq!(status.available, true);
        assert_eq!(
            status.backend,
            DESKTOP_SECRET_STORE_BACKEND_LINUX_SECRET_SERVICE
        );
        assert_eq!(status.can_persist_session, true);
        assert_eq!(status.reason, None);
    }

    #[test]
    fn linux_secret_store_status_reports_keyutils_when_probe_passes() {
        let status = linux_secret_store_status_from_signals(false, true);

        assert_eq!(status.available, true);
        assert_eq!(status.backend, DESKTOP_SECRET_STORE_BACKEND_LINUX_KEYUTILS);
        assert_eq!(status.can_persist_session, false);
        assert_eq!(status.reason, Some(DESKTOP_SECRET_STORE_SESSION_SCOPED));
    }

    #[test]
    fn linux_secret_store_status_reports_unavailable_when_probe_fails() {
        let status = linux_secret_store_status_from_signals(false, false);

        assert_eq!(status.available, false);
        assert_eq!(status.backend, DESKTOP_SECRET_STORE_BACKEND_NONE);
        assert_eq!(status.can_persist_session, false);
        assert_eq!(status.reason, Some(DESKTOP_SECRET_STORE_LINUX_UNAVAILABLE));
    }

    #[test]
    fn linux_secret_store_status_does_not_prefer_keyutils_over_unavailable() {
        let status = linux_secret_store_status_from_signals(false, false);

        assert_ne!(status.backend, DESKTOP_SECRET_STORE_BACKEND_LINUX_KEYUTILS);
        assert!(!status.available);
        assert!(!status.can_persist_session);
    }

    #[test]
    fn linux_secret_store_status_reports_unavailable_reason_from_probe_failure() {
        let status = linux_secret_store_status_from_signals_with_reason(
            false,
            false,
            Some(DESKTOP_SECRET_STORE_LINUX_UNAVAILABLE),
        );

        assert!(!status.available);
        assert_eq!(status.backend, DESKTOP_SECRET_STORE_BACKEND_NONE);
        assert_eq!(status.reason, Some(DESKTOP_SECRET_STORE_LINUX_UNAVAILABLE));
    }

    #[cfg(target_os = "linux")]
    mod linux_secret_service_probe_tests {
        use super::*;
        use std::time::Instant;

        struct DbusSessionEnvGuard {
            original_dbus: Option<std::ffi::OsString>,
            original_runtime: Option<std::ffi::OsString>,
        }

        impl DbusSessionEnvGuard {
            fn with_fake_session_bus() -> Self {
                let _env_guard = ENV_LOCK.lock().expect("env lock should not be poisoned");
                let original_dbus = std::env::var_os("DBUS_SESSION_BUS_ADDRESS");
                let original_runtime = std::env::var_os("XDG_RUNTIME_DIR");
                std::env::set_var(
                    "DBUS_SESSION_BUS_ADDRESS",
                    "unix:path=/tmp/synara-secret-service-probe-test-bus",
                );
                std::env::remove_var("XDG_RUNTIME_DIR");
                drop(_env_guard);

                Self {
                    original_dbus,
                    original_runtime,
                }
            }
        }

        impl Drop for DbusSessionEnvGuard {
            fn drop(&mut self) {
                let _env_guard = ENV_LOCK.lock().expect("env lock should not be poisoned");
                if let Some(value) = self.original_dbus.take() {
                    std::env::set_var("DBUS_SESSION_BUS_ADDRESS", value);
                } else {
                    std::env::remove_var("DBUS_SESSION_BUS_ADDRESS");
                }
                if let Some(value) = self.original_runtime.take() {
                    std::env::set_var("XDG_RUNTIME_DIR", value);
                } else {
                    std::env::remove_var("XDG_RUNTIME_DIR");
                }
            }
        }

        #[derive(Clone, Copy)]
        enum MockSecretServiceProbeOutcome {
            Success,
            Unavailable,
            Hangs,
        }

        struct MockLinuxSecretServiceProbe {
            outcome: MockSecretServiceProbeOutcome,
        }

        impl LinuxSecretServiceProbe for MockLinuxSecretServiceProbe {
            fn round_trip(&self) -> Result<(), KeyringError> {
                match self.outcome {
                    MockSecretServiceProbeOutcome::Success => Ok(()),
                    MockSecretServiceProbeOutcome::Unavailable => {
                        Err(KeyringError::PlatformFailure(
                            std::io::Error::other("simulated secret service unavailable").into(),
                        ))
                    }
                    MockSecretServiceProbeOutcome::Hangs => {
                        std::thread::sleep(DESKTOP_SECRET_STORE_PROBE_TIMEOUT * 3);
                        Ok(())
                    }
                }
            }
        }

        #[test]
        fn linux_secret_store_status_from_live_probe_reports_secret_service_when_probe_succeeds(
        ) {
            let _dbus = DbusSessionEnvGuard::with_fake_session_bus();
            let status = linux_secret_store_status_from_live_probes(MockLinuxSecretServiceProbe {
                outcome: MockSecretServiceProbeOutcome::Success,
            });

            assert!(status.available);
            assert_eq!(
                status.backend,
                DESKTOP_SECRET_STORE_BACKEND_LINUX_SECRET_SERVICE
            );
            assert!(status.can_persist_session);
            assert_eq!(status.reason, None);
            assert!(bridge_supports_secure_secret_store(&status));
        }

        #[test]
        fn linux_secret_store_status_reports_unavailable_when_probe_fails_despite_service_files(
        ) {
            let _dbus = DbusSessionEnvGuard::with_fake_session_bus();
            let status = linux_secret_store_status_from_signals_with_reason(
                false,
                false,
                Some(DESKTOP_SECRET_STORE_LINUX_UNAVAILABLE),
            );

            assert!(!status.available);
            assert_eq!(status.backend, DESKTOP_SECRET_STORE_BACKEND_NONE);
            assert!(!status.can_persist_session);
            assert_eq!(status.reason, Some(DESKTOP_SECRET_STORE_LINUX_UNAVAILABLE));

            let live_status = linux_secret_store_status_from_live_probes(MockLinuxSecretServiceProbe {
                outcome: MockSecretServiceProbeOutcome::Unavailable,
            });
            assert_ne!(
                live_status.backend,
                DESKTOP_SECRET_STORE_BACKEND_LINUX_SECRET_SERVICE
            );

            if has_secret_service_backend_service_files() {
                assert!(
                    live_status.backend != DESKTOP_SECRET_STORE_BACKEND_LINUX_SECRET_SERVICE,
                    "service-file heuristic alone must not mark Secret Service available"
                );
            }
        }

        #[test]
        fn linux_secret_store_does_not_trust_service_file_heuristic_without_probe_success() {
            if !has_secret_service_backend_service_files() {
                return;
            }

            let _dbus = DbusSessionEnvGuard::with_fake_session_bus();
            let status = linux_secret_store_status_from_live_probes(MockLinuxSecretServiceProbe {
                outcome: MockSecretServiceProbeOutcome::Unavailable,
            });

            assert!(!status.can_persist_session);
            assert_ne!(
                status.backend,
                DESKTOP_SECRET_STORE_BACKEND_LINUX_SECRET_SERVICE
            );
        }

        #[test]
        fn linux_secret_service_probe_respects_timeout() {
            let started = Instant::now();
            let result = linux_secret_service_probe_with_timeout(MockLinuxSecretServiceProbe {
                outcome: MockSecretServiceProbeOutcome::Hangs,
            });

            assert!(matches!(result, Err(LinuxSecretServiceProbeError::Timeout)));
            assert!(
                started.elapsed() < DESKTOP_SECRET_STORE_PROBE_TIMEOUT * 2,
                "probe should fail within the configured timeout window"
            );
        }

        #[test]
        fn platform_secret_store_status_honors_injected_secret_service_probe_failure() {
            let _dbus = DbusSessionEnvGuard::with_fake_session_bus();
            reset_linux_secret_store_status_cache_for_tests();
            set_linux_secret_service_probe_test_override(Some(Err(KeyringError::PlatformFailure(
                std::io::Error::other("simulated stopped secret service daemon").into(),
            ))));

            let status = platform_secret_store_status();

            assert!(!status.can_persist_session);
            assert_ne!(
                status.backend,
                DESKTOP_SECRET_STORE_BACKEND_LINUX_SECRET_SERVICE
            );
            if !has_linux_keyutils_backend() {
                assert_eq!(status.reason, Some(DESKTOP_SECRET_STORE_LINUX_UNAVAILABLE));
            }
        }

        #[test]
        fn platform_secret_store_status_probes_live_secret_service_when_accessible() {
            reset_linux_secret_store_status_cache_for_tests();
            set_linux_secret_service_probe_test_override(None);

            let status = platform_secret_store_status();

            if has_linux_dbus_session_bus()
                && status.backend == DESKTOP_SECRET_STORE_BACKEND_LINUX_SECRET_SERVICE
            {
                assert!(status.available);
                assert!(status.can_persist_session);
                assert_eq!(status.reason, None);
            }
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn has_linux_keyutils_backend_matches_probe_not_proc_keys_existence() {
        let probe_passes = linux_keyutils_probe_passes();

        assert_eq!(has_linux_keyutils_backend(), probe_passes);

        if Path::new("/proc/keys").exists() && !probe_passes {
            assert!(
                !has_linux_keyutils_backend(),
                "/proc/keys alone must not mark keyutils as available"
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_keyutils_probe_round_trip_is_non_destructive() {
        let first = linux_keyutils_probe_round_trip();
        let second = linux_keyutils_probe_round_trip();

        assert_eq!(first.is_ok(), second.is_ok());
    }

    #[test]
    fn bridge_supports_secure_secret_store_only_when_persistence_is_available() {
        let persistent = linux_secret_store_status_from_signals(true, false);
        let session_scoped = linux_secret_store_status_from_signals(false, true);
        let unavailable = linux_secret_store_status_from_signals(false, false);

        assert!(bridge_supports_secure_secret_store(&persistent));
        assert!(!bridge_supports_secure_secret_store(&session_scoped));
        assert!(!bridge_supports_secure_secret_store(&unavailable));
    }

    #[test]
    fn desktop_session_store_persists_and_reads_sanitized_session_envelopes() {
        let store = TestSessionSecretStore::available();
        let stored = desktop_set_session_in_store(
            &store,
            DesktopSessionEnvelope {
                base_url: " https://matrix.example.org ".to_owned(),
                user_id: " @alice:example.org ".to_owned(),
                device_id: " DEVICEID ".to_owned(),
                access_token: " access-token ".to_owned(),
                refresh_token: Some(" refresh-token ".to_owned()),
                expires_in_ms: Some(3_600_000),
                stored_at_ms: None,
            },
        )
        .expect("session should store");

        assert!(stored);
        let raw = store
            .stored_secret()
            .expect("session secret should be stored");
        assert!(!raw.contains("fallbackSdkStores"));
        let stored_json: serde_json::Value =
            serde_json::from_str(&raw).expect("stored session should be json");
        assert_eq!(stored_json["baseUrl"], "https://matrix.example.org");
        assert_eq!(stored_json["accessToken"], "access-token");

        let session = desktop_get_session_from_store(&store)
            .expect("session should read")
            .expect("session should exist");
        assert_eq!(session.base_url, "https://matrix.example.org");
        assert_eq!(session.user_id, "@alice:example.org");
        assert_eq!(session.device_id, "DEVICEID");
        assert_eq!(session.access_token, "access-token");
        assert_eq!(session.refresh_token.as_deref(), Some("refresh-token"));
    }

    #[test]
    fn desktop_session_store_returns_none_when_no_session_exists() {
        let store = TestSessionSecretStore::available();

        assert!(desktop_get_session_from_store(&store).unwrap().is_none());
    }

    #[test]
    fn desktop_session_store_skips_unavailable_backends() {
        let store = TestSessionSecretStore::unavailable();

        assert!(!desktop_set_session_in_store(&store, valid_session_envelope()).unwrap());
        assert!(desktop_get_session_from_store(&store).unwrap().is_none());
        assert!(!desktop_remove_session_from_store(&store).unwrap());
        assert_eq!(store.stored_secret(), None);
    }

    #[test]
    fn desktop_session_store_rejects_invalid_stored_json_without_echoing_secret() {
        let raw_secret = "not-json-access-token";
        let store = TestSessionSecretStore::with_secret(raw_secret.to_owned());

        let error = desktop_get_session_from_store(&store)
            .err()
            .expect("stored session should fail");

        assert_eq!(error, DESKTOP_STORED_SESSION_INVALID);
        assert!(!error.contains(raw_secret));
    }

    #[test]
    fn session_envelope_expiry_honors_tolerance_and_missing_metadata() {
        let stored_at_ms = 1_000_000;
        let session = DesktopSessionEnvelope {
            stored_at_ms: Some(stored_at_ms),
            expires_in_ms: Some(3_600_000),
            ..valid_session_envelope()
        };

        assert!(!session_envelope_is_expired(&session, stored_at_ms));
        assert!(!session_envelope_is_expired(
            &session,
            stored_at_ms + 3_600_000 + SESSION_EXPIRY_CLOCK_SKEW_TOLERANCE_MS
        ));
        assert!(session_envelope_is_expired(
            &session,
            stored_at_ms + 3_600_000 + SESSION_EXPIRY_CLOCK_SKEW_TOLERANCE_MS + 1
        ));

        let without_expiry = DesktopSessionEnvelope {
            expires_in_ms: None,
            stored_at_ms: Some(stored_at_ms),
            ..valid_session_envelope()
        };
        assert!(!session_envelope_is_expired(&without_expiry, stored_at_ms + 9_999_999));

        let without_stored_at = DesktopSessionEnvelope {
            expires_in_ms: Some(3_600_000),
            stored_at_ms: None,
            ..valid_session_envelope()
        };
        assert!(!session_envelope_is_expired(&without_stored_at, stored_at_ms + 9_999_999));
    }

    #[test]
    fn desktop_session_store_clears_expired_sessions_on_read() {
        let store = TestSessionSecretStore::available();
        let stored_at_ms = 1_000_000;
        let mut session = valid_session_envelope();
        session.stored_at_ms = Some(stored_at_ms);
        session.expires_in_ms = Some(60_000);

        let secret = serde_json::to_string(&session).expect("session should encode");
        store.set_secret(&secret).expect("session should store");
        assert!(desktop_get_session_from_store_at(&store, stored_at_ms + 30_000)
            .expect("session should read")
            .is_some());
        assert!(desktop_get_session_from_store_at(
            &store,
            stored_at_ms + 60_000 + SESSION_EXPIRY_CLOCK_SKEW_TOLERANCE_MS + 1
        )
        .expect("expired session should read as none")
        .is_none());
        assert_eq!(store.stored_secret(), None);
    }

    #[test]
    fn desktop_session_store_removes_existing_session() {
        let store = TestSessionSecretStore::with_secret(
            serde_json::to_string(&valid_session_envelope()).expect("session should encode"),
        );

        assert!(desktop_remove_session_from_store(&store).unwrap());
        assert_eq!(store.stored_secret(), None);
        assert!(!desktop_remove_session_from_store(&store).unwrap());
    }

    #[test]
    fn desktop_set_session_validates_payload_before_storage() {
        let store = TestSessionSecretStore::available();
        let mut session = valid_session_envelope();
        session.access_token = " ".to_owned();

        assert!(desktop_set_session_in_store(&store, session).is_err());
        assert_eq!(store.stored_secret(), None);
    }

    struct FailingSessionSecretStore {
        status: DesktopSecretStoreStatus,
        set_error: String,
    }

    impl FailingSessionSecretStore {
        fn with_set_error(set_error: String) -> Self {
            Self {
                status: available_test_secret_store_status(),
                set_error,
            }
        }
    }

    impl DesktopSessionSecretStore for FailingSessionSecretStore {
        fn status(&self) -> DesktopSecretStoreStatus {
            self.status
        }

        fn get_secret(&self) -> Result<Option<String>, String> {
            Ok(None)
        }

        fn set_secret(&self, _secret: &str) -> Result<bool, String> {
            Err(self.set_error.clone())
        }

        fn remove_secret(&self) -> Result<bool, String> {
            Ok(false)
        }
    }

    #[test]
    fn map_keyring_error_reports_locked_when_storage_is_inaccessible() {
        let error = map_keyring_error(
            "write-session",
            KeyringError::NoStorageAccess(Box::new(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "keychain locked",
            ))),
        );

        assert_eq!(error, DESKTOP_SECRET_STORE_OPERATION_LOCKED);
    }

    #[test]
    fn map_keyring_error_reports_denied_when_access_is_blocked() {
        let error = map_keyring_error(
            "write-session",
            KeyringError::NoStorageAccess(Box::new(std::io::Error::other(
                "test keychain error -25293",
            ))),
        );

        assert_eq!(error, DESKTOP_SECRET_STORE_OPERATION_DENIED);
    }

    #[test]
    fn map_keyring_error_reports_unavailable_on_platform_failures() {
        let error = map_keyring_error(
            "write-session",
            KeyringError::PlatformFailure(Box::new(std::io::Error::other(
                "dbus service unavailable",
            ))),
        );

        assert_eq!(error, DESKTOP_SECRET_STORE_OPERATION_UNAVAILABLE);
    }

    #[test]
    fn map_keyring_error_never_echoes_sensitive_payloads() {
        let secret_payload = r#"{"accessToken":"super-secret-token","baseUrl":"https://x"}"#;
        let error = map_keyring_error(
            "write-session",
            KeyringError::PlatformFailure(Box::new(std::io::Error::other(secret_payload))),
        );

        assert_eq!(error, DESKTOP_SECRET_STORE_OPERATION_UNAVAILABLE);
        assert!(!error.contains("super-secret-token"));
        assert!(!error.contains("accessToken"));
    }

    #[test]
    fn desktop_set_session_reports_locked_when_keychain_is_locked() {
        let locked_error = map_keyring_error(
            "write-session",
            KeyringError::NoStorageAccess(Box::new(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "keychain locked",
            ))),
        );
        let store = FailingSessionSecretStore::with_set_error(locked_error);
        let session = valid_session_envelope();
        let access_token = session.access_token.clone();

        let error = desktop_set_session_in_store(&store, session)
            .err()
            .expect("set session should fail");

        assert_eq!(error, DESKTOP_SECRET_STORE_OPERATION_LOCKED);
        assert!(!error.contains(&access_token));
        assert!(!error.contains("access-token"));
    }

    #[test]
    fn desktop_set_session_error_never_contains_session_json() {
        let unavailable_error = map_keyring_error(
            "write-session",
            KeyringError::PlatformFailure(Box::new(std::io::Error::other(
                "secret service write failed",
            ))),
        );
        let store = FailingSessionSecretStore::with_set_error(unavailable_error);
        let session = valid_session_envelope();
        let session_json =
            serde_json::to_string(&session).expect("session envelope should encode");

        let error = desktop_set_session_in_store(&store, session)
            .err()
            .expect("set session should fail");

        assert_eq!(error, DESKTOP_SECRET_STORE_OPERATION_UNAVAILABLE);
        assert!(!error.contains(&session_json));
        assert!(!error.contains("access-token"));
        assert!(!error.contains("matrix.example.org"));
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
    fn shortcut_slot_helpers_detect_rebind_and_retired_shortcuts() {
        let previous = DesktopShortcutConfig {
            show: "CmdOrCtrl+Shift+C".to_string(),
            later: "CmdOrCtrl+Shift+L".to_string(),
            notifications: "CmdOrCtrl+Shift+N".to_string(),
        };
        let swapped = DesktopShortcutConfig {
            show: "CmdOrCtrl+Shift+L".to_string(),
            later: "CmdOrCtrl+Shift+C".to_string(),
            notifications: "CmdOrCtrl+Shift+N".to_string(),
        };

        assert_eq!(
            shortcuts_needing_registration(Some(&previous), &swapped),
            vec![
                "CmdOrCtrl+Shift+L".to_string(),
                "CmdOrCtrl+Shift+C".to_string()
            ]
        );
        assert_eq!(
            shortcuts_needing_handler_rebind(&previous, &swapped),
            vec![
                "CmdOrCtrl+Shift+L".to_string(),
                "CmdOrCtrl+Shift+C".to_string()
            ]
        );
        assert!(retired_shortcut_strings(&previous, &swapped).is_empty());
    }

    #[test]
    fn shortcut_slot_helpers_detect_retired_shortcuts_on_replacement() {
        let previous = DesktopShortcutConfig {
            show: "CmdOrCtrl+Shift+C".to_string(),
            later: "CmdOrCtrl+Shift+L".to_string(),
            notifications: "CmdOrCtrl+Shift+N".to_string(),
        };
        let replaced = DesktopShortcutConfig {
            show: "CmdOrCtrl+Shift+1".to_string(),
            later: "CmdOrCtrl+Shift+2".to_string(),
            notifications: "CmdOrCtrl+Shift+3".to_string(),
        };

        assert_eq!(
            shortcuts_needing_registration(Some(&previous), &replaced),
            vec![
                "CmdOrCtrl+Shift+1".to_string(),
                "CmdOrCtrl+Shift+2".to_string(),
                "CmdOrCtrl+Shift+3".to_string()
            ]
        );
        assert_eq!(
            retired_shortcut_strings(&previous, &replaced),
            vec![
                "CmdOrCtrl+Shift+C".to_string(),
                "CmdOrCtrl+Shift+L".to_string(),
                "CmdOrCtrl+Shift+N".to_string()
            ]
        );
    }

    #[test]
    fn shortcut_state_classifier_detects_permission_errors_and_result_shapes() {
        assert_eq!(
            shortcut_state_from_error("failed with denied"),
            DesktopShortcutApplyState::PermissionNeeded
        );
        assert_eq!(
            shortcut_state_from_error("shortcut unsupported on this build"),
            DesktopShortcutApplyState::Unsupported
        );

        let result = shortcut_result(DesktopShortcutApplyState::PermissionNeeded, None, None);
        assert!(!result.success);
        assert_eq!(result.state, DesktopShortcutApplyState::PermissionNeeded);
        assert!(result.message.contains("permission"));
        assert!(result.message.contains("shortcut"));
    }

    #[test]
    fn shortcut_permission_fallback_is_kde_wayland_only() {
        let _guard = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let original_desktop = std::env::var("XDG_CURRENT_DESKTOP").ok();
        let original_wayland = std::env::var("WAYLAND_DISPLAY").ok();

        std::env::remove_var("XDG_CURRENT_DESKTOP");
        std::env::remove_var("WAYLAND_DISPLAY");
        let generic = shortcut_result(DesktopShortcutApplyState::PermissionNeeded, None, None);
        assert!(generic.fallback_command.is_none());
        assert!(!generic.message.to_ascii_lowercase().contains("kde plasma wayland"));

        std::env::set_var("XDG_CURRENT_DESKTOP", "KDE");
        std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
        let kde = shortcut_result(DesktopShortcutApplyState::PermissionNeeded, None, None);
        assert!(kde.fallback_command.is_some());
        assert!(kde.message.to_ascii_lowercase().contains("kde plasma wayland"));

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
    fn unresolved_shortcut_state_is_unknown_on_kde_wayland_before_apply() {
        let _guard = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let original_desktop = std::env::var("XDG_CURRENT_DESKTOP").ok();
        let original_wayland = std::env::var("WAYLAND_DISPLAY").ok();

        std::env::set_var("XDG_CURRENT_DESKTOP", "KDE");
        std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
        assert_eq!(
            unresolved_shortcut_apply_state(),
            DesktopShortcutApplyState::Unknown
        );

        std::env::remove_var("WAYLAND_DISPLAY");
        std::env::remove_var("XDG_SESSION_TYPE");
        assert_eq!(
            unresolved_shortcut_apply_state(),
            DesktopShortcutApplyState::Failed
        );

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
        assert!(!should_stream_file_ipc(DESKTOP_FILE_IPC_INLINE_THRESHOLD as u64));
        assert!(should_stream_file_ipc(
            DESKTOP_FILE_IPC_INLINE_THRESHOLD as u64 + 1
        ));
    }

    #[test]
    fn dropped_file_read_mode_selects_inline_or_streamed_transfer() {
        assert_eq!(
            dropped_file_read_mode(DESKTOP_FILE_IPC_INLINE_THRESHOLD as u64),
            DroppedFileReadMode::Inline
        );
        assert_eq!(
            dropped_file_read_mode(DESKTOP_FILE_IPC_INLINE_THRESHOLD as u64 + 1),
            DroppedFileReadMode::Streamed
        );
    }

    #[test]
    fn desktop_save_file_rejects_inline_payload_over_threshold() {
        let oversized = vec![0_u8; DESKTOP_FILE_IPC_INLINE_THRESHOLD + 1];
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

        let total_size = (DESKTOP_FILE_IPC_INLINE_THRESHOLD + 1) as u64;
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
            saved_bytes[DESKTOP_FILE_IPC_INLINE_THRESHOLD],
            ((DESKTOP_FILE_IPC_INLINE_THRESHOLD as u64 % 251) + 1) as u8
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
        let total_size = (DESKTOP_FILE_IPC_INLINE_THRESHOLD + 512) as u64;
        let payload = vec![9_u8; total_size as usize];
        fs::write(&file_path, &payload).expect("dropped file fixture should be written");

        remember_dropped_paths(&[file_path.clone()]);
        let descriptors = desktop_read_dropped_files(vec![file_path.to_string_lossy().into_owned()])
            .expect("dropped file metadata should be returned");
        assert_eq!(descriptors.len(), 1);
        assert!(descriptors[0].bytes.is_none());
        let transfer_id = descriptors[0]
            .transfer_id
            .clone()
            .expect("streamed transfer id should be present");
        assert_eq!(descriptors[0].size, Some(total_size));

        let first_chunk = desktop_read_dropped_file_chunk(
            transfer_id.clone(),
            0,
            DESKTOP_FILE_IPC_CHUNK_SIZE,
        )
        .expect("first chunk should be readable");
        assert_eq!(first_chunk.len(), DESKTOP_FILE_IPC_CHUNK_SIZE);
        assert!(first_chunk.iter().all(|byte| *byte == 9));

        let second_chunk = desktop_read_dropped_file_chunk(
            transfer_id.clone(),
            DESKTOP_FILE_IPC_INLINE_THRESHOLD as u64,
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
            fs::write(&file_path, format!("payload-{index}")).expect("drop fixture should be written");
            paths.push(file_path);
        }

        remember_dropped_paths(&paths);
        assert!(dropped_file_allowlist_len_for_tests() <= MAX_DROPPED_FILE_ALLOWLIST);
        assert_eq!(dropped_file_allowlist_len_for_tests(), MAX_DROPPED_FILE_ALLOWLIST);

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

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn global_shortcut_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    // Global shortcuts are registered only via `desktop_set_shortcuts` after the
    // frontend DesktopShortcutSync mounts. Until then, no shortcuts are active.
    tauri_plugin_global_shortcut::Builder::new().build()
}
