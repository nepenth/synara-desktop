//! Unit tests for P8.1 security status store.

use super::*;
use crate::dto::{BackupStatus, RecoveryStatus, SecurityStatus, VerificationState};
use crate::transport::MatrixIpcErrorCategory;

#[test]
fn marker_stable() {
    assert_eq!(matrix_security_markers(), MATRIX_SECURITY_MARKER);
}

#[test]
fn default_unknown() {
    let store = SecurityStatusStore::new(1);
    let s = store.snapshot();
    assert!(!s.cross_signing_active);
    assert_eq!(s.backup_status, BackupStatus::Unknown);
    assert_eq!(s.recovery_status, RecoveryStatus::Unknown);
    assert_eq!(s.verification_state, VerificationState::Unavailable);
    assert!(!store.needs_attention());
}

#[test]
fn apply_and_partial_setters() {
    let mut store = SecurityStatusStore::new(2);
    store
        .apply(SecurityStatus {
            cross_signing_active: true,
            backup_status: BackupStatus::Enabled,
            recovery_status: RecoveryStatus::Ready,
            verification_state: VerificationState::Verified,
            device_count: Some(3),
            has_pending_verification_requests: false,
        })
        .unwrap();
    assert!(!store.needs_attention());
    store.set_pending_verification_requests(true);
    assert!(store.needs_attention());
    store.set_pending_verification_requests(false);
    store.set_verification_state(VerificationState::Unverified);
    assert!(store.needs_attention());
    store.set_verification_state(VerificationState::Verified);
    store.set_recovery_status(RecoveryStatus::NotSetup);
    assert!(store.needs_attention());
    store.set_recovery_status(RecoveryStatus::Ready);
    store.set_backup_status(BackupStatus::Outdated);
    assert!(store.needs_attention());
}

#[test]
fn device_count_cap() {
    let mut store = SecurityStatusStore::new(1);
    let err = store.set_device_count(Some(10_001)).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.1-device-count-cap");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);
    store.set_device_count(Some(2)).unwrap();
    assert_eq!(store.snapshot().device_count, Some(2));
}

#[test]
fn retire_generation_resets() {
    let mut store = SecurityStatusStore::new(5);
    store
        .apply(SecurityStatus {
            cross_signing_active: true,
            backup_status: BackupStatus::Enabled,
            recovery_status: RecoveryStatus::Ready,
            verification_state: VerificationState::Verified,
            device_count: Some(1),
            has_pending_verification_requests: true,
        })
        .unwrap();
    store.retire_generation(6);
    assert_eq!(store.session_generation(), 6);
    assert!(!store.snapshot().cross_signing_active);
    assert!(!store.needs_attention());
}
