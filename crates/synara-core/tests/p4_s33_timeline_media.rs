//! P4-S33: native timeline media-handle channel on SharedCore.
//!
//! Bytes are a dedicated UniFFI argument. Not Core.command. Not leftover
//! media_download. NSE cannot download.

use synara_core::SharedCore;

fn test_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

#[test]
fn timeline_media_surface_is_handle_channel_not_leftover_envelope() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("timeline_media_bytes"));
    assert!(udl.contains("string? media_handle_id"));
    assert!(udl.contains("sequence<TimelineViewReactionDto> reactions"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(!udl.contains("matrix_login_password"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("timeline_media_bytes"));
    assert!(!shared_core.contains("command("));
}

#[test]
fn timeline_media_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let handle = "timeline-media-s33-secret";
    let error = test_runtime()
        .block_on(shared.timeline_media_bytes(handle.to_owned()))
        .expect_err("media requires an attached timeline owner");
    let text = format!("{error:?}{error}");
    assert!(text.contains("p4-s33-media-no-session"));
    assert!(!text.contains("password"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("mxc://"));
    assert!(!text.contains(handle));
}
