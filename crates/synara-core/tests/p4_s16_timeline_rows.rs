//! P4-S16: timeline snapshot rows on SharedCore.
//!
//! Open/paginate already returned a Core snapshot. This slice keeps the
//! privacy-safe row bodies on the UniFFI DTO so iOS product timeline can
//! render. No media bytes. Not P4 acceptance.

use synara_core::{SharedCore, TimelineOpenPositionDto};

fn test_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn live_bottom() -> TimelineOpenPositionDto {
    TimelineOpenPositionDto {
        kind: "live".to_owned(),
        at_bottom: true,
        restored_anchor_event_id: None,
        live_tail_event_id: None,
        updated_at_ms: None,
        event_id: None,
    }
}

#[test]
fn timeline_snapshot_surface_includes_rows_and_not_leftovers() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("dictionary TimelineViewRowDto"));
    assert!(udl.contains("sequence<TimelineViewRowDto> rows"));
    assert!(udl.contains("sequence<TimelineViewReactionDto> reactions"));
    assert!(udl.contains("string? media_handle_id"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(!udl.contains("matrix_login_password"));
    let snapshot = udl
        .split("dictionary TimelineSnapshotDto {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("TimelineSnapshotDto");
    assert!(snapshot.contains("sequence<TimelineViewRowDto> rows"));
}

#[test]
fn timeline_open_without_session_fails_closed_for_live_alias() {
    let shared = SharedCore::new();
    let error = test_runtime()
        .block_on(shared.timeline_open("!missing:example.org".to_owned(), live_bottom()))
        .expect_err("open requires a session");
    let text = format!("{error:?}{error}");
    assert!(text.contains("p2-timeline-open-no-session"));
    assert!(!text.contains("password"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("@alice"));
    assert!(!text.contains("https://"));
}
