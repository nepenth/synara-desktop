//! P4-S17: poll privacy-safe owner emit summaries on SharedCore.
//!
//! Presence, devices, join_rules, and image_packs. This is not
//! Platform::emit, not Core.command, and not P4 acceptance. Presence user
//! ids never appear.

use synara_core::SharedCore;

fn test_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

#[test]
fn owner_update_surface_is_poll_only_and_not_a_leftover() {
    let udl = crate::ffi_surface::udl();
    assert!(udl.contains("dictionary OwnerUpdateDto"));
    assert!(udl.contains("interface OwnerUpdateError"));
    assert!(udl.contains("sequence<OwnerUpdateDto> poll_owner_updates()"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!udl.contains("matrix_send_attachment"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("poll_owner_updates()"));
    assert!(!crate::ffi_surface::shared_core_declares(
        shared_core,
        "command"
    ));
}

#[test]
fn poll_owner_updates_without_attach_returns_empty() {
    let shared = SharedCore::new();
    let updates = test_runtime()
        .block_on(shared.poll_owner_updates())
        .expect("empty queue is success");
    assert!(updates.is_empty());
    let text = format!("{updates:?}");
    assert!(!text.contains("password"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("@alice"));
}

#[test]
fn enqueue_then_poll_owner_updates_is_privacy_safe() {
    let shared = SharedCore::new();
    shared.enqueue_owner_update_for_test("presence".to_owned(), 3, None);
    shared.enqueue_owner_update_for_test("verification".to_owned(), 3, None);
    shared.enqueue_owner_update_for_test(
        "typing".to_owned(),
        3,
        Some("!s21Room:example.org".to_owned()),
    );
    shared.enqueue_owner_update_for_test(
        "join_rules".to_owned(),
        3,
        Some("!s17Room:example.org".to_owned()),
    );
    let updates = test_runtime()
        .block_on(shared.poll_owner_updates())
        .expect("queued summaries drain");
    assert_eq!(updates.len(), 4);
    assert_eq!(updates[0].family, "presence");
    assert_eq!(updates[0].session_generation, 3);
    assert_eq!(updates[0].room_id, None);
    assert_eq!(updates[1].family, "verification");
    assert_eq!(updates[1].room_id, None);
    assert_eq!(updates[2].family, "typing");
    assert_eq!(updates[2].room_id.as_deref(), Some("!s21Room:example.org"));
    assert_eq!(updates[3].family, "join_rules");
    assert_eq!(updates[3].room_id.as_deref(), Some("!s17Room:example.org"));
    let text = format!("{updates:?}");
    assert!(!text.contains("syt_"));
    assert!(!text.contains("password"));
    assert!(!text.contains("@alice"));
    assert!(!text.contains("https://"));
    let drained = test_runtime()
        .block_on(shared.poll_owner_updates())
        .expect("second poll is empty");
    assert!(drained.is_empty());
}
