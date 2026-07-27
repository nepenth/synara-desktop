//! Unit tests for P9.1 widget registry.

use super::*;
use crate::matrix::dto::{WidgetKind, WidgetSession, WidgetSessionState};
use crate::matrix::ipc::MatrixIpcErrorCategory;

#[test]
fn marker_stable() {
    assert_eq!(matrix_widgets_markers(), MATRIX_WIDGETS_MARKER);
}

#[test]
fn begin_activate_list() {
    let mut reg = WidgetRegistry::new(1);
    let id = reg
        .begin("!r:example.org", WidgetKind::ElementCall)
        .unwrap();
    assert!(id.starts_with("widget-"));
    reg.set_state(&id, WidgetSessionState::Active).unwrap();
    reg.set_url(&id, Some("https://call.example.org/room".into()))
        .unwrap();
    let s = reg.get(&id).unwrap();
    assert_eq!(s.state, WidgetSessionState::Active);
    assert!(s.has_active_call);
    assert_eq!(reg.list(Some("!r:example.org")).len(), 1);
    assert!(reg.active_call_in_room("!r:example.org").is_some());
}

#[test]
fn forbid_token_in_url() {
    let mut reg = WidgetRegistry::new(1);
    let id = reg.begin("!r:example.org", WidgetKind::Custom).unwrap();
    let err = reg
        .set_url(&id, Some("https://x/?access_token=sekrit".into()))
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p9.1-forbidden-url-secret");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);
}

#[test]
fn invalid_room() {
    let mut reg = WidgetRegistry::new(1);
    let err = reg.begin("bad", WidgetKind::Custom).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p9.1-invalid-room-id");
}

#[test]
fn end_clears_active_call() {
    let mut reg = WidgetRegistry::new(1);
    let id = reg
        .begin("!r:example.org", WidgetKind::ElementCall)
        .unwrap();
    reg.set_state(&id, WidgetSessionState::Active).unwrap();
    reg.set_state(&id, WidgetSessionState::Ending).unwrap();
    assert!(!reg.get(&id).unwrap().has_active_call);
}

#[test]
fn remove_and_retire() {
    let mut reg = WidgetRegistry::new(2);
    let id = reg.begin("!r:example.org", WidgetKind::Custom).unwrap();
    assert!(reg.remove(&id));
    assert!(!reg.remove(&id));
    let id = reg.begin("!r:example.org", WidgetKind::Custom).unwrap();
    reg.retire_generation(3);
    assert_eq!(reg.session_generation(), 3);
    assert!(reg.get(&id).is_none());
    assert!(reg.is_empty());
}

#[test]
fn upsert_and_cap() {
    let mut reg = WidgetRegistry::new(1);
    for i in 0..MAX_WIDGET_SESSIONS {
        reg.upsert(WidgetSession {
            widget_id: format!("w{i}"),
            room_id: "!r:example.org".into(),
            kind: WidgetKind::Custom,
            state: WidgetSessionState::Idle,
            url: None,
            has_active_call: false,
        })
        .unwrap();
    }
    let err = reg
        .upsert(WidgetSession {
            widget_id: "overflow".into(),
            room_id: "!r:example.org".into(),
            kind: WidgetKind::Custom,
            state: WidgetSessionState::Idle,
            url: None,
            has_active_call: false,
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p9.1-session-cap");
    // Overwrite existing ok.
    reg.upsert(WidgetSession {
        widget_id: "w0".into(),
        room_id: "!r:example.org".into(),
        kind: WidgetKind::ElementCall,
        state: WidgetSessionState::Active,
        url: None,
        has_active_call: true,
    })
    .unwrap();
    assert_eq!(reg.get("w0").unwrap().kind, WidgetKind::ElementCall);
}
