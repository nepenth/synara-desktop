use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, Runtime};

const LOG_FILE_NAME: &str = "synara-desktop.log";
const MAX_LOG_FIELD_LEN: usize = 4_000;

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
