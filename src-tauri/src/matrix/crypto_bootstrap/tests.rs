//! Unit tests for P8.9 crypto bootstrap coordinator.

use super::*;

#[test]
fn marker_stable() {
    assert_eq!(
        matrix_crypto_bootstrap_markers(),
        MATRIX_CRYPTO_BOOTSTRAP_MARKER
    );
}

#[test]
fn happy_path_ready() {
    let mut c = CryptoBootstrapCoordinator::new(1);
    c.begin().unwrap();
    assert_eq!(c.phase(), BootstrapPhase::InProgress);
    for step in [
        BootstrapStep::StoreReady,
        BootstrapStep::CrossSigningReady,
        BootstrapStep::DeviceListReady,
        BootstrapStep::BackupReady,
    ] {
        c.set_step(step, true).unwrap();
    }
    // verification optional by default
    assert_eq!(c.phase(), BootstrapPhase::Ready);
    assert!(c.is_dogfood_ready());
    assert!(c.pending_labels().is_empty());
}

#[test]
fn degraded_when_backup_skipped() {
    let mut c = CryptoBootstrapCoordinator::new(2);
    c.begin().unwrap();
    c.set_backup_optional(true);
    c.set_step(BootstrapStep::StoreReady, true).unwrap();
    c.set_step(BootstrapStep::CrossSigningReady, true).unwrap();
    c.set_step(BootstrapStep::DeviceListReady, true).unwrap();
    assert_eq!(c.phase(), BootstrapPhase::Degraded);
    assert!(c.is_dogfood_ready());
    assert!(!c.pending_labels().iter().any(|l| l == "backup_ready"));
}

#[test]
fn fail_and_retire() {
    let mut c = CryptoBootstrapCoordinator::new(3);
    c.begin().unwrap();
    c.fail("p8.9-store-corrupt").unwrap();
    assert_eq!(c.phase(), BootstrapPhase::Failed);
    assert_eq!(c.failure_diagnostic_id(), Some("p8.9-store-corrupt"));
    assert!(!c.is_dogfood_ready());
    c.retire_generation(9);
    assert_eq!(c.session_generation(), 9);
    assert_eq!(c.phase(), BootstrapPhase::Idle);
}

#[test]
fn cannot_set_before_begin() {
    let mut c = CryptoBootstrapCoordinator::new(1);
    let err = c.set_step(BootstrapStep::StoreReady, true).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.9-not-started");
}

#[test]
fn pending_labels_track_required() {
    let mut c = CryptoBootstrapCoordinator::new(1);
    c.begin().unwrap();
    c.set_step(BootstrapStep::StoreReady, true).unwrap();
    let pending = c.pending_labels();
    assert!(pending.iter().any(|l| l == "cross_signing_ready"));
    assert!(pending.iter().any(|l| l == "device_list_ready"));
    assert!(pending.iter().any(|l| l == "backup_ready"));
}
