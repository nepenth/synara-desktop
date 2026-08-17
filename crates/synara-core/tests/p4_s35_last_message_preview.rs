//! P4-S35: room-list DTO keeps a privacy-safe last-message preview.

#[test]
fn room_list_surface_includes_last_message_preview_and_not_leftovers() {
    let udl = include_str!("../src/synara_core.udl");
    let room = udl
        .split("dictionary RoomListRoomDto {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("RoomListRoomDto");
    assert!(room.contains("string? last_message_preview"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(!udl.contains("matrix_login_password"));
    assert!(!room.contains("mxc://"));
}
