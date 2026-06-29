use std::env;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const DESKTOP_FILE_IPC_INLINE_THRESHOLD: usize = 8 * 1024 * 1024;
pub(crate) const DESKTOP_FILE_IPC_CHUNK_SIZE: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DroppedFileReadMode {
    Inline,
    Streamed,
}

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

pub(crate) fn downloads_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    let home = env::var_os("USERPROFILE");

    #[cfg(not(target_os = "windows"))]
    let home = env::var_os("HOME");

    let Some(home_dir) = home else {
        return Err("Unable to resolve home directory".to_owned());
    };

    Ok(PathBuf::from(home_dir).join("Downloads"))
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

fn timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

pub(crate) fn should_stream_file_ipc(byte_count: u64) -> bool {
    byte_count > DESKTOP_FILE_IPC_INLINE_THRESHOLD as u64
}

pub(crate) fn dropped_file_read_mode(byte_count: u64) -> DroppedFileReadMode {
    if should_stream_file_ipc(byte_count) {
        DroppedFileReadMode::Streamed
    } else {
        DroppedFileReadMode::Inline
    }
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

#[cfg(test)]
mod tests {
    use super::{
        dropped_file_read_mode, new_file_transfer_id, sanitize_download_filename,
        should_stream_file_ipc, unique_download_path, DroppedFileReadMode,
        DESKTOP_FILE_IPC_INLINE_THRESHOLD,
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
    fn transfer_ids_include_prefix_and_random_hex_body() {
        let id = new_file_transfer_id("save");
        assert!(id.starts_with("save-"));
        assert_eq!(id.len(), "save-".len() + 32);
        assert!(id["save-".len()..]
            .chars()
            .all(|character| character.is_ascii_hexdigit()));
    }
}
