//! P4-S30: room-list DTO keeps joined-room encryption and notification mode.

#[test]
fn room_list_surface_includes_encryption_and_not_leftovers() {
    let udl = include_str!("../src/synara_core.udl");
    let room = udl
        .split("dictionary RoomListRoomDto {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("RoomListRoomDto");
    assert!(room.contains("boolean is_encrypted"));
    assert!(room.contains("RoomEncryptionStatus encryption_status"));
    assert!(udl.contains("enum RoomEncryptionStatus"));
    assert!(room.contains("string? notification_mode"));
    assert!(!udl.contains("matrix_send_attachment"));
    assert!(!udl.contains("matrix_login_password"));
}
