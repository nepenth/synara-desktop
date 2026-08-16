//! Unit tests for P4.8 route resolution.

use super::*;
use crate::transport::MatrixIpcErrorCategory;

#[test]
fn marker_stable() {
    assert_eq!(matrix_routes_markers(), MATRIX_ROUTES_MARKER);
}

#[test]
fn home_and_settings() {
    assert_eq!(resolve_path("/home").unwrap(), RouteTarget::Home);
    assert_eq!(resolve_path("/").unwrap(), RouteTarget::Home);
    assert_eq!(
        resolve_path("/settings").unwrap(),
        RouteTarget::Settings { section: None }
    );
    assert_eq!(
        resolve_path("/settings/security").unwrap(),
        RouteTarget::Settings {
            section: Some("security".into())
        }
    );
    assert_eq!(build_path(&RouteTarget::Home).unwrap(), "/home");
}

#[test]
fn room_event_thread() {
    let t = resolve_path("/home/room/!r:example.org").unwrap();
    assert_eq!(
        t,
        RouteTarget::Room {
            room_id: "!r:example.org".into(),
            event_id: None,
            thread_root_id: None,
        }
    );
    let t = resolve_path("/home/room/!r:example.org/event/$e1").unwrap();
    assert_eq!(
        t,
        RouteTarget::Room {
            room_id: "!r:example.org".into(),
            event_id: Some("$e1".into()),
            thread_root_id: None,
        }
    );
    let t = resolve_path("/home/room/!r:example.org/thread/$root").unwrap();
    assert_eq!(
        t,
        RouteTarget::Room {
            room_id: "!r:example.org".into(),
            event_id: None,
            thread_root_id: Some("$root".into()),
        }
    );
    assert_eq!(
        build_path(&t).unwrap(),
        "/home/room/!r:example.org/thread/$root"
    );
}

#[test]
fn space_and_user() {
    assert_eq!(
        resolve_path("/home/space/!s:example.org").unwrap(),
        RouteTarget::Space {
            space_id: "!s:example.org".into()
        }
    );
    assert_eq!(
        resolve_path("/home/user/@alice:example.org").unwrap(),
        RouteTarget::User {
            user_id: "@alice:example.org".into()
        }
    );
}

#[test]
fn invalid_ids() {
    let err = resolve_path("/home/room/bad").unwrap_err();
    assert_eq!(err.diagnostic_id(), "p4.8-invalid-room-id");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);
    assert!(resolve_path("https://example.com/home").is_err());
}

#[test]
fn unknown_and_roundtrip() {
    let t = resolve_path("/home/later/inbox").unwrap();
    assert_eq!(
        t,
        RouteTarget::Unknown {
            path: "/home/later/inbox".into()
        }
    );
    let room = RouteTarget::Room {
        room_id: "!r:example.org".into(),
        event_id: Some("$e".into()),
        thread_root_id: None,
    };
    let path = build_path(&room).unwrap();
    assert_eq!(resolve_path(&path).unwrap(), room);
}

#[test]
fn strip_query_fragment() {
    let t = resolve_path("/home/room/!r:example.org?x=1#y").unwrap();
    assert_eq!(
        t,
        RouteTarget::Room {
            room_id: "!r:example.org".into(),
            event_id: None,
            thread_root_id: None,
        }
    );
}
