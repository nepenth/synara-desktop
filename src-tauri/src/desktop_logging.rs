use std::fs::{create_dir_all, metadata, remove_file, rename, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, Runtime};

const LOG_FILE_NAME: &str = "synara-desktop.log";
const MAX_LOG_FIELD_LEN: usize = 4_000;
const MAX_LOG_FILE_BYTES: u64 = 5 * 1024 * 1024;
static LOG_WRITE_LOCK: Mutex<()> = Mutex::new(());

fn rotate_log_file_if_needed(log_path: &Path, max_bytes: u64) {
    if metadata(log_path).map_or(true, |entry| entry.len() < max_bytes) {
        return;
    }

    let backup_path = log_path.with_extension("log.1");
    let _ = remove_file(&backup_path);
    if let Err(error) = rename(log_path, backup_path) {
        eprintln!("[synara] failed to rotate app log: {error}");
    }
}

fn sanitize_log_field(value: &str) -> String {
    value
        .replace('\n', " ")
        .replace('\r', " ")
        .replace("access_token", "[redacted]")
        .replace("refresh_token", "[redacted]")
        .chars()
        .take(MAX_LOG_FIELD_LEN)
        .collect()
}

fn timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
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
    rotate_log_file_if_needed(&log_path, MAX_LOG_FILE_BYTES);
    let source = sanitize_log_field(source);
    let message = sanitize_log_field(message);
    let line = format!("[{}] {} {}\n", timestamp_ms(), source, message);

    match OpenOptions::new().create(true).append(true).open(&log_path) {
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

#[cfg(test)]
mod tests {
    use super::rotate_log_file_if_needed;
    use std::fs::{read, remove_dir_all, write};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn rotates_logs_at_the_size_limit() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("synara-log-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&root).expect("temporary log directory should be created");
        let log_path = root.join("synara-desktop.log");
        write(&log_path, b"0123456789").expect("test log should be written");

        rotate_log_file_if_needed(&log_path, 10);

        assert!(!log_path.exists());
        assert_eq!(
            read(root.join("synara-desktop.log.1")).expect("rotated log should exist"),
            b"0123456789"
        );
        remove_dir_all(root).expect("temporary log directory should be removed");
    }
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
        .map_err(|error| error.to_string())
}
