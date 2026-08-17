//! P4-S36: desktop leftover download may resolve timeline handles.
//! Bytes still must not cross Core.command.

#[test]
fn media_download_stays_off_core_command() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(!udl.contains("matrix_media_download"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(udl.contains("timeline_media_bytes"));
}

#[test]
fn desktop_download_owner_resolves_timeline_handles() {
    let download = include_str!("../../../src-tauri/src/matrix/media/product_commands.rs");
    assert!(download.contains("is_timeline_media_handle"));
    assert!(download.contains("resolve_timeline_media"));
    assert!(download.contains("MediaSource::Plain"));
    assert!(!download.contains("matrix_login_password"));
}
