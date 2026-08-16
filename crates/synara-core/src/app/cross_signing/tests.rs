//! Unit tests for P8.4 cross-signing / identity store.

use super::*;
use crate::transport::MatrixIpcErrorCategory;

fn remote(user: &str, trust: IdentityTrust) -> RemoteIdentity {
    RemoteIdentity {
        user_id: user.into(),
        trust,
        has_master_key: true,
    }
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_cross_signing_markers(), MATRIX_CROSS_SIGNING_MARKER);
}

#[test]
fn local_keys_usable() {
    let mut store = CrossSigningStore::new(1);
    assert!(store.needs_attention());
    store.set_local_keys(LocalCrossSigningKeys {
        has_master: true,
        has_self_signing: true,
        has_user_signing: true,
        private_keys_cached: false,
    });
    assert!(store.local_keys().public_complete());
    assert!(!store.local_keys().fully_usable());
    assert!(store.needs_attention());
    store.set_private_keys_cached(true);
    assert!(store.local_keys().fully_usable());
    assert!(!store.needs_attention());
}

#[test]
fn remote_list_order_and_trust() {
    let mut store = CrossSigningStore::new(1);
    store
        .upsert_remote(remote("@c:ex.org", IdentityTrust::Unknown))
        .unwrap();
    store
        .upsert_remote(remote("@a:ex.org", IdentityTrust::Verified))
        .unwrap();
    store
        .upsert_remote(remote("@b:ex.org", IdentityTrust::PinViolation))
        .unwrap();
    store
        .upsert_remote(remote("@d:ex.org", IdentityTrust::Unverified))
        .unwrap();
    let list = store.list_remote();
    assert_eq!(list.len(), 4);
    assert_eq!(list[0].user_id, "@a:ex.org");
    assert_eq!(list[1].user_id, "@b:ex.org");
    assert_eq!(list[2].user_id, "@d:ex.org");
    assert_eq!(list[3].user_id, "@c:ex.org");
    assert_eq!(store.verified_count(), 1);
    assert_eq!(store.pin_violation_count(), 1);
    assert!(store.needs_attention()); // pin violation
}

#[test]
fn set_trust_and_remove() {
    let mut store = CrossSigningStore::new(1);
    store
        .upsert_remote(remote("@u:ex.org", IdentityTrust::Unverified))
        .unwrap();
    store
        .set_trust("@u:ex.org", IdentityTrust::Verified)
        .unwrap();
    assert_eq!(
        store.get_remote("@u:ex.org").unwrap().trust,
        IdentityTrust::Verified
    );
    assert!(store.remove_remote("@u:ex.org").is_some());
    assert!(store.get_remote("@u:ex.org").is_none());
}

#[test]
fn invalid_user_and_cap() {
    let mut store = CrossSigningStore::new(1);
    let err = store
        .upsert_remote(RemoteIdentity {
            user_id: "not-mxid".into(),
            trust: IdentityTrust::Unknown,
            has_master_key: false,
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.4-invalid-user-id");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);

    for i in 0..MAX_TRACKED_IDENTITIES {
        store
            .upsert_remote(remote(&format!("@u{i}:ex.org"), IdentityTrust::Unknown))
            .unwrap();
    }
    let err = store
        .upsert_remote(remote("@overflow:ex.org", IdentityTrust::Unknown))
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.4-identity-cap");
    // overwrite existing ok
    store
        .upsert_remote(remote("@u0:ex.org", IdentityTrust::Verified))
        .unwrap();
}

#[test]
fn local_user_and_retire() {
    let mut store = CrossSigningStore::new(2);
    store.set_local_user_id(Some("@me:ex.org".into())).unwrap();
    assert_eq!(store.local_user_id(), Some("@me:ex.org"));
    let err = store.set_local_user_id(Some("bad".into())).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.4-invalid-user-id");
    store.set_local_keys(LocalCrossSigningKeys {
        has_master: true,
        has_self_signing: true,
        has_user_signing: true,
        private_keys_cached: true,
    });
    store
        .upsert_remote(remote("@x:ex.org", IdentityTrust::Verified))
        .unwrap();
    store.retire_generation(3);
    assert_eq!(store.session_generation(), 3);
    assert!(store.is_empty());
    assert!(store.needs_attention());
}
