//! P4-S30: room-list DTO keeps joined-room encryption and notification mode.

#[test]
fn room_list_surface_includes_encryption_and_not_leftovers() {
    let udl = crate::ffi_surface::udl();
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

fn dictionary<'a>(udl: &'a str, name: &str) -> &'a str {
    udl.split(&format!("dictionary {name} {{"))
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .unwrap_or_else(|| panic!("{name}"))
}

#[test]
fn room_list_surface_carries_room_folders() {
    let udl = crate::ffi_surface::udl();
    assert!(dictionary(udl, "RoomListRoomDto").contains("string? folder_id"));
    assert!(
        dictionary(udl, "RoomListSnapshotDto").contains("sequence<RoomFolderSectionDto> folders")
    );
    let section = dictionary(udl, "RoomFolderSectionDto");
    for field in [
        "sequence<string> room_ids",
        "sequence<string> recent_order",
        "sequence<string> name_order",
        "u32 unread_highlight",
        "u32 unread_total",
    ] {
        assert!(section.contains(field), "{field}");
    }
    assert!(udl.contains("room_section_placement"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    for method in [
        "room_folders_snapshot",
        "room_folders_create",
        "room_folders_rename",
        "room_folders_delete",
        "room_folders_move",
        "room_folders_set_room",
    ] {
        assert!(shared_core.contains(method), "{method}");
    }
}

#[test]
fn room_folders_without_a_session_fail_closed() {
    let shared = synara_core::SharedCore::new();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let code = |error: synara_core::RoomFoldersCommandError| match error {
        synara_core::RoomFoldersCommandError::Failed { code, .. } => code,
    };
    assert_eq!(
        code(rt.block_on(shared.room_folders_snapshot()).unwrap_err()),
        "p2-room-folders-snapshot-no-session"
    );
    assert_eq!(
        code(
            rt.block_on(shared.room_folders_create("Work".to_owned(), None))
                .unwrap_err()
        ),
        "p2-room-folders-create-no-session"
    );
    assert_eq!(
        code(
            rt.block_on(shared.room_folders_set_room("!r:example.org".to_owned(), None))
                .unwrap_err()
        ),
        "p2-room-folders-set-room-no-session"
    );
}
