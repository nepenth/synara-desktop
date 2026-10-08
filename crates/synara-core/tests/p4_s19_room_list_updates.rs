//! P4-S19: poll privacy-safe room-list wake-ups on SharedCore.
//!
//! After start_sync. This is not Platform::emit, not Core.command, and
//! not P4 acceptance. Room ids never appear.

use synara_core::SharedCore;

fn test_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

#[test]
fn room_list_update_surface_is_poll_only_and_not_a_leftover() {
    let udl = crate::ffi_surface::udl();
    assert!(udl.contains("dictionary RoomListUpdateDto"));
    assert!(udl.contains("interface RoomListUpdateError"));
    assert!(udl.contains("sequence<RoomListUpdateDto> poll_room_list_updates()"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("poll_room_list_updates()"));
    assert!(!crate::ffi_surface::shared_core_declares(
        shared_core,
        "command"
    ));
}

#[test]
fn poll_room_list_updates_without_start_returns_empty() {
    let shared = SharedCore::new();
    let updates = test_runtime()
        .block_on(shared.poll_room_list_updates())
        .expect("empty queue is success");
    assert!(updates.is_empty());
    let text = format!("{updates:?}");
    assert!(!text.contains("password"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("@alice"));
    assert!(!text.contains("!"));
}

#[test]
fn enqueue_then_poll_room_list_updates_is_privacy_safe() {
    let shared = SharedCore::new();
    shared.enqueue_room_list_update_for_test(4);
    let updates = test_runtime()
        .block_on(shared.poll_room_list_updates())
        .expect("queued wake-ups drain");
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].session_generation, 4);
    let text = format!("{updates:?}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("password"));
    assert!(!text.contains("@alice"));
    assert!(!text.contains("https://"));
    assert!(!text.contains("!room"));
    let drained = test_runtime()
        .block_on(shared.poll_room_list_updates())
        .expect("second poll is empty");
    assert!(drained.is_empty());
}
