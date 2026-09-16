use matrix_sdk::ruma::events::MessageLikeEventType;
use matrix_sdk::widget::{Capabilities, Filter, MessageLikeEventFilter};

use super::*;

fn message_filter(event_type: MessageLikeEventType) -> Filter {
    Filter::MessageLike(MessageLikeEventFilter::WithType(event_type))
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_widgets_markers(), MATRIX_WIDGETS_MARKER);
}

#[test]
fn public_https_is_safe_for_room_state_and_agent() {
    assert!(is_safe_widget_url("https://widgets.example.org/app", false));
    assert!(is_safe_widget_url("https://widgets.example.org/app", true));
}

#[test]
fn file_javascript_data_and_credentials_are_rejected() {
    assert!(!is_safe_widget_url("file:///etc/passwd", false));
    assert!(!is_safe_widget_url("javascript:alert(1)", true));
    assert!(!is_safe_widget_url("data:text/html,hi", false));
    assert!(!is_safe_widget_url(
        "https://user:pass@widgets.example.org/app",
        true
    ));
}

#[test]
fn token_query_markers_are_rejected() {
    assert!(!is_safe_widget_url(
        "https://widgets.example.org/app?access_token=secret",
        false
    ));
    assert!(!is_safe_widget_url(
        "https://widgets.example.org/app?loginToken=secret",
        true
    ));
    assert!(!is_safe_widget_url(
        "http://127.0.0.1:3000/?accessToken=secret",
        true
    ));
}

#[test]
fn loopback_is_agent_only_and_room_state_cannot_load_it() {
    for url in [
        "http://127.0.0.1:8765/agent",
        "http://localhost:3000/",
        "http://[::1]:8080/",
        "https://127.0.0.1/agent",
        "https://localhost/agent",
    ] {
        assert!(
            is_safe_widget_url(url, true),
            "agent list must allow loopback {url}"
        );
        assert!(
            !is_safe_widget_url(url, false),
            "room-state must reject loopback {url}"
        );
    }
}

#[test]
fn lan_and_private_hosts_are_never_safe() {
    for url in [
        "https://10.0.0.5/widget",
        "https://192.168.1.10/widget",
        "http://10.0.0.5/widget",
        "https://widget.internal/app",
        "https://app.local/widget",
    ] {
        assert!(!is_safe_widget_url(url, false), "{url}");
        assert!(!is_safe_widget_url(url, true), "{url}");
    }
}

#[test]
fn registry_caps_at_thirty_two_and_retire_is_idempotent() {
    let mut registry = WidgetRegistry::new();
    for index in 0..MAX_WIDGET_SESSIONS {
        registry
            .insert(WidgetSessionRecord {
                session_id: format!("w{index}"),
                widget_id: format!("id{index}"),
                room_id: "!r:example.org".to_owned(),
                name: "Widget".to_owned(),
                origin: "https://widgets.example.org".to_owned(),
                kind: WidgetKind::RoomState,
            })
            .expect("insert within cap");
    }
    let overflow = registry.insert(WidgetSessionRecord {
        session_id: "overflow".to_owned(),
        widget_id: "overflow".to_owned(),
        room_id: "!r:example.org".to_owned(),
        name: "Overflow".to_owned(),
        origin: "https://widgets.example.org".to_owned(),
        kind: WidgetKind::RoomState,
    });
    assert_eq!(overflow, Err("experimental-widgets-registry-full"));
    assert_eq!(registry.len(), MAX_WIDGET_SESSIONS);

    registry.retire_idempotent();
    registry.retire_idempotent();
    assert!(registry.is_empty());
    assert_eq!(
        registry.insert(WidgetSessionRecord {
            session_id: "after".to_owned(),
            widget_id: "after".to_owned(),
            room_id: "!r:example.org".to_owned(),
            name: "After".to_owned(),
            origin: "https://widgets.example.org".to_owned(),
            kind: WidgetKind::RoomState,
        }),
        Err("experimental-widgets-session-not-live")
    );
}

#[test]
fn capability_filter_denies_send_without_grant_and_never_grants_rtc() {
    let requested = Capabilities {
        read: vec![message_filter(MessageLikeEventType::RoomMessage)],
        send: vec![
            message_filter(MessageLikeEventType::RoomMessage),
            message_filter(MessageLikeEventType::Reaction),
        ],
        requires_client: true,
        update_delayed_event: true,
        send_delayed_event: true,
        download_file: true,
        rtc_transports: true,
    };

    let denied = filter_requested_capabilities(
        requested.clone(),
        &WidgetGrantPolicy {
            receive_room: true,
            send_room_message: false,
        },
    );
    assert!(denied.send.is_empty());
    assert_eq!(denied.read.len(), 1);
    assert!(!denied.rtc_transports);
    assert!(!denied.download_file);
    assert!(!denied.update_delayed_event);
    assert!(!denied.send_delayed_event);

    let granted = filter_requested_capabilities(
        requested,
        &WidgetGrantPolicy {
            receive_room: true,
            send_room_message: true,
        },
    );
    assert_eq!(granted.send.len(), 1);
    assert!(!granted.rtc_transports);
    assert!(!granted.download_file);
}

#[test]
fn capability_filter_clears_read_when_receive_is_denied() {
    let requested = Capabilities {
        read: vec![message_filter(MessageLikeEventType::RoomMessage)],
        send: vec![message_filter(MessageLikeEventType::RoomMessage)],
        requires_client: false,
        update_delayed_event: false,
        send_delayed_event: false,
        download_file: false,
        rtc_transports: false,
    };
    let denied = filter_requested_capabilities(requested, &WidgetGrantPolicy::default());
    assert!(denied.read.is_empty());
    assert!(denied.send.is_empty());
    assert!(!denied.rtc_transports);
}

#[test]
fn target_origin_is_scheme_and_authority_only() {
    assert_eq!(
        widget_target_origin("https://widgets.example.org/app?x=1#frag").as_deref(),
        Some("https://widgets.example.org")
    );
    assert_eq!(
        widget_target_origin("http://127.0.0.1:8765/agent").as_deref(),
        Some("http://127.0.0.1:8765")
    );
}
