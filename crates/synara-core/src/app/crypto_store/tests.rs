//! Unit tests for P8.8 crypto-store continuity.

use super::*;

#[test]
fn marker_stable() {
    assert_eq!(matrix_crypto_store_markers(), MATRIX_CRYPTO_STORE_MARKER);
}

#[test]
fn open_ready_continuous_reopen() {
    let mut c = CryptoStoreContinuity::new(1);
    c.begin_open().unwrap();
    c.mark_ready(false).unwrap();
    assert_eq!(c.phase(), CryptoStorePhase::Ready);
    assert_eq!(c.open_count(), 1);
    assert!(!c.last_reopen_continuous());

    c.close();
    c.begin_open().unwrap();
    c.mark_ready(true).unwrap();
    assert!(c.last_reopen_continuous());
    assert_eq!(c.continuity_ok_count(), 1);
    assert_eq!(c.recommended_action(), CryptoStoreAction::None);
    assert!(c.never_auto_wipes());
}

#[test]
fn corrupt_offers_manual_recovery_never_wipe() {
    let mut c = CryptoStoreContinuity::new(1);
    c.begin_open().unwrap();
    let action = c
        .fail(CryptoStoreHealth::Corrupt, "p8.8-sqlite-corrupt")
        .unwrap();
    assert_eq!(action, CryptoStoreAction::OfferManualRecovery);
    assert!(!action.requests_wipe());
    assert_eq!(c.phase(), CryptoStorePhase::Failed);
    assert!(c.never_auto_wipes());
}

#[test]
fn locked_and_missing_actions() {
    let mut c = CryptoStoreContinuity::new(1);
    c.begin_open().unwrap();
    let a = c
        .fail(CryptoStoreHealth::Locked, "p8.8-store-locked")
        .unwrap();
    assert_eq!(a, CryptoStoreAction::WaitUnlock);

    let mut c = CryptoStoreContinuity::new(1);
    c.begin_open().unwrap();
    let a = c
        .fail(CryptoStoreHealth::Missing, "p8.8-store-missing")
        .unwrap();
    assert_eq!(a, CryptoStoreAction::CreateFresh);
}

#[test]
fn degraded_retry_and_forbid_secrets() {
    let mut c = CryptoStoreContinuity::new(1);
    c.begin_open().unwrap();
    c.mark_ready(true).unwrap();
    c.mark_degraded("p8.8-slow-io").unwrap();
    assert_eq!(c.recommended_action(), CryptoStoreAction::RetryOpen);
    let err = c
        .fail(CryptoStoreHealth::Corrupt, "recovery_key=abc")
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.8-forbidden-diagnostic");
}

#[test]
fn retire_generation() {
    let mut c = CryptoStoreContinuity::new(1);
    c.begin_open().unwrap();
    c.mark_ready(true).unwrap();
    c.retire_generation(9);
    assert_eq!(c.session_generation(), 9);
    assert_eq!(c.phase(), CryptoStorePhase::Closed);
    assert_eq!(c.open_count(), 0);
}
