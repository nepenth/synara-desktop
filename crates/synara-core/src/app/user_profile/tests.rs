//! Unit tests for P6.6 user profile / ignore index.

use super::*;
use crate::transport::MatrixIpcErrorCategory;

#[test]
fn marker_stable() {
    assert_eq!(matrix_user_profile_markers(), MATRIX_USER_PROFILE_MARKER);
}

#[test]
fn own_and_peer_profiles() {
    let mut idx = UserProfileIndex::new(2);
    idx.set_own_user_id("@alice:example.org").unwrap();
    idx.set_own_profile(UserProfile {
        user_id: "@alice:example.org".into(),
        display_name: Some("Alice".into()),
        avatar_url: Some("mxc://example.org/abc".into()),
    })
    .unwrap();
    idx.upsert_peer(UserProfile {
        user_id: "@bob:example.org".into(),
        display_name: Some("Bob".into()),
        avatar_url: None,
    })
    .unwrap();
    assert_eq!(
        idx.own_profile().unwrap().display_name.as_deref(),
        Some("Alice")
    );
    assert_eq!(
        idx.get("@bob:example.org").unwrap().display_name.as_deref(),
        Some("Bob")
    );
    assert_eq!(idx.peer_count(), 1);
}

#[test]
fn ignore_list() {
    let mut idx = UserProfileIndex::new(1);
    idx.set_own_user_id("@me:example.org").unwrap();
    idx.ignore_user("@spam:example.org").unwrap();
    idx.ignore_user("@troll:example.org").unwrap();
    assert!(idx.is_ignored("@spam:example.org"));
    assert_eq!(idx.ignored_users().len(), 2);
    idx.unignore_user("@spam:example.org").unwrap();
    assert!(!idx.is_ignored("@spam:example.org"));
    let err = idx.ignore_user("@me:example.org").unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.6-cannot-ignore-self");
}

#[test]
fn forbids_data_avatar_and_invalid_ids() {
    let mut idx = UserProfileIndex::new(1);
    let err = idx
        .set_own_profile(UserProfile {
            user_id: "@a:example.org".into(),
            display_name: None,
            avatar_url: Some("data:image/png;base64,AAAA".into()),
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.6-forbidden-avatar-scheme");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);
    let err = idx
        .upsert_peer(UserProfile {
            user_id: "not-a-user".into(),
            display_name: None,
            avatar_url: None,
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.6-invalid-user-id");
}

#[test]
fn peer_cap_enforced() {
    let mut idx = UserProfileIndex::new(1);
    for i in 0..MAX_CACHED_PROFILES {
        idx.upsert_peer(UserProfile {
            user_id: format!("@u{i}:example.org"),
            display_name: None,
            avatar_url: None,
        })
        .unwrap();
    }
    let err = idx
        .upsert_peer(UserProfile {
            user_id: "@overflow:example.org".into(),
            display_name: None,
            avatar_url: None,
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p6.6-peer-profile-cap");
}

#[test]
fn retire_generation_wipes() {
    let mut idx = UserProfileIndex::new(1);
    idx.set_own_user_id("@a:example.org").unwrap();
    idx.ignore_user("@b:example.org").unwrap();
    idx.retire_generation(4);
    assert_eq!(idx.session_generation(), 4);
    assert!(idx.is_empty());
}
