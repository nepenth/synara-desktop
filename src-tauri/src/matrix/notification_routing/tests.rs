//! Unit tests for P9.4 notification routing.

use super::*;
use crate::matrix::dto::{NotificationCandidate, NotificationKind};
use crate::matrix::ipc::MatrixIpcErrorCategory;

fn candidate(event_id: Option<&str>) -> NotificationCandidate {
    NotificationCandidate {
        candidate_id: "candidate-1".into(),
        room_id: "!room:example.org".into(),
        event_id: event_id.map(str::to_owned),
        kind: NotificationKind::Message,
        title: "filtered title".into(),
        body: "filtered body".into(),
        route: None,
        suppress_if_focused_room: false,
        is_encrypted: false,
    }
}

#[test]
fn marker_stable() {
    assert_eq!(
        matrix_notification_routing_markers(),
        MATRIX_NOTIFICATION_ROUTING_MARKER
    );
}

#[test]
fn resolves_room_destination() {
    let mut router = NotificationRouter::new(7);
    let route = router
        .resolve(
            "candidate-room".into(),
            "!room:example.org".into(),
            None,
            None,
        )
        .unwrap();

    assert_eq!(route.kind, NotificationRouteKind::Room);
    assert_eq!(route.room_id, "!room:example.org");
    assert_eq!(route.event_id, None);
    assert_eq!(route.thread_root_id, None);
}

#[test]
fn resolves_event_destination() {
    let mut router = NotificationRouter::new(7);
    let route = router
        .resolve_candidate(&candidate(Some("$event")), None)
        .unwrap();

    assert_eq!(route.kind, NotificationRouteKind::Event);
    assert_eq!(route.event_id.as_deref(), Some("$event"));
    assert_eq!(router.last_route("candidate-1"), Some(&route));
}

#[test]
fn thread_root_selects_thread_and_preserves_event_anchor() {
    let mut router = NotificationRouter::new(7);
    let route = router
        .resolve_candidate(&candidate(Some("$reply")), Some("$root".into()))
        .unwrap();

    assert_eq!(route.kind, NotificationRouteKind::Thread);
    assert_eq!(route.event_id.as_deref(), Some("$reply"));
    assert_eq!(route.thread_root_id.as_deref(), Some("$root"));
}

#[test]
fn thread_destination_does_not_require_reply_event() {
    let mut router = NotificationRouter::new(7);
    let route = router
        .resolve_candidate(&candidate(None), Some("$root".into()))
        .unwrap();

    assert_eq!(route.kind, NotificationRouteKind::Thread);
    assert_eq!(route.event_id, None);
}

#[test]
fn repeated_candidate_key_replaces_last_route() {
    let mut router = NotificationRouter::new(7);
    router
        .resolve("candidate-1".into(), "!room:example.org".into(), None, None)
        .unwrap();
    let latest = router
        .resolve(
            "candidate-1".into(),
            "!other:example.org".into(),
            Some("$event".into()),
            None,
        )
        .unwrap();

    assert_eq!(router.len(), 1);
    assert_eq!(router.last_route("candidate-1"), Some(&latest));
}

#[test]
fn invalid_candidate_key_is_rejected_without_registry_mutation() {
    let mut router = NotificationRouter::new(7);
    let error = router
        .resolve(String::new(), "!room:example.org".into(), None, None)
        .unwrap_err();

    assert_eq!(error.diagnostic_id(), "p9.4-empty-candidate-key");
    assert_eq!(error.category(), MatrixIpcErrorCategory::SdkInvariant);
    assert!(router.is_empty());
}

#[test]
fn invalid_room_id_is_rejected_without_echoing_input() {
    let mut router = NotificationRouter::new(7);
    for invalid_room_id in ["not-a-room", "!:example.org", "!room:", "!room:bad server"] {
        let error = router
            .resolve("candidate-1".into(), invalid_room_id.into(), None, None)
            .unwrap_err();

        assert_eq!(error.diagnostic_id(), "p9.4-invalid-room-id");
        assert!(!error.to_string().contains(invalid_room_id));
    }
    assert!(router.is_empty());
}

#[test]
fn invalid_event_and_thread_ids_are_distinguished() {
    let mut router = NotificationRouter::new(7);
    let event_error = router
        .resolve(
            "candidate-event".into(),
            "!room:example.org".into(),
            Some("event".into()),
            None,
        )
        .unwrap_err();
    let thread_error = router
        .resolve(
            "candidate-thread".into(),
            "!room:example.org".into(),
            None,
            Some("root".into()),
        )
        .unwrap_err();

    assert_eq!(event_error.diagnostic_id(), "p9.4-invalid-event-id");
    assert_eq!(thread_error.diagnostic_id(), "p9.4-invalid-thread-root-id");
    assert!(router.is_empty());
}

#[test]
fn retire_generation_clears_routes() {
    let mut router = NotificationRouter::new(7);
    router
        .resolve_candidate(&candidate(Some("$event")), None)
        .unwrap();

    router.retire_generation(8);

    assert_eq!(router.session_generation(), 8);
    assert!(router.is_empty());
    assert_eq!(router.last_route("candidate-1"), None);
}
