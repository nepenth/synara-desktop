use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fs::{create_dir_all, metadata, remove_file, rename, File, OpenOptions, Permissions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, Runtime};

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const LOG_FILE_NAME: &str = "synara-desktop.log";
const DIAGNOSTICS_FILE_NAME: &str = "synara-diagnostics.jsonl";
const MAX_LOG_FIELD_LEN: usize = 4_000;
const MAX_LOG_FILE_BYTES: u64 = 5 * 1024 * 1024;
const MAX_DIAGNOSTICS_FILE_BYTES: u64 = 5 * 1024 * 1024;
const MAX_DIAGNOSTIC_RECORD_BYTES: usize = 4_096;
// The frontend accepts up to 20 caller fields, then adds five bounded envelope fields.
const MAX_DIAGNOSTIC_FIELDS: usize = 32;
const DIAGNOSTICS_RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const DIAGNOSTICS_RETENTION_DAYS: u64 = 7;
const DIAGNOSTICS_SCHEMA_VERSION: u8 = 1;
const DIAGNOSTICS_ERROR_UNAVAILABLE: &str = "diagnostics-unavailable";
const DIAGNOSTICS_FULL_SCAN_INTERVAL: usize = 256;
static LOG_WRITE_LOCK: Mutex<()> = Mutex::new(());
static DIAGNOSTICS_PRUNE_TICK: AtomicUsize = AtomicUsize::new(0);

const SESSION_EVENT_NAMESPACES: &[&str] = &[
    "bootstrap.",
    "crypto-continuity.",
    "expired-session.",
    "initial-sync.",
    "matrix-client.",
    "matrix-crypto.",
    "matrix-store.",
    "migration.",
    "persistence.",
    "persisted-session-clear.",
    "platform-store.",
    "session.",
    "sync.",
    "token-refresh.",
];
const PERFORMANCE_EVENT_NAMESPACES: &[&str] = &[
    "matrix-crypto.",
    "matrix-store.",
    "performance.",
    "room-timeline.",
    "runtime.",
    "virtual-paginator.",
];
const ROOM_EVENT_NAMESPACES: &[&str] = &["marker.", "room-activity.", "room-timeline."];

const NUMBER_FIELDS: &[&str] = &[
    "sequence",
    "traceSequence",
    "uptimeMs",
    "durationMs",
    "elapsedMs",
    "ageMs",
    "expiresInMs",
    "retryCount",
    "attempt",
    "generation",
    "revision",
    "eventCount",
    "linkedEventCount",
    "renderedRowCount",
    "rowCount",
    "rowIndex",
    "previousRowIndex",
    "offsetTop",
    "scrollTop",
    "previousScrollTop",
    "scrollDelta",
    "scrollHeight",
    "previousScrollHeight",
    "heightDelta",
    "viewportHeight",
    "bottomGap",
    "previousBottomGap",
    "totalSize",
    "totalSizeDelta",
    "anchorCorrection",
    "maxScrollDelta",
    "maxVelocity",
    "stableFrames",
    "waiterCount",
    "queueDepth",
    "coalescedCount",
    "fps",
    "longTaskCount",
    "lastLongTaskMs",
    "maxLongTaskMs",
    "memoryMb",
    "requestDurationMs",
    "nativeWriteDurationMs",
];
const BOOLEAN_FIELDS: &[&str] = &[
    "available",
    "success",
    "hasSession",
    "hasRefreshToken",
    "hasExpiry",
    "freshLogin",
    "identityCleared",
    "fallbackPresent",
    "nativeStoreAvailable",
    "nativeStoreError",
    "bridgeAvailable",
    "canPersistSession",
    "hasUnreadTarget",
    "hasUnreadSignal",
    "readFrontierAtLiveTail",
    "unreadInInitialWindow",
    "hasSavedViewport",
    "savedViewportAtBottom",
    "savedAnchorPresent",
    "anchorInWindow",
    "restoredSavedViewport",
    "loadedAtEnd",
    "liveTailRecorded",
    "liveEndPinned",
    "atBottom",
    "userScrolling",
    "programmaticScroll",
    "structuralUpdateQueued",
    "timedOut",
    "confirmed",
    "fromLiveTimeline",
    "privateReceipt",
    "publicReceipt",
    "hasConcreteHead",
    "preservedSummary",
    "activityChanged",
    "latestChanged",
    "enabled",
    "boundedContextsEnabled",
    "stableAnchoringEnabled",
    "documentVisible",
    "documentFocused",
    "online",
    "nativeStoreConfigured",
    "hasExpiryMetadata",
    "fallbackSdkStores",
    "fallbackUsed",
    "expired",
    "identityStoresCleared",
    "continuityConfirmationPending",
    "nativeRemovalError",
    "matrixStoreClearSuccess",
];
const LABEL_FIELDS: &[&str] = &[
    "appRunId",
    "roomToken",
    "eventToken",
    "traceId",
    "openMode",
    "source",
    "target",
    "status",
    "outcome",
    "phase",
    "direction",
    "errorType",
    "reason",
    "eventType",
    "msgtype",
    "mode",
    "queueState",
    "feature",
    "backend",
    "persistence",
    "continuity",
    "syncState",
    "previousSyncState",
    "inputKind",
    "writer",
    "navigationPhase",
    "readFrontierSource",
];
const RANGE_FIELDS: &[&str] = &["range", "virtualRange", "previousVirtualRange"];
const RANGE_KEYS: &[&str] = &["start", "end", "startIndex", "endIndex"];

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesktopDiagnosticRecord {
    schema_version: u8,
    timestamp_ms: u64,
    category: String,
    event: String,
    fields: Map<String, Value>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDiagnosticsStatus {
    available: bool,
    has_data: bool,
    size_bytes: u64,
    current_bytes: u64,
    rotated_bytes: u64,
    total_bytes: u64,
    entry_count: u64,
    discarded_entries: u64,
    oldest_timestamp_ms: Option<u64>,
    newest_timestamp_ms: Option<u64>,
    retention_days: u64,
    max_total_bytes: u64,
    error_code: Option<&'static str>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDiagnosticsReport {
    schema_version: u8,
    generated_at_ms: u64,
    app_version: &'static str,
    build_revision: &'static str,
    build_branch: &'static str,
    os: &'static str,
    architecture: &'static str,
    status: DesktopDiagnosticsStatus,
    entries: Vec<DesktopDiagnosticRecord>,
}

fn diagnostics_paths(log_dir: &Path) -> (PathBuf, PathBuf) {
    let current = log_dir.join(DIAGNOSTICS_FILE_NAME);
    let rotated = current.with_extension("jsonl.1");
    (current, rotated)
}

fn rotated_path(path: &Path) -> PathBuf {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some(extension) => path.with_extension(format!("{extension}.1")),
        None => path.with_extension("1"),
    }
}

fn rotate_file_if_needed(log_path: &Path, max_bytes: u64, incoming_bytes: u64) {
    if metadata(log_path).map_or(true, |entry| {
        entry.len().saturating_add(incoming_bytes) <= max_bytes
    }) {
        return;
    }

    let backup_path = rotated_path(log_path);
    let _ = remove_file(&backup_path);
    if let Err(error) = rename(log_path, &backup_path) {
        eprintln!("[synara] failed to rotate app log: {error}");
    }
    set_private_file_permissions(&backup_path);
}

fn sanitize_log_field(value: &str) -> String {
    static SECRET_VALUE: OnceLock<Regex> = OnceLock::new();
    static BEARER_VALUE: OnceLock<Regex> = OnceLock::new();
    let secret_value = SECRET_VALUE.get_or_init(|| {
        Regex::new(
            r#"(?i)(access[_-]?token|refresh[_-]?token|authorization|password)([\"']?\s*[:=]\s*)(?:bearer\s+[a-z0-9._~+/=-]+|\"[^\"]*\"|'[^']*'|[^\s,;}]+)"#,
        )
        .expect("static secret-value redaction regex")
    });
    let bearer_value = BEARER_VALUE.get_or_init(|| {
        Regex::new(r"(?i)\bbearer\s+[a-z0-9._~+/=-]+").expect("static bearer redaction regex")
    });
    let normalized = value
        .replace(['\n', '\r'], " ")
        .replace("\\\"", "\"")
        .replace("\\'", "'");
    let redacted = secret_value.replace_all(&normalized, "$1$2[redacted]");
    let redacted = bearer_value.replace_all(&redacted, "Bearer [redacted]");
    redacted.chars().take(MAX_LOG_FIELD_LEN).collect()
}

fn timestamp_ms() -> u64 {
    system_time_ms(SystemTime::now())
}

fn system_time_ms(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn diagnostics_cutoff_ms(now: SystemTime) -> u64 {
    system_time_ms(now).saturating_sub(DIAGNOSTICS_RETENTION.as_millis() as u64)
}

fn set_private_file_permissions(path: &Path) {
    #[cfg(unix)]
    {
        let _ = std::fs::set_permissions(path, Permissions::from_mode(0o600));
    }

    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

fn open_private_append(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    options.mode(0o600);
    let file = options.open(path)?;
    set_private_file_permissions(path);
    Ok(file)
}

fn open_private_truncate(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    options.mode(0o600);
    let file = options.open(path)?;
    set_private_file_permissions(path);
    Ok(file)
}

pub fn append_app_log<R: Runtime>(app: &AppHandle<R>, source: &str, message: &str) {
    let _write_guard = LOG_WRITE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Ok(log_dir) = app.path().app_log_dir() else {
        eprintln!("[synara] failed to resolve app log directory");
        return;
    };
    if let Err(error) = create_dir_all(&log_dir) {
        eprintln!("[synara] failed to create app log directory: {error}");
        return;
    }

    let log_path = log_dir.join(LOG_FILE_NAME);
    let source = sanitize_log_field(source);
    let message = sanitize_log_field(message);
    let line = format!("[{}] {} {}\n", timestamp_ms(), source, message);
    rotate_file_if_needed(&log_path, MAX_LOG_FILE_BYTES, line.len() as u64);

    match open_private_append(&log_path) {
        Ok(mut file) => {
            if let Err(error) = file.write_all(line.as_bytes()) {
                eprintln!("[synara] failed to write app log: {error}");
            }
        }
        Err(error) => {
            eprintln!("[synara] failed to open app log: {error}");
        }
    }
}

fn allowed_event(category: &str, event: &str) -> bool {
    let namespaces = match category {
        "performance" => PERFORMANCE_EVENT_NAMESPACES,
        "session" => SESSION_EVENT_NAMESPACES,
        "room" => ROOM_EVENT_NAMESPACES,
        _ => return false,
    };
    (3..=64).contains(&event.len())
        && event.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
        })
        && !event.contains("..")
        && namespaces
            .iter()
            .any(|namespace| event.starts_with(namespace))
}

fn is_bounded_number(value: &Value) -> bool {
    value
        .as_f64()
        .is_some_and(|number| number.is_finite() && number.abs() <= 1_000_000_000_000.0)
}

fn is_random_token(value: &str) -> bool {
    (4..=64).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn is_index_token(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|suffix| {
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn is_safe_label_value(value: &str) -> bool {
    (1..=64).contains(&value.len())
        && !value.contains("://")
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'-')
        })
}

fn allowed_label(field: &str, value: &str) -> bool {
    if !LABEL_FIELDS.contains(&field) || !is_safe_label_value(value) {
        return false;
    }
    match field {
        "appRunId" => value.starts_with("run-") && is_random_token(value),
        "traceId" => is_random_token(value),
        "roomToken" => is_index_token(value, "room-"),
        "eventToken" => is_index_token(value, "event-"),
        _ => true,
    }
}

fn sanitize_range(value: &Value) -> Option<Value> {
    let source = value.as_object()?;
    if source.len() > RANGE_KEYS.len()
        || source.keys().any(|key| !RANGE_KEYS.contains(&key.as_str()))
    {
        return None;
    }
    let mut range = Map::new();
    for (key, value) in source {
        if !is_bounded_number(value) {
            return None;
        }
        range.insert(key.clone(), value.clone());
    }
    Some(Value::Object(range))
}

fn sanitize_diagnostic_fields(fields: Value) -> Option<Map<String, Value>> {
    let fields = fields.as_object()?;
    if fields.len() > MAX_DIAGNOSTIC_FIELDS {
        return None;
    }

    let mut sanitized = Map::new();
    for (key, value) in fields {
        let accepted = if NUMBER_FIELDS.contains(&key.as_str()) {
            is_bounded_number(value).then(|| value.clone())
        } else if BOOLEAN_FIELDS.contains(&key.as_str()) {
            value.as_bool().map(Value::Bool)
        } else if RANGE_FIELDS.contains(&key.as_str()) {
            sanitize_range(value)
        } else if let Some(label) = value.as_str() {
            allowed_label(key, label).then(|| Value::String(label.to_owned()))
        } else {
            None
        }?;
        sanitized.insert(key.clone(), accepted);
    }
    Some(sanitized)
}

fn build_diagnostic_record(
    category: String,
    event: String,
    fields: Value,
    now_ms: u64,
) -> Option<DesktopDiagnosticRecord> {
    if !allowed_event(&category, &event) {
        return None;
    }
    Some(DesktopDiagnosticRecord {
        schema_version: DIAGNOSTICS_SCHEMA_VERSION,
        timestamp_ms: now_ms,
        category,
        event,
        fields: sanitize_diagnostic_fields(fields)?,
    })
}

fn should_prune(modified: SystemTime, now: SystemTime) -> bool {
    now.duration_since(modified)
        .is_ok_and(|age| age > DIAGNOSTICS_RETENTION)
}

fn diagnostic_record_is_valid(record: &DesktopDiagnosticRecord) -> bool {
    record.schema_version == DIAGNOSTICS_SCHEMA_VERSION
        && allowed_event(&record.category, &record.event)
        && sanitize_diagnostic_fields(Value::Object(record.fields.clone()))
            == Some(record.fields.clone())
}

fn diagnostic_file_needs_compaction(path: &Path, cutoff_ms: u64, full_scan: bool) -> bool {
    let Ok(file) = File::open(path) else {
        return false;
    };
    let record_needs_compaction = |line: &str| {
        serde_json::from_str::<DesktopDiagnosticRecord>(line).map_or(true, |record| {
            !diagnostic_record_is_valid(&record) || record.timestamp_ms < cutoff_ms
        })
    };
    let mut reader = BufReader::new(file);
    let mut first_line = String::new();
    match reader.read_line(&mut first_line) {
        Ok(0) => false,
        Ok(_) if record_needs_compaction(&first_line) => true,
        Ok(_) if !full_scan => false,
        Ok(_) => reader
            .lines()
            .any(|line| line.map_or(true, |value| record_needs_compaction(&value))),
        Err(_) => true,
    }
}

fn rewrite_diagnostic_file(path: &Path, records: &[DesktopDiagnosticRecord]) -> bool {
    if records.is_empty() {
        return match remove_file(path) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
            Err(_) => false,
        };
    }

    let temp_path = path.with_extension("prune-tmp");
    let write_result = (|| -> std::io::Result<()> {
        let mut file = open_private_truncate(&temp_path)?;
        for record in records {
            let mut line = serde_json::to_vec(record).map_err(std::io::Error::other)?;
            line.push(b'\n');
            if line.len() > MAX_DIAGNOSTIC_RECORD_BYTES {
                return Err(std::io::Error::other("diagnostic-record-too-large"));
            }
            file.write_all(&line)?;
        }
        file.sync_all()
    })();
    if write_result.is_err() {
        let _ = remove_file(&temp_path);
        return false;
    }
    if rename(&temp_path, path).is_err() {
        let _ = remove_file(&temp_path);
        return false;
    }
    set_private_file_permissions(path);
    true
}

fn compact_diagnostic_file(path: &Path, cutoff_ms: u64) -> bool {
    let mut retained = Vec::new();
    read_diagnostic_file(path, &mut retained, cutoff_ms);
    rewrite_diagnostic_file(path, &retained)
}

fn prune_expired_diagnostics(log_dir: &Path, now: SystemTime) {
    let cutoff_ms = diagnostics_cutoff_ms(now);
    let full_scan = DIAGNOSTICS_PRUNE_TICK
        .fetch_add(1, Ordering::Relaxed)
        .is_multiple_of(DIAGNOSTICS_FULL_SCAN_INTERVAL);
    let (current, rotated) = diagnostics_paths(log_dir);
    for path in [current, rotated] {
        if metadata(&path)
            .and_then(|entry| entry.modified())
            .is_ok_and(|modified| should_prune(modified, now))
        {
            let _ = remove_file(path);
        } else if diagnostic_file_needs_compaction(&path, cutoff_ms, full_scan) {
            let _ = compact_diagnostic_file(&path, cutoff_ms);
        }
    }
}

fn append_diagnostic_record_to_dir(
    log_dir: &Path,
    record: &DesktopDiagnosticRecord,
    max_bytes: u64,
) -> bool {
    if create_dir_all(log_dir).is_err() {
        return false;
    }
    let Ok(mut line) = serde_json::to_vec(record) else {
        return false;
    };
    line.push(b'\n');
    if line.len() > MAX_DIAGNOSTIC_RECORD_BYTES {
        return false;
    }

    let (current, _) = diagnostics_paths(log_dir);
    rotate_file_if_needed(&current, max_bytes, line.len() as u64);
    if metadata(&current)
        .is_ok_and(|entry| entry.len().saturating_add(line.len() as u64) > max_bytes)
    {
        return false;
    }
    open_private_append(&current)
        .and_then(|mut file| file.write_all(&line))
        .is_ok()
}

fn read_diagnostic_file(
    path: &Path,
    entries: &mut Vec<DesktopDiagnosticRecord>,
    cutoff_ms: u64,
) -> u64 {
    let Ok(file) = File::open(path) else {
        return 0;
    };
    let mut discarded = 0;
    for line in BufReader::new(file).lines() {
        let Ok(line) = line else {
            discarded += 1;
            continue;
        };
        match serde_json::from_str::<DesktopDiagnosticRecord>(&line) {
            Ok(record)
                if diagnostic_record_is_valid(&record) && record.timestamp_ms >= cutoff_ms =>
            {
                entries.push(record);
            }
            _ => discarded += 1,
        }
    }
    discarded
}

fn unavailable_status() -> DesktopDiagnosticsStatus {
    DesktopDiagnosticsStatus {
        available: false,
        has_data: false,
        size_bytes: 0,
        current_bytes: 0,
        rotated_bytes: 0,
        total_bytes: 0,
        entry_count: 0,
        discarded_entries: 0,
        oldest_timestamp_ms: None,
        newest_timestamp_ms: None,
        retention_days: DIAGNOSTICS_RETENTION_DAYS,
        max_total_bytes: MAX_DIAGNOSTICS_FILE_BYTES * 2,
        error_code: Some(DIAGNOSTICS_ERROR_UNAVAILABLE),
    }
}

fn diagnostics_report_from_dir(log_dir: &Path, cutoff_ms: u64) -> DesktopDiagnosticsReport {
    let (current, rotated) = diagnostics_paths(log_dir);
    let mut rotated_entries = Vec::new();
    let rotated_discarded = read_diagnostic_file(&rotated, &mut rotated_entries, cutoff_ms);
    if rotated_discarded > 0 {
        let _ = rewrite_diagnostic_file(&rotated, &rotated_entries);
    }
    let mut current_entries = Vec::new();
    let current_discarded = read_diagnostic_file(&current, &mut current_entries, cutoff_ms);
    if current_discarded > 0 {
        let _ = rewrite_diagnostic_file(&current, &current_entries);
    }

    let current_bytes = metadata(&current).map_or(0, |entry| entry.len());
    let rotated_bytes = metadata(&rotated).map_or(0, |entry| entry.len());
    let mut entries = rotated_entries;
    entries.extend(current_entries);
    let discarded_entries = rotated_discarded.saturating_add(current_discarded);
    let oldest_timestamp_ms = entries.iter().map(|record| record.timestamp_ms).min();
    let newest_timestamp_ms = entries.iter().map(|record| record.timestamp_ms).max();
    let total_bytes = current_bytes.saturating_add(rotated_bytes);
    let status = DesktopDiagnosticsStatus {
        available: true,
        has_data: !entries.is_empty(),
        size_bytes: total_bytes,
        current_bytes,
        rotated_bytes,
        total_bytes,
        entry_count: entries.len() as u64,
        discarded_entries,
        oldest_timestamp_ms,
        newest_timestamp_ms,
        retention_days: DIAGNOSTICS_RETENTION_DAYS,
        max_total_bytes: MAX_DIAGNOSTICS_FILE_BYTES * 2,
        error_code: None,
    };
    DesktopDiagnosticsReport {
        schema_version: DIAGNOSTICS_SCHEMA_VERSION,
        generated_at_ms: timestamp_ms(),
        app_version: crate::build_info::app_version(),
        build_revision: crate::build_info::revision(),
        build_branch: crate::build_info::branch(),
        os: std::env::consts::OS,
        architecture: std::env::consts::ARCH,
        status,
        entries,
    }
}

fn clear_diagnostics_from_dir(log_dir: &Path) -> bool {
    let (current, rotated) = diagnostics_paths(log_dir);
    let mut cleared = true;
    for path in [current, rotated] {
        match remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => cleared = false,
        }
    }
    cleared
}

#[tauri::command]
pub fn desktop_append_log(app: AppHandle, source: String, message: String) {
    append_app_log(&app, &source, &message);
}

#[tauri::command]
pub fn desktop_log_path<R: Runtime>(app: AppHandle<R>) -> Result<String, String> {
    app.path()
        .app_log_dir()
        .map(|path| path.join(LOG_FILE_NAME).to_string_lossy().into_owned())
        .map_err(|_| "app-log-path-unavailable".to_owned())
}

#[tauri::command]
pub fn desktop_record_diagnostic(
    app: AppHandle,
    category: String,
    event: String,
    fields: Value,
) -> bool {
    let Some(record) = build_diagnostic_record(category, event, fields, timestamp_ms()) else {
        return false;
    };
    let _write_guard = LOG_WRITE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Ok(log_dir) = app.path().app_log_dir() else {
        return false;
    };
    prune_expired_diagnostics(&log_dir, SystemTime::now());
    append_diagnostic_record_to_dir(&log_dir, &record, MAX_DIAGNOSTICS_FILE_BYTES)
}

#[tauri::command]
pub fn desktop_read_diagnostics(app: AppHandle) -> DesktopDiagnosticsReport {
    let _write_guard = LOG_WRITE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Ok(log_dir) = app.path().app_log_dir() else {
        return DesktopDiagnosticsReport {
            schema_version: DIAGNOSTICS_SCHEMA_VERSION,
            generated_at_ms: timestamp_ms(),
            app_version: crate::build_info::app_version(),
            build_revision: crate::build_info::revision(),
            build_branch: crate::build_info::branch(),
            os: std::env::consts::OS,
            architecture: std::env::consts::ARCH,
            status: unavailable_status(),
            entries: Vec::new(),
        };
    };
    let now = SystemTime::now();
    prune_expired_diagnostics(&log_dir, now);
    diagnostics_report_from_dir(&log_dir, diagnostics_cutoff_ms(now))
}

#[tauri::command]
pub fn desktop_diagnostics_status(app: AppHandle) -> DesktopDiagnosticsStatus {
    desktop_read_diagnostics(app).status
}

#[tauri::command]
pub fn desktop_clear_diagnostics(app: AppHandle) -> bool {
    let _write_guard = LOG_WRITE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Ok(log_dir) = app.path().app_log_dir() else {
        return false;
    };
    clear_diagnostics_from_dir(&log_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn generic_log_redaction_removes_secret_values_and_bearer_credentials() {
        let secret = "syt_super_secret_value";
        for input in [
            format!("access_token={secret}"),
            format!(r#"{{\"refreshToken\":\"{secret}\"}}"#),
            format!("Authorization: Bearer {secret}"),
            format!("password: {secret}"),
        ] {
            let sanitized = sanitize_log_field(&input);
            assert!(!sanitized.contains(secret));
            assert!(sanitized.contains("[redacted]"));
        }
    }

    fn temp_log_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("synara-{label}-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&root).expect("temporary log directory should be created");
        root
    }

    fn valid_record(sequence: u64) -> DesktopDiagnosticRecord {
        build_diagnostic_record(
            "room".to_owned(),
            "room-timeline.open".to_owned(),
            json!({
                "appRunId": "run-lz123abc-1234abcd",
                "traceId": "trace-1234",
                "roomToken": "room-1",
                "sequence": sequence,
                "uptimeMs": 1234,
                "openMode": "live-end",
                "loadedAtEnd": true
            }),
            1_000 + sequence,
        )
        .expect("record should be valid")
    }

    #[test]
    fn rotates_logs_at_the_size_limit() {
        let root = temp_log_dir("log");
        let log_path = root.join("synara-desktop.log");
        std::fs::write(&log_path, b"0123456789").expect("test log should be written");

        rotate_file_if_needed(&log_path, 10, 1);

        assert!(!log_path.exists());
        assert_eq!(
            std::fs::read(root.join("synara-desktop.log.1")).expect("rotated log should exist"),
            b"0123456789"
        );
        std::fs::remove_dir_all(root).expect("temporary log directory should be removed");
    }

    #[test]
    fn diagnostic_schema_accepts_only_allowlisted_content() {
        let record = valid_record(1);
        assert_eq!(record.category, "room");
        assert_eq!(record.fields.get("roomToken"), Some(&json!("room-1")));

        assert!(build_diagnostic_record(
            "room".to_owned(),
            "room-timeline.open".to_owned(),
            json!({"messageBody": "private text"}),
            1,
        )
        .is_none());
        assert!(build_diagnostic_record(
            "room".to_owned(),
            "unknown-event".to_owned(),
            json!({}),
            1,
        )
        .is_none());
        assert!(build_diagnostic_record(
            "unknown".to_owned(),
            "room-timeline.open".to_owned(),
            json!({}),
            1,
        )
        .is_none());
        assert!(build_diagnostic_record(
            "room".to_owned(),
            "room-timeline.open".to_owned(),
            json!({"traceId": "https://matrix.example.org/private"}),
            1,
        )
        .is_none());
        assert!(build_diagnostic_record(
            "room".to_owned(),
            "room-timeline.open".to_owned(),
            json!({"roomToken": "!room:example.org"}),
            1,
        )
        .is_none());
        assert!(build_diagnostic_record(
            "session".to_owned(),
            "bootstrap.completed".to_owned(),
            json!({"appRunId": "run-lz123abc-1234abcd", "outcome": "success"}),
            1,
        )
        .is_some());
        assert!(build_diagnostic_record(
            "performance".to_owned(),
            "runtime.sample".to_owned(),
            json!({"appRunId": "run-lz123abc-1234abcd", "fps": 59.8}),
            1,
        )
        .is_some());
    }

    #[test]
    fn diagnostic_schema_accepts_production_frontend_event_names() {
        let production_events = [
            ("performance", "runtime.sample"),
            ("performance", "room-timeline.slow-event-render"),
            ("session", "bootstrap.started"),
            ("session", "bootstrap.native-read-completed"),
            ("session", "bootstrap.completed"),
            ("session", "bootstrap.failed"),
            ("session", "crypto-continuity.completed"),
            ("session", "crypto-continuity.post-sync-completed"),
            ("session", "expired-session.reconciliation-completed"),
            ("session", "initial-sync.prepared"),
            ("session", "matrix-client.bootstrap-completed"),
            ("session", "matrix-client.bootstrap-decision"),
            ("session", "matrix-client.initialization-completed"),
            ("session", "matrix-client.initialization-started"),
            ("session", "matrix-client.start-call-completed"),
            ("session", "matrix-client.start-requested"),
            ("session", "matrix-crypto.initialization-completed"),
            ("session", "matrix-store.startup-completed"),
            ("session", "migration.completed"),
            ("session", "persisted-session-clear.completed"),
            ("session", "persistence.completed"),
            ("session", "persistence.native-write-completed"),
            ("session", "persistence.started"),
            ("session", "platform-store.read-completed"),
            ("session", "platform-store.remove-completed"),
            ("session", "platform-store.status-completed"),
            ("session", "platform-store.write-completed"),
            ("session", "sync.prepared-timeout"),
            ("session", "sync.recovery-requested"),
            ("session", "sync.resume-retry"),
            ("session", "sync.transition"),
            ("session", "token-refresh.completed"),
            ("session", "token-refresh.started"),
            ("room", "marker.already-current"),
            ("room", "marker.commit-failed"),
            ("room", "marker.commit-success"),
            ("room", "marker.legacy-failed"),
            ("room", "marker.legacy-success"),
            ("room", "room-activity.records-coalesced"),
            ("room", "room-activity.rollout-mode"),
            ("room", "room-activity.updated"),
            ("room", "room-timeline.anchor-restore-cancelled"),
            ("room", "room-timeline.anchor-restored"),
            ("room", "room-timeline.defer-empty-refresh"),
            ("room", "room-timeline.first-stable-bottom"),
            ("room", "room-timeline.jump-latest"),
            ("room", "room-timeline.jump-latest-failed"),
            ("room", "room-timeline.jump-latest-fetched"),
            ("room", "room-timeline.jump-latest-requested"),
            ("room", "room-timeline.jump-latest-result-ignored"),
            ("room", "room-timeline.jump-latest-settled"),
            ("room", "room-timeline.jump-latest-suppressed"),
            ("room", "room-timeline.live-refresh-deferred"),
            ("room", "room-timeline.live-reset"),
            ("room", "room-timeline.live-tail-refresh"),
            ("room", "room-timeline.live-tail-refresh-deferred"),
            ("room", "room-timeline.mark-read-failed"),
            ("room", "room-timeline.open"),
            ("room", "room-timeline.pagination-complete"),
            ("room", "room-timeline.pagination-error"),
            ("room", "room-timeline.pagination-start"),
            ("room", "room-timeline.pagination-suppressed"),
            ("room", "room-timeline.render-window"),
            ("room", "room-timeline.scroll-gesture-ended"),
            ("room", "room-timeline.scroll-gesture-started"),
            ("room", "room-timeline.slow-event-render"),
            ("room", "room-timeline.structural-update-flushed"),
            ("room", "room-timeline.structural-update-queued"),
            ("room", "room-timeline.unexpected-scroll-jump"),
            ("room", "room-timeline.viewport-saved"),
        ];

        for (category, event) in production_events {
            assert!(
                build_diagnostic_record(category.to_owned(), event.to_owned(), json!({}), 1)
                    .is_some(),
                "frontend diagnostic event should pass the native schema: {category}/{event}"
            );
        }
    }

    #[test]
    fn diagnostic_schema_accepts_the_full_frontend_envelope_but_rejects_excess_fields() {
        let mut valid_fields = Map::new();
        for key in NUMBER_FIELDS.iter().take(20) {
            valid_fields.insert((*key).to_owned(), json!(1));
        }
        valid_fields.insert("appRunId".to_owned(), json!("run-lz123abc-1234abcd"));
        valid_fields.insert("roomToken".to_owned(), json!("room-1"));
        valid_fields.insert("eventToken".to_owned(), json!("event-1"));
        valid_fields.insert("traceId".to_owned(), json!("trace-1234"));
        valid_fields.insert("source".to_owned(), json!("timeline"));
        assert_eq!(valid_fields.len(), 25);
        assert!(build_diagnostic_record(
            "room".to_owned(),
            "room-timeline.open".to_owned(),
            Value::Object(valid_fields),
            1,
        )
        .is_some());

        let mut excessive_fields = Map::new();
        for key in NUMBER_FIELDS.iter().take(MAX_DIAGNOSTIC_FIELDS + 1) {
            excessive_fields.insert((*key).to_owned(), json!(1));
        }
        assert_eq!(excessive_fields.len(), MAX_DIAGNOSTIC_FIELDS + 1);
        assert!(build_diagnostic_record(
            "room".to_owned(),
            "room-timeline.open".to_owned(),
            Value::Object(excessive_fields),
            1,
        )
        .is_none());
    }

    #[test]
    fn diagnostic_ranges_reject_unknown_or_non_numeric_members() {
        assert!(build_diagnostic_record(
            "room".to_owned(),
            "room-timeline.render-window".to_owned(),
            json!({"range": {"start": 1, "end": 20}}),
            1,
        )
        .is_some());
        assert!(build_diagnostic_record(
            "room".to_owned(),
            "room-timeline.render-window".to_owned(),
            json!({"range": {"start": 1, "secret": 20}}),
            1,
        )
        .is_none());
        assert!(build_diagnostic_record(
            "room".to_owned(),
            "room-timeline.render-window".to_owned(),
            json!({"range": {"start": "message"}}),
            1,
        )
        .is_none());
    }

    #[test]
    fn diagnostic_store_rotates_reads_and_clears_both_files() {
        let root = temp_log_dir("diagnostics");
        let record_bytes = serde_json::to_vec(&valid_record(1))
            .expect("test record should serialize")
            .len() as u64
            + 1;
        let max_bytes = record_bytes * 2 + 8;
        assert!(append_diagnostic_record_to_dir(
            &root,
            &valid_record(1),
            max_bytes
        ));
        assert!(append_diagnostic_record_to_dir(
            &root,
            &valid_record(2),
            max_bytes
        ));
        assert!(append_diagnostic_record_to_dir(
            &root,
            &valid_record(3),
            max_bytes
        ));

        let report = diagnostics_report_from_dir(&root, 0);
        assert!(report.status.has_data);
        assert_eq!(report.status.entry_count, 3);
        assert_eq!(report.entries[0].fields.get("sequence"), Some(&json!(1)));
        assert_eq!(report.entries[2].fields.get("sequence"), Some(&json!(3)));
        assert!(report.status.total_bytes <= max_bytes * 2);
        assert_eq!(report.status.size_bytes, report.status.total_bytes);

        assert!(clear_diagnostics_from_dir(&root));
        let (current, rotated) = diagnostics_paths(&root);
        assert!(!current.exists());
        assert!(!rotated.exists());
        assert!(clear_diagnostics_from_dir(&root));
        std::fs::remove_dir_all(root).expect("temporary log directory should be removed");
    }

    #[test]
    fn corrupt_or_non_schema_records_are_discarded_from_reports() {
        let root = temp_log_dir("corrupt-diagnostics");
        assert!(append_diagnostic_record_to_dir(
            &root,
            &valid_record(1),
            1_024
        ));
        let (current, _) = diagnostics_paths(&root);
        let mut file = OpenOptions::new()
            .append(true)
            .open(&current)
            .expect("diagnostics should open");
        writeln!(file, "not-json").expect("corrupt line should append");
        writeln!(
            file,
            "{}",
            json!({
                "schemaVersion": 1,
                "timestampMs": 2,
                "category": "room",
                "event": "room-timeline.open",
                "fields": {"messageBody": "private"}
            })
        )
        .expect("invalid schema line should append");

        let report = diagnostics_report_from_dir(&root, 0);
        assert_eq!(report.status.entry_count, 1);
        assert_eq!(report.status.discarded_entries, 2);
        std::fs::remove_dir_all(root).expect("temporary log directory should be removed");
    }

    #[test]
    fn periodic_full_scan_detects_corruption_after_a_valid_first_record() {
        let root = temp_log_dir("full-scan-diagnostics");
        assert!(append_diagnostic_record_to_dir(
            &root,
            &valid_record(1),
            4_096
        ));
        let (current, _) = diagnostics_paths(&root);
        let mut file = OpenOptions::new()
            .append(true)
            .open(&current)
            .expect("diagnostics should open");
        writeln!(file, "not-json").expect("corrupt line should append");

        assert!(!diagnostic_file_needs_compaction(&current, 0, false));
        assert!(diagnostic_file_needs_compaction(&current, 0, true));
        std::fs::remove_dir_all(root).expect("temporary log directory should be removed");
    }

    #[test]
    fn clear_attempts_both_files_after_one_removal_fails() {
        let root = temp_log_dir("clear-failure");
        let (current, rotated) = diagnostics_paths(&root);
        std::fs::create_dir(&current).expect("blocking directory should be created");
        std::fs::write(&rotated, b"record").expect("rotated diagnostics should be written");

        assert!(!clear_diagnostics_from_dir(&root));
        assert!(current.is_dir());
        assert!(!rotated.exists());
        std::fs::remove_dir_all(root).expect("temporary log directory should be removed");
    }

    #[test]
    fn diagnostics_expire_after_seven_days() {
        let now = UNIX_EPOCH + Duration::from_secs(20 * 24 * 60 * 60);
        assert!(!should_prune(
            now - Duration::from_secs(7 * 24 * 60 * 60),
            now
        ));
        assert!(should_prune(
            now - Duration::from_secs(7 * 24 * 60 * 60 + 1),
            now
        ));
        assert!(!should_prune(now + Duration::from_secs(1), now));
    }

    #[test]
    fn diagnostics_compact_mixed_age_files_at_the_per_record_cutoff() {
        let root = temp_log_dir("mixed-age-diagnostics");
        let now = UNIX_EPOCH + Duration::from_secs(20 * 24 * 60 * 60);
        let cutoff_ms = diagnostics_cutoff_ms(now);
        let mut expired = valid_record(1);
        expired.timestamp_ms = cutoff_ms - 1;
        let mut retained = valid_record(2);
        retained.timestamp_ms = cutoff_ms + 1;

        assert!(append_diagnostic_record_to_dir(&root, &expired, 4_096));
        assert!(append_diagnostic_record_to_dir(&root, &retained, 4_096));
        prune_expired_diagnostics(&root, now);

        let report = diagnostics_report_from_dir(&root, cutoff_ms);
        assert_eq!(report.status.entry_count, 1);
        assert_eq!(report.entries[0].timestamp_ms, cutoff_ms + 1);
        assert_eq!(report.entries[0].fields.get("sequence"), Some(&json!(2)));
        let (current, _) = diagnostics_paths(&root);
        let persisted =
            std::fs::read_to_string(current).expect("compacted diagnostics should remain readable");
        assert!(!persisted.contains(&format!("\"timestampMs\":{}", cutoff_ms - 1)));
        assert!(persisted.contains(&format!("\"timestampMs\":{}", cutoff_ms + 1)));
        std::fs::remove_dir_all(root).expect("temporary log directory should be removed");
    }

    #[cfg(unix)]
    #[test]
    fn diagnostic_files_are_owner_read_write_only() {
        let root = temp_log_dir("diagnostic-mode");
        assert!(append_diagnostic_record_to_dir(
            &root,
            &valid_record(1),
            1_024
        ));
        let (current, _) = diagnostics_paths(&root);
        let mode = metadata(&current)
            .expect("diagnostics metadata should exist")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
        std::fs::remove_dir_all(root).expect("temporary log directory should be removed");
    }

    #[cfg(unix)]
    #[test]
    fn generic_log_creation_and_reopen_enforce_owner_read_write_only() {
        let root = temp_log_dir("generic-log-mode");
        let path = root.join(LOG_FILE_NAME);
        std::fs::write(&path, b"existing log").expect("generic log should be written");
        std::fs::set_permissions(&path, Permissions::from_mode(0o644))
            .expect("test permissions should be widened");

        drop(open_private_append(&path).expect("generic log should reopen privately"));
        let mode = metadata(&path)
            .expect("generic log metadata should exist")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
        std::fs::remove_dir_all(root).expect("temporary log directory should be removed");
    }
}
