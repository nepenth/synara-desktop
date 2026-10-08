//! P4-S14: poll privacy-safe timeline view-delta summaries on SharedCore.
//!
//! This is not Platform::emit, not Core.command, and not P4 acceptance.
//! Empty queue is success, not an error.

use synara_core::app::timeline::TIMELINE_VIEW_SCHEMA_VERSION;
use synara_core::SharedCore;

fn test_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

#[test]
fn timeline_view_update_surface_is_poll_only_and_not_a_leftover() {
    let udl = crate::ffi_surface::udl();
    assert!(udl.contains("dictionary TimelineViewUpdateDto"));
    assert!(udl.contains("interface TimelineViewUpdateError"));
    assert!(udl.contains("sequence<TimelineViewUpdateDto> poll_timeline_view_updates()"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("poll_timeline_view_updates()"));
    assert!(!crate::ffi_surface::shared_core_declares(
        shared_core,
        "command"
    ));
    assert!(!shared_core.contains("matrix_login_password"));
    assert!(!shared_core.contains("Platform::emit"));
}

#[test]
fn poll_timeline_view_updates_without_attach_returns_empty() {
    let shared = SharedCore::new();
    let updates = test_runtime()
        .block_on(shared.poll_timeline_view_updates())
        .expect("empty queue is success");
    assert!(updates.is_empty());
    let text = format!("{updates:?}");
    assert!(!text.contains("password"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("@alice"));
    assert!(!text.contains("https://"));
}

#[test]
fn enqueue_then_poll_returns_privacy_safe_summaries() {
    let access = "syt_s14_timeline_view_access";
    let refresh = "syr_s14_timeline_view_refresh";
    let user_id = "@alice:example.org";
    let homeserver = "https://matrix.example.org";
    let shared = SharedCore::new();
    shared.enqueue_timeline_view_update_for_test(
        "stream-s14".to_owned(),
        "!s14Room:example.org".to_owned(),
        7,
    );
    let updates = test_runtime()
        .block_on(shared.poll_timeline_view_updates())
        .expect("queued summary drains");
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].schema_version, TIMELINE_VIEW_SCHEMA_VERSION);
    assert_eq!(updates[0].session_generation, 1);
    assert_eq!(updates[0].stream_id, "stream-s14");
    assert_eq!(updates[0].room_id, "!s14Room:example.org");
    assert_eq!(updates[0].revision, 7);
    assert_eq!(updates[0].op_count, 0);
    let text = format!("{updates:?}");
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("password"));
    assert!(!text.contains(user_id));
    assert!(!text.contains(homeserver));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
    let drained = test_runtime()
        .block_on(shared.poll_timeline_view_updates())
        .expect("second poll is empty");
    assert!(drained.is_empty());
}
