//! Unit tests for P4.5 space hierarchy foundation.

use super::*;
use crate::app::room_list::RoomSummaryBuilder;
use crate::dto::SpaceSummary;

fn space(
    id: &str,
    name: &str,
    children: Vec<crate::dto::SpaceChild>,
    parents: Option<Vec<String>>,
) -> SpaceSummary {
    SpaceSummary {
        room_id: id.into(),
        name: Some(name.into()),
        avatar_url: None,
        children,
        parent_room_ids: parents,
    }
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_spaces_markers(), MATRIX_SPACES_MARKER);
}

#[test]
fn hierarchy_children_order_and_descendants() {
    let mut h = SpaceHierarchy::new();
    h.replace_all(vec![
        space(
            "!root:example.org",
            "Root",
            vec![
                space_child("!sub:example.org", Some("b")),
                space_child("!roomA:example.org", Some("a")),
            ],
            None,
        ),
        space(
            "!sub:example.org",
            "Sub",
            vec![space_child("!roomB:example.org", None)],
            Some(vec!["!root:example.org".into()]),
        ),
    ])
    .unwrap();

    assert_eq!(h.len(), 2);
    let direct = h.direct_child_ids("!root:example.org").unwrap();
    // order "a" then "b"
    assert_eq!(
        direct,
        vec![
            "!roomA:example.org".to_string(),
            "!sub:example.org".to_string()
        ]
    );

    let desc = h.descendant_room_ids("!root:example.org").unwrap();
    assert!(desc.contains(&"!roomA:example.org".into()));
    assert!(desc.contains(&"!sub:example.org".into()));
    assert!(desc.contains(&"!roomB:example.org".into()));
    assert_eq!(desc.len(), 3);

    let roots = h.root_spaces();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].room_id, "!root:example.org");
}

#[test]
fn filter_rooms_in_space() {
    let mut h = SpaceHierarchy::new();
    h.replace_all(vec![space(
        "!space:example.org",
        "Space",
        vec![
            space_child("!in:example.org", None),
            space_child("!also:example.org", None),
        ],
        None,
    )])
    .unwrap();

    let rooms = vec![
        RoomSummaryBuilder::new("!in:example.org")
            .name("In")
            .build()
            .unwrap(),
        RoomSummaryBuilder::new("!out:example.org")
            .name("Out")
            .build()
            .unwrap(),
        RoomSummaryBuilder::new("!also:example.org")
            .name("Also")
            .build()
            .unwrap(),
    ];
    let filtered = h
        .filter_rooms_in_space("!space:example.org", &rooms)
        .unwrap();
    assert_eq!(filtered.len(), 2);
    assert!(filtered.iter().all(|r| r.room_id != "!out:example.org"));
}

#[test]
fn parent_cycle_rejected() {
    let mut h = SpaceHierarchy::new();
    let err = h
        .replace_all(vec![
            space(
                "!a:example.org",
                "A",
                vec![],
                Some(vec!["!b:example.org".into()]),
            ),
            space(
                "!b:example.org",
                "B",
                vec![],
                Some(vec!["!a:example.org".into()]),
            ),
        ])
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p4.5-space-parent-cycle");
}

#[test]
fn missing_space_errors() {
    let h = SpaceHierarchy::new();
    let err = h.direct_child_ids("!missing:example.org").unwrap_err();
    assert_eq!(err.diagnostic_id(), "p4.5-space-not-found");
}

#[test]
fn privacy_safe_errors() {
    let err = SpaceError::Cycle {
        diagnostic_id: "p4.5-space-parent-cycle",
    };
    let s = err.to_string();
    assert!(!s.contains("access_token"));
    assert_eq!(
        err.category(),
        crate::transport::MatrixIpcErrorCategory::SdkInvariant
    );
}
