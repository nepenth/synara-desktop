//! Unit tests for P5.11 timeline filter.

use super::*;

fn msg(sender: &str) -> FilterableItem {
    FilterableItem {
        event_id: Some("$e:example.org".into()),
        sender: Some(sender.into()),
        kind: TimelineItemKind::Message,
        is_local_echo: false,
        is_redacted: false,
    }
}

#[test]
fn marker_stable() {
    assert_eq!(
        matrix_timeline_filter_markers(),
        MATRIX_TIMELINE_FILTER_MARKER
    );
}

#[test]
fn default_allows_all() {
    let f = TimelineFilter::new();
    assert!(f.allows(&msg("@a:example.org")));
    assert!(f.allows(&FilterableItem {
        event_id: None,
        sender: None,
        kind: TimelineItemKind::State,
        is_local_echo: true,
        is_redacted: true,
    }));
}

#[test]
fn kinds_and_senders() {
    let f = TimelineFilter::new()
        .with_kinds(vec![TimelineItemKind::Message, TimelineItemKind::Poll])
        .unwrap()
        .with_senders(vec!["@a:example.org".into()])
        .unwrap();
    assert!(f.allows(&msg("@a:example.org")));
    assert!(!f.allows(&msg("@b:example.org")));
    assert!(!f.allows(&FilterableItem {
        event_id: None,
        sender: Some("@a:example.org".into()),
        kind: TimelineItemKind::State,
        is_local_echo: false,
        is_redacted: false,
    }));
}

#[test]
fn flags_local_redacted_encrypted() {
    let mut f = TimelineFilter::new();
    f.include_local_echo = false;
    f.include_redacted = false;
    f.include_encrypted = false;
    assert!(!f.allows(&FilterableItem {
        event_id: None,
        sender: None,
        kind: TimelineItemKind::Message,
        is_local_echo: true,
        is_redacted: false,
    }));
    assert!(!f.allows(&FilterableItem {
        event_id: None,
        sender: None,
        kind: TimelineItemKind::Message,
        is_local_echo: false,
        is_redacted: true,
    }));
    assert!(!f.allows(&FilterableItem {
        event_id: None,
        sender: None,
        kind: TimelineItemKind::Encrypted,
        is_local_echo: false,
        is_redacted: false,
    }));
}

#[test]
fn select_indices() {
    let f = TimelineFilter::new()
        .with_kinds(vec![TimelineItemKind::Message])
        .unwrap();
    let items = vec![
        msg("@a:example.org"),
        FilterableItem {
            event_id: None,
            sender: None,
            kind: TimelineItemKind::State,
            is_local_echo: false,
            is_redacted: false,
        },
        msg("@b:example.org"),
    ];
    assert_eq!(f.select_indices(&items), vec![0, 2]);
}

#[test]
fn invalid_sender() {
    let err = TimelineFilter::new()
        .with_senders(vec!["not-user".into()])
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.11-invalid-sender");
}
