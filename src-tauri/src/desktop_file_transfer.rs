use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub(crate) const DESKTOP_FILE_IPC_INLINE_THRESHOLD: usize = 8 * 1024 * 1024;
pub(crate) const DESKTOP_FILE_IPC_CHUNK_SIZE: usize = 1024 * 1024;
pub(crate) const MAX_SAVE_FILE_BYTES: u64 = 300 * 1024 * 1024;
const MAX_ACTIVE_SAVE_TOTAL_BYTES: u64 = 512 * 1024 * 1024;

pub(crate) fn sanitize_download_filename(filename: &str) -> String {
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

/// The user's Downloads folder: `XDG_DOWNLOAD_DIR` from `user-dirs.dirs` on
/// Linux (localized desktops name it differently), the known folder on
/// Windows, `~/Downloads` on macOS. Falls back to `~/Downloads`.
pub(crate) fn downloads_dir() -> Result<PathBuf, String> {
    if let Some(dir) = dirs::download_dir() {
        return Ok(dir);
    }

    #[cfg(target_os = "windows")]
    let home = env::var_os("USERPROFILE");

    #[cfg(not(target_os = "windows"))]
    let home = env::var_os("HOME");

    let Some(home_dir) = home else {
        return Err("Unable to resolve home directory".to_owned());
    };

    Ok(PathBuf::from(home_dir).join("Downloads"))
}

/// Write `bytes` into the user's Downloads folder and return only the filename.
///
/// The bytes go to a hidden temp file in Downloads first and only then take
/// their final name, so a quit mid-write never leaves a truncated file under
/// the name the user expects. The final name is claimed without overwriting:
/// when another save takes it first, the next free `name (n).ext` is used.
pub(crate) fn save_exclusive_download(filename: &str, bytes: &[u8]) -> Result<String, String> {
    if bytes.is_empty() {
        return Err("File is empty".to_owned());
    }
    if bytes.len() as u64 > MAX_SAVE_FILE_BYTES {
        return Err("File exceeds the maximum save size".to_owned());
    }
    let downloads = downloads_dir()?;
    fs::create_dir_all(&downloads).map_err(|err| format!("Unable to create Downloads: {err}"))?;
    save_exclusive_download_in(&downloads, filename, bytes)
}

/// Attempts at a free name; each one loses only to a save that claimed the
/// same name between our existence check and our link.
const MAX_DOWNLOAD_NAME_ATTEMPTS: usize = 16;

fn save_exclusive_download_in(
    downloads: &Path,
    filename: &str,
    bytes: &[u8],
) -> Result<String, String> {
    let filename = sanitize_download_filename(filename);
    let partial = PartialDownload::write(downloads, bytes)?;
    for _ in 0..MAX_DOWNLOAD_NAME_ATTEMPTS {
        let path = unique_download_path(downloads, &filename);
        ensure_inside_downloads(downloads, &path)?;
        match partial.claim(&path) {
            Ok(()) => {
                return Ok(path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or(&filename)
                    .to_owned());
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(format!("Unable to write file: {err}")),
        }
    }
    Err("Unable to find a free download filename".to_owned())
}

/// A complete download in a hidden, private (0600) temp file next to its
/// destination. Dropping it removes the temp file, on success and on error.
struct PartialDownload {
    path: PathBuf,
}

impl PartialDownload {
    fn write(downloads: &Path, bytes: &[u8]) -> Result<Self, String> {
        let path = downloads.join(format!(".{}.part", new_file_transfer_id("synara-download")));
        ensure_inside_downloads(downloads, &path)?;
        let mut file = create_private_temp_file(&path)
            .map_err(|err| format!("Unable to write file: {err}"))?;
        // From here on, `Drop` removes whatever was written.
        let partial = Self { path };
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|err| format!("Unable to write file: {err}"))?;
        Ok(partial)
    }

    /// Give the complete file `destination`'s name, failing with
    /// `AlreadyExists` instead of replacing a file that already has it.
    fn claim(&self, destination: &Path) -> std::io::Result<()> {
        // A hard link is an atomic create-if-absent. File systems without hard
        // links (FAT, some network shares) reserve the name with an empty
        // exclusive file and rename the temp file over it.
        match fs::hard_link(&self.path, destination) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Err(err),
            Err(_) => {
                drop(create_private_temp_file(destination)?);
                fs::rename(&self.path, destination).inspect_err(|_| {
                    let _ = fs::remove_file(destination);
                })
            }
        }
    }
}

impl Drop for PartialDownload {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// `path` must sit directly in `downloads`, after resolving symlinks.
fn ensure_inside_downloads(downloads: &Path, path: &Path) -> Result<(), String> {
    let downloads = downloads
        .canonicalize()
        .map_err(|err| format!("Unable to resolve Downloads: {err}"))?;
    let parent = path
        .parent()
        .ok_or_else(|| "Unable to resolve download path".to_owned())?
        .canonicalize()
        .map_err(|err| format!("Unable to resolve Downloads: {err}"))?;
    if parent != downloads {
        return Err("Refusing to save outside Downloads".to_owned());
    }
    Ok(())
}

/// The path of a file Synara saved as `filename` (the name a save returned)
/// in `downloads`. The name is sanitized again and must come back unchanged,
/// so a path, a traversal or any name the save could not have produced is
/// refused rather than reinterpreted. The file must exist.
pub(crate) fn saved_download_path(downloads: &Path, filename: &str) -> Result<PathBuf, String> {
    if filename.is_empty() || sanitize_download_filename(filename) != filename {
        return Err("Not a saved download filename".to_owned());
    }
    let path = downloads.join(filename);
    ensure_inside_downloads(downloads, &path)?;
    let metadata =
        fs::symlink_metadata(&path).map_err(|_| "The file is not in Downloads".to_owned())?;
    if !metadata.is_file() {
        return Err("The file is not in Downloads".to_owned());
    }
    Ok(path)
}

pub(crate) fn unique_download_path(downloads: &Path, filename: &str) -> PathBuf {
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

    downloads.join(format!("{stem} ({})", timestamp_ms()))
}

fn write_exclusive_download(downloads: &Path, path: &Path, bytes: &[u8]) -> Result<(), String> {
    let downloads = downloads
        .canonicalize()
        .map_err(|err| format!("Unable to resolve Downloads: {err}"))?;
    let parent = path
        .parent()
        .ok_or_else(|| "Unable to resolve download path".to_owned())?;
    let parent = if parent.exists() {
        parent
            .canonicalize()
            .map_err(|err| format!("Unable to resolve Downloads: {err}"))?
    } else {
        parent.to_path_buf()
    };
    if parent != downloads {
        return Err("Refusing to save outside Downloads".to_owned());
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|err| format!("Unable to write file: {err}"))?;
    file.write_all(bytes)
        .map_err(|err| format!("Unable to write file: {err}"))?;
    Ok(())
}

fn timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

pub(crate) fn should_stream_file_ipc(byte_count: u64) -> bool {
    byte_count > DESKTOP_FILE_IPC_INLINE_THRESHOLD as u64
}

pub(crate) fn new_file_transfer_id(prefix: &str) -> String {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("cryptographic random bytes for file transfer id");
    format!(
        "{prefix}-{}",
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

const MAX_DROPPED_FILES: usize = 32;
const MAX_DROPPED_FILE_ALLOWLIST: usize = 256;
const MAX_DROPPED_FILE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_DROPPED_TOTAL_BYTES: u64 = 512 * 1024 * 1024;
/// Largest dropped-file chunk per read. Chunks cross IPC as raw bytes
/// (`ipc::Response`), not JSON number arrays.
pub(crate) const DROPPED_FILE_READ_CHUNK_SIZE: usize = 8 * 1024 * 1024;
/// Every file of one drop is read through its own session.
const MAX_DROPPED_READ_SESSIONS: usize = MAX_DROPPED_FILES * 2;

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

/// One dropped file: a read session for its bytes, or `too_large` when it is
/// over the caller's limit (nothing is opened or read then).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDroppedFilePayload {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer_id: Option<String>,
    pub size: u64,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub too_large: bool,
}

#[derive(Clone, Debug, Serialize)]
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
    let active_bytes = sessions
        .values()
        .try_fold(0_u64, |total, active| {
            total.checked_add(active.expected_size)
        })
        .ok_or_else(|| "Active file save transfers exceed the size limit".to_owned())?;
    if active_bytes
        .checked_add(session.expected_size)
        .is_none_or(|total| total > MAX_ACTIVE_SAVE_TOTAL_BYTES)
    {
        return Err("Active file save transfers exceed the size limit".to_owned());
    }
    sessions.insert(session_id.clone(), session);
    Ok(session_id)
}

fn create_private_temp_file(path: &Path) -> Result<File, std::io::Error> {
    let mut options = OpenOptions::new();
    options.create_new(true).read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
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
    if sessions.len() >= MAX_DROPPED_READ_SESSIONS {
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
    write_exclusive_download(&downloads, &destination, &[])?;
    fs::rename(&session.temp_path, &destination)
        .map_err(|err| format!("Unable to finalize saved file: {err}"))?;

    Ok(destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&filename)
        .to_owned())
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
    write_exclusive_download(&downloads, &path, &payload.bytes)?;

    Ok(path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&filename)
        .to_owned())
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
    if total_size > MAX_SAVE_FILE_BYTES {
        return Err("File exceeds the maximum save size".to_owned());
    }

    let safe_filename = sanitize_download_filename(&filename);
    let temp_path = std::env::temp_dir().join(new_file_transfer_id("save-temp"));
    create_private_temp_file(&temp_path)
        .map_err(|err| format!("Unable to create temp save file: {err}"))?;

    let session = SaveFileSession {
        temp_path: temp_path.clone(),
        filename: safe_filename,
        expected_size: total_size,
        bytes_received: 0,
        created_at: Instant::now(),
    };
    let session_id = register_save_session(session).inspect_err(|_| {
        let _ = fs::remove_file(temp_path);
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

/// What "Show in folder" managed to open.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DesktopRevealDownloadResult {
    /// The file manager opened Downloads with the file selected.
    Item,
    /// Revealing failed, so Downloads itself was opened.
    Folder,
}

/// Show a file Synara saved in the system file manager. Only the filename the
/// save returned crosses IPC; the path is rebuilt from Downloads here.
///
/// On Linux the reveal uses the `org.freedesktop.FileManager1` D-Bus service
/// (then the OpenURI portal), which tiling desktops often lack, so a failed
/// reveal opens the Downloads folder instead.
#[tauri::command]
pub async fn desktop_reveal_download(
    app: tauri::AppHandle,
    filename: String,
) -> Result<DesktopRevealDownloadResult, String> {
    use tauri_plugin_opener::OpenerExt;

    // D-Bus and AppKit calls block; keep them off the async runtime.
    tauri::async_runtime::spawn_blocking(move || {
        let downloads = downloads_dir()?;
        let path = saved_download_path(&downloads, &filename)?;
        let opener = app.opener();
        if opener.reveal_item_in_dir(&path).is_ok() {
            return Ok(DesktopRevealDownloadResult::Item);
        }
        opener
            .open_path(downloads.to_string_lossy(), None::<&str>)
            .map(|()| DesktopRevealDownloadResult::Folder)
            .map_err(|err| format!("Unable to open Downloads: {err}"))
    })
    .await
    .map_err(|err| format!("Unable to open Downloads: {err}"))?
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

/// Open read sessions for the files of one drop. `max_file_bytes` is the
/// composer's upload limit: a file over it is reported as `too_large` from
/// its metadata, before anything is read into the webview.
#[tauri::command]
pub fn desktop_read_dropped_files(
    paths: Vec<String>,
    max_file_bytes: Option<u64>,
) -> Result<Vec<DesktopDroppedFilePayload>, String> {
    let _clear_allowlist_guard = ClearDroppedFileAllowlistGuard;

    if paths.len() > MAX_DROPPED_FILES {
        return Err(format!(
            "Too many dropped files. Maximum is {MAX_DROPPED_FILES}."
        ));
    }

    let file_limit = max_file_bytes
        .filter(|limit| *limit > 0)
        .map_or(MAX_DROPPED_FILE_BYTES, |limit| {
            limit.min(MAX_DROPPED_FILE_BYTES)
        });
    let mut total_bytes = 0_u64;
    let mut files = Vec::with_capacity(paths.len());

    for path in paths {
        let canonical = take_allowed_dropped_path(&path)?;
        let metadata = fs::metadata(&canonical)
            .map_err(|err| format!("Unable to inspect dropped file: {err}"))?;
        if !metadata.is_file() {
            return Err("Only files can be dropped into the composer".to_owned());
        }

        let name = canonical
            .file_name()
            .and_then(|value| value.to_str())
            .map(sanitize_download_filename)
            .unwrap_or_else(|| "attachment".to_owned());
        let file_size = metadata.len();
        if file_size > file_limit {
            files.push(DesktopDroppedFilePayload {
                name,
                transfer_id: None,
                size: file_size,
                too_large: true,
            });
            continue;
        }
        total_bytes = total_bytes.saturating_add(file_size);
        if total_bytes > MAX_DROPPED_TOTAL_BYTES {
            return Err("Dropped files are too large to attach together".to_owned());
        }

        let transfer_id = register_dropped_read_session(DroppedReadSession {
            path: canonical,
            size: file_size,
            created_at: Instant::now(),
        })?;
        files.push(DesktopDroppedFilePayload {
            name,
            transfer_id: Some(transfer_id),
            size: file_size,
            too_large: false,
        });
    }

    Ok(files)
}

/// The bytes at `offset`, at most `length` and [`DROPPED_FILE_READ_CHUNK_SIZE`],
/// as a raw IPC body (an `ArrayBuffer` in the webview). Empty at the end.
#[tauri::command]
pub fn desktop_read_dropped_file_chunk(
    transfer_id: String,
    offset: u64,
    length: usize,
) -> Result<tauri::ipc::Response, String> {
    read_dropped_file_chunk(&transfer_id, offset, length).map(tauri::ipc::Response::new)
}

fn read_dropped_file_chunk(
    transfer_id: &str,
    offset: u64,
    length: usize,
) -> Result<Vec<u8>, String> {
    if length == 0 {
        return Err("Dropped file chunk length must be positive".to_owned());
    }
    let chunk_length = length.min(DROPPED_FILE_READ_CHUNK_SIZE);

    purge_stale_file_transfer_sessions(Instant::now());
    let now = Instant::now();
    let mut sessions = dropped_read_sessions()
        .lock()
        .map_err(|_| "Unable to access dropped file read sessions".to_owned())?;
    if let Some(session) = sessions.get(transfer_id) {
        if dropped_read_session_is_stale(session, now) {
            sessions.remove(transfer_id);
            return Err("Dropped file read transfer has expired".to_owned());
        }
    }
    let session = sessions
        .get(transfer_id)
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
    let mut filled = 0;
    while filled < read_length {
        let read = file
            .read(&mut buffer[filled..])
            .map_err(|err| format!("Unable to read dropped file chunk: {err}"))?;
        if read == 0 {
            break;
        }
        filled += read;
    }
    buffer.truncate(filled);

    Ok(buffer)
}

#[tauri::command]
pub fn desktop_read_dropped_file_end(transfer_id: String) -> Result<bool, String> {
    purge_stale_file_transfer_sessions(Instant::now());
    remove_dropped_read_session(&transfer_id);
    Ok(true)
}

#[cfg(test)]
mod download_save_tests {
    use super::*;

    fn temp_downloads(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(new_file_transfer_id(label));
        fs::create_dir_all(&dir).expect("test downloads directory");
        dir
    }

    fn names_in(dir: &Path) -> Vec<String> {
        let mut names = fs::read_dir(dir)
            .expect("list downloads")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .into_string()
                    .expect("utf-8")
            })
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    #[test]
    fn save_writes_the_complete_private_file_and_leaves_no_temp_file() {
        let dir = temp_downloads("save-complete-test");
        let saved = save_exclusive_download_in(&dir, "key.pem", b"-----BEGIN-----").expect("save");
        assert_eq!(saved, "key.pem");
        assert_eq!(
            fs::read(dir.join("key.pem")).expect("read"),
            b"-----BEGIN-----"
        );
        assert_eq!(names_in(&dir), ["key.pem"]);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(dir.join("key.pem"))
                .expect("meta")
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn save_never_replaces_an_existing_file() {
        let dir = temp_downloads("save-existing-test");
        fs::write(dir.join("report.txt"), b"theirs").expect("fixture");
        let saved = save_exclusive_download_in(&dir, "report.txt", b"ours").expect("save");
        assert_eq!(saved, "report (1).txt");
        assert_eq!(fs::read(dir.join("report.txt")).expect("read"), b"theirs");
        assert_eq!(fs::read(dir.join("report (1).txt")).expect("read"), b"ours");
        assert_eq!(names_in(&dir), ["report (1).txt", "report.txt"]);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn concurrent_saves_of_one_name_all_land_under_distinct_names() {
        let dir = temp_downloads("save-race-test");
        let saves = 8;
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(saves));
        let handles = (0..saves)
            .map(|index| {
                let dir = dir.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let bytes = format!("copy {index}").into_bytes();
                    barrier.wait();
                    let name = save_exclusive_download_in(&dir, "report.txt", &bytes)
                        .expect("every racing save succeeds");
                    (name, bytes)
                })
            })
            .collect::<Vec<_>>();
        let results = handles
            .into_iter()
            .map(|handle| handle.join().expect("save thread"))
            .collect::<Vec<_>>();
        let mut names = results
            .iter()
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), saves);
        for (name, bytes) in &results {
            assert_eq!(&fs::read(dir.join(name)).expect("read"), bytes);
        }
        assert_eq!(names_in(&dir), names);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_taken_name_fails_the_claim_and_the_temp_file_is_removed() {
        let dir = temp_downloads("save-claim-test");
        fs::write(dir.join("taken.txt"), b"theirs").expect("fixture");
        let partial = PartialDownload::write(&dir, b"ours").expect("temp write");
        assert_eq!(names_in(&dir).len(), 2);
        let err = partial
            .claim(&dir.join("taken.txt"))
            .expect_err("name is taken");
        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
        drop(partial);
        assert_eq!(names_in(&dir), ["taken.txt"]);
        assert_eq!(fs::read(dir.join("taken.txt")).expect("read"), b"theirs");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_failed_write_leaves_nothing_behind() {
        let dir = temp_downloads("save-missing-dir-test");
        let missing = dir.join("gone");
        assert!(save_exclusive_download_in(&missing, "report.txt", b"bytes").is_err());
        assert!(names_in(&dir).is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn reveal_accepts_only_a_saved_filename_that_exists_in_downloads() {
        let dir = temp_downloads("reveal-test");
        fs::write(dir.join("key.pem"), b"pem").expect("fixture");
        fs::create_dir(dir.join("folder")).expect("fixture dir");
        assert_eq!(
            saved_download_path(&dir, "key.pem").expect("saved file"),
            dir.join("key.pem")
        );
        for refused in [
            "",
            "../key.pem",
            "..",
            ".",
            "sub/key.pem",
            "sub\\key.pem",
            "/etc/passwd",
            "C:\\Windows\\win.ini",
            " key.pem",
            "missing.pem",
            "folder",
        ] {
            assert!(
                saved_download_path(&dir, refused).is_err(),
                "{refused:?} must be refused"
            );
        }
        #[cfg(unix)]
        {
            let outside = temp_downloads("reveal-outside-test");
            fs::write(outside.join("secret"), b"secret").expect("fixture");
            std::os::unix::fs::symlink(outside.join("secret"), dir.join("link")).expect("link");
            assert!(saved_download_path(&dir, "link").is_err());
            let _ = fs::remove_dir_all(outside);
        }
        let _ = fs::remove_dir_all(dir);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        downloads_dir, new_file_transfer_id, sanitize_download_filename, should_stream_file_ipc,
        unique_download_path, DESKTOP_FILE_IPC_INLINE_THRESHOLD,
    };

    #[test]
    fn download_filenames_strip_paths_and_unsafe_characters() {
        assert_eq!(sanitize_download_filename("../secret.txt"), "secret.txt");
        assert_eq!(sanitize_download_filename("bad:name?.txt"), "bad_name_.txt");
        assert_eq!(sanitize_download_filename("..."), "download");
        assert_eq!(sanitize_download_filename("  report.pdf  "), "report.pdf");
    }

    #[test]
    fn unique_download_path_adds_suffix_when_name_exists() {
        let dir = std::env::temp_dir().join(new_file_transfer_id("unique-path-test"));
        std::fs::create_dir_all(&dir).expect("test directory should be created");
        std::fs::write(dir.join("report.txt"), b"one").expect("fixture should be written");

        assert_eq!(
            unique_download_path(&dir, "other.txt"),
            dir.join("other.txt")
        );
        assert_eq!(
            unique_download_path(&dir, "report.txt"),
            dir.join("report (1).txt")
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn streaming_threshold_matches_frontend_contract() {
        assert!(!should_stream_file_ipc(
            DESKTOP_FILE_IPC_INLINE_THRESHOLD as u64
        ));
        assert!(should_stream_file_ipc(
            DESKTOP_FILE_IPC_INLINE_THRESHOLD as u64 + 1
        ));
    }

    #[test]
    fn downloads_follow_the_platform_user_directory() {
        let dir = downloads_dir().expect("downloads dir");
        if let Some(platform) = dirs::download_dir() {
            assert_eq!(dir, platform);
        }
        #[cfg(target_os = "macos")]
        assert!(dir.ends_with("Downloads"));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn linux_downloads_use_the_localized_xdg_download_dir() {
        let source = include_str!("desktop_file_transfer.rs");
        assert!(source.contains("dirs::download_dir()"));
        // `dirs` reads XDG_DOWNLOAD_DIR from user-dirs.dirs (e.g.
        // "$HOME/Téléchargements" on a French desktop).
        if let Some(dir) = dirs::download_dir() {
            assert_eq!(downloads_dir().expect("downloads dir"), dir);
        }
    }

    #[test]
    fn transfer_ids_include_prefix_and_random_hex_body() {
        let id = new_file_transfer_id("save");
        assert!(id.starts_with("save-"));
        assert_eq!(id.len(), "save-".len() + 32);
        assert!(id["save-".len()..]
            .chars()
            .all(|character| character.is_ascii_hexdigit()));
    }
}

#[cfg(test)]
mod command_tests {
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
    fn should_stream_file_ipc_uses_eight_mebibyte_threshold() {
        assert!(!should_stream_file_ipc(
            crate::desktop_file_transfer::DESKTOP_FILE_IPC_INLINE_THRESHOLD as u64
        ));
        assert!(should_stream_file_ipc(
            crate::desktop_file_transfer::DESKTOP_FILE_IPC_INLINE_THRESHOLD as u64 + 1
        ));
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
    fn desktop_save_file_begin_rejects_unbounded_declared_sizes() {
        let result = desktop_save_file_begin(
            "too-large.bin".to_owned(),
            MAX_SAVE_FILE_BYTES.saturating_add(1),
        );
        assert!(result
            .expect_err("oversized streaming save should fail")
            .contains("maximum save size"));
    }

    #[test]
    fn desktop_read_dropped_files_rejects_unauthorized_path() {
        let _guard = DROPPED_FILE_ALLOWLIST_TEST_LOCK
            .lock()
            .expect("drop allowlist test lock should not be poisoned");
        clear_dropped_file_registry_for_tests();
        // A real file that was never dropped, so only the allowlist refuses it.
        let (temp_dir, file_path) = write_temp_drop_fixture("never-dropped.txt", b"secret");
        let result =
            desktop_read_dropped_files(vec![file_path.to_string_lossy().into_owned()], None);
        let _ = fs::remove_dir_all(temp_dir);

        assert!(result.is_err());
        assert!(result
            .expect_err("unauthorized path should fail")
            .contains("not available"));
    }

    #[test]
    fn streaming_save_round_trip_writes_expected_bytes_without_inline_buffer() {
        let _guard = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let original_home = std::env::var("HOME").ok();
        let original_config = std::env::var("XDG_CONFIG_HOME").ok();
        let temp_home = std::env::temp_dir().join(new_file_transfer_id("save-home-test"));
        fs::create_dir_all(&temp_home).expect("temp home should be created");
        std::env::set_var("HOME", &temp_home);
        // No user-dirs.dirs here, so the save falls back to $HOME/Downloads.
        std::env::set_var("XDG_CONFIG_HOME", temp_home.join(".config"));

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

        let saved_name =
            desktop_save_file_end(begin.session_id).expect("streaming save should finalize");
        let saved_path = downloads_dir().expect("downloads dir").join(saved_name);
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
        if let Some(config) = original_config {
            std::env::set_var("XDG_CONFIG_HOME", config);
        } else {
            std::env::remove_var("XDG_CONFIG_HOME");
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

        remember_dropped_paths(std::slice::from_ref(&file_path));
        let descriptors =
            desktop_read_dropped_files(vec![file_path.to_string_lossy().into_owned()], None)
                .expect("dropped file metadata should be returned");
        assert_eq!(descriptors.len(), 1);
        assert!(!descriptors[0].too_large);
        let transfer_id = descriptors[0]
            .transfer_id
            .clone()
            .expect("streamed transfer id should be present");
        assert_eq!(descriptors[0].size, total_size);

        let first_chunk = read_dropped_file_chunk(&transfer_id, 0, DESKTOP_FILE_IPC_CHUNK_SIZE)
            .expect("first chunk should be readable");
        assert_eq!(first_chunk.len(), DESKTOP_FILE_IPC_CHUNK_SIZE);
        assert!(first_chunk.iter().all(|byte| *byte == 9));

        let second_chunk = read_dropped_file_chunk(
            &transfer_id,
            crate::desktop_file_transfer::DESKTOP_FILE_IPC_INLINE_THRESHOLD as u64,
            DESKTOP_FILE_IPC_CHUNK_SIZE,
        )
        .expect("second chunk should be readable");
        assert_eq!(second_chunk.len(), 512);
        assert!(second_chunk.iter().all(|byte| *byte == 9));

        assert!(desktop_read_dropped_file_end(transfer_id).expect("transfer should end"));

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
        remember_dropped_paths(std::slice::from_ref(&file_path));
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

        remember_dropped_paths(std::slice::from_ref(&file_path));
        sleep(Duration::from_millis(10));

        let result =
            desktop_read_dropped_files(vec![file_path.to_string_lossy().into_owned()], None);
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

        remember_dropped_paths(std::slice::from_ref(&file_path));
        let files =
            desktop_read_dropped_files(vec![file_path.to_string_lossy().into_owned()], None)
                .expect("authorized dropped file should be readable");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "consume.txt");
        let transfer_id = files[0].transfer_id.clone().expect("read session");
        assert_eq!(
            read_dropped_file_chunk(&transfer_id, 0, DROPPED_FILE_READ_CHUNK_SIZE)
                .expect("chunk")
                .as_slice(),
            b"payload"
        );
        assert!(desktop_read_dropped_file_end(transfer_id).expect("transfer should end"));
        assert_eq!(dropped_file_allowlist_len_for_tests(), 0);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn dropped_files_over_the_upload_limit_are_rejected_before_any_read() {
        let _guard = DROPPED_FILE_ALLOWLIST_TEST_LOCK
            .lock()
            .expect("drop allowlist test lock should not be poisoned");
        clear_dropped_file_registry_for_tests();
        let temp_dir = std::env::temp_dir().join(new_file_transfer_id("drop-limit-test"));
        fs::create_dir_all(&temp_dir).expect("temp drop dir should be created");
        let small = temp_dir.join("small.txt");
        let large = temp_dir.join("large.bin");
        fs::write(&small, b"ok").expect("small fixture");
        fs::write(&large, vec![1_u8; 64]).expect("large fixture");
        // Unreadable: a read attempt would fail, so a too-large result proves
        // the limit is applied from metadata alone.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&large, fs::Permissions::from_mode(0o000))
                .expect("make fixture unreadable");
        }

        remember_dropped_paths(&[small.clone(), large.clone()]);
        let sessions_before = dropped_read_sessions().lock().expect("sessions").len();
        let files = desktop_read_dropped_files(
            vec![
                small.to_string_lossy().into_owned(),
                large.to_string_lossy().into_owned(),
            ],
            Some(32),
        )
        .expect("drop should be described");
        assert_eq!(files.len(), 2);
        assert!(!files[0].too_large);
        assert!(files[0].transfer_id.is_some());
        assert!(files[1].too_large);
        assert_eq!(files[1].size, 64);
        assert!(files[1].transfer_id.is_none());
        // Only the accepted file got a read session.
        assert_eq!(
            dropped_read_sessions().lock().expect("sessions").len(),
            sessions_before + 1
        );
        let json = serde_json::to_value(&files[1]).expect("payload json");
        assert_eq!(json["tooLarge"], true);
        assert!(serde_json::to_value(&files[0]).expect("payload json")["tooLarge"].is_null());

        if let Some(transfer_id) = files[0].transfer_id.clone() {
            let _ = desktop_read_dropped_file_end(transfer_id);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&large, fs::Permissions::from_mode(0o600));
        }
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn dropped_file_chunks_cross_ipc_as_raw_bytes() {
        let source = include_str!("desktop_file_transfer.rs");
        assert!(source.contains(
            "read_dropped_file_chunk(&transfer_id, offset, length).map(tauri::ipc::Response::new)"
        ));
        assert!(!source.contains(concat!("pub bytes: Option<", "Vec<u8>>")));
    }
}
