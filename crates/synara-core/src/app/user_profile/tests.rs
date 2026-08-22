//! Unit tests for P6.6 user profile / ignore index.

use super::*;
use crate::transport::MatrixIpcErrorCategory;

#[test]
fn marker_stable() {
    assert_eq!(matrix_user_profile_markers(), MATRIX_USER_PROFILE_MARKER);
}

#[test]
fn own_profile_read_dto_is_mxc_only() {
    let profile = MatrixOwnProfile {
        user_id: "@alice:example.org".into(),
        display_name: Some("Alice".into()),
        avatar_url: Some("mxc://example.org/abc".into()),
    };
    let json = serde_json::to_value(&profile).expect("own profile serializes");
    assert_eq!(json["userId"], "@alice:example.org");
    assert_eq!(json["displayName"], "Alice");
    assert_eq!(json["avatarUrl"], "mxc://example.org/abc");
    assert!(json.get("bytes").is_none());
    assert!(parse_own_avatar_mxc("mxc://example.org/abc")
        .unwrap()
        .is_some());
    assert!(parse_own_avatar_mxc("data:image/png;base64,AAAA").is_err());
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
fn user_directory_search_validates_term_and_limit_without_echo() {
    assert_eq!(
        parse_user_directory_term("   ").unwrap_err(),
        "v-search.directory-empty-term"
    );
    let too_long = "t".repeat(MAX_USER_DIRECTORY_TERM_CHARS + 1);
    assert_eq!(
        parse_user_directory_term(&too_long).unwrap_err(),
        "v-search.directory-term-too-long"
    );
    assert_eq!(
        parse_user_directory_limit(Some(0)).unwrap_err(),
        "v-search.directory-invalid-limit"
    );
    assert_eq!(
        parse_user_directory_limit(Some(MAX_USER_DIRECTORY_LIMIT + 1)).unwrap_err(),
        "v-search.directory-invalid-limit"
    );
    assert_eq!(
        parse_user_directory_limit(None).unwrap(),
        DEFAULT_USER_DIRECTORY_LIMIT
    );
    assert_eq!(parse_user_directory_term("alice").unwrap(), "alice");
}

#[test]
fn user_directory_search_dto_is_mxc_and_ids_only() {
    let result = MatrixUserDirectorySearchResult {
        limited: true,
        results: vec![MatrixUserDirectoryHit {
            user_id: "@bob:example.org".into(),
            display_name: Some("Bob".into()),
            avatar_url: Some("mxc://example.org/abc".into()),
        }],
    };
    let json = serde_json::to_value(&result).expect("user directory serializes");
    assert_eq!(json["limited"], true);
    assert_eq!(json["results"][0]["userId"], "@bob:example.org");
    assert_eq!(json["results"][0]["displayName"], "Bob");
    assert_eq!(json["results"][0]["avatarUrl"], "mxc://example.org/abc");
    assert!(json.get("token").is_none());
    assert!(json.get("term").is_none());
    assert!(json["results"][0].get("bytes").is_none());
}

#[test]
fn ignored_users_snapshot_dto_is_ids_only() {
    let snapshot = MatrixIgnoredUsersSnapshot {
        user_ids: vec!["@spam:example.org".into()],
    };
    let json = serde_json::to_value(&snapshot).expect("ignored users serializes");
    assert_eq!(json["userIds"][0], "@spam:example.org");
    assert!(json.get("token").is_none());
}

#[test]
fn threepid_snapshot_dto_is_addresses_only() {
    let snapshot = MatrixThreepidSnapshot {
        emails: vec![MatrixThreepidEmail {
            address: "alice@example.org".into(),
        }],
    };
    let json = serde_json::to_value(&snapshot).expect("threepid serializes");
    assert_eq!(json["emails"][0]["address"], "alice@example.org");
    assert!(json.get("clientSecret").is_none());
    assert!(json.get("sid").is_none());
    let token = MatrixThreepidEmailTokenResult {
        session_id: "sid123".into(),
    };
    let token_json = serde_json::to_value(&token).expect("token result serializes");
    assert_eq!(token_json["sessionId"], "sid123");
    assert!(token_json.get("clientSecret").is_none());
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
