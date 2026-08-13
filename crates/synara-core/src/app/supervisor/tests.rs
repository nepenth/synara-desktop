//! Deterministic unit tests for P2.1 Matrix supervisor actor.

use super::actor::{harness_login_ready, harness_restore_ready, MatrixSupervisor};
use super::handle::{FailingClientFactory, NullClientFactory, TestClientFactory};
use super::state::SupervisorState;
use super::transition::SupervisorCommand;
use super::*;
use crate::dto::SessionLifecycle;
use crate::transport::MatrixIpcErrorCategory;

#[test]
fn marker_and_command_table_non_empty() {
    assert_eq!(matrix_supervisor_markers(), MATRIX_SUPERVISOR_MARKER);
    assert!(SupervisorCommand::ALL.len() >= 12);
    assert_eq!(SupervisorState::ALL.len(), 10);
    assert_eq!(SessionLifecycle::ALL.len(), 10);
    assert!(!SupervisorEvent::ALL.is_empty());
}

#[test]
fn supervisor_state_aligns_with_session_lifecycle_wire_names() {
    for state in SupervisorState::ALL {
        let life: SessionLifecycle = (*state).into();
        let back: SupervisorState = life.into();
        assert_eq!(back, *state);
        assert_eq!(state.as_str(), life.as_str());
    }
}

#[test]
fn fresh_supervisor_is_empty() {
    let s = MatrixSupervisor::new();
    assert_eq!(s.state(), SupervisorState::Empty);
    assert_eq!(s.lifecycle(), SessionLifecycle::Empty);
    assert_eq!(s.session_generation(), 0);
    assert!(!s.has_client());
    assert_eq!(s.live_handles(), 0);
    assert_eq!(s.installed_total(), 0);
    assert_eq!(s.shutdown_total(), 0);
    assert!(s.last_failure().is_none());
}

#[test]
fn happy_path_login_to_ready() {
    let mut s = MatrixSupervisor::new();
    let factory = TestClientFactory::new();
    harness_login_ready(&mut s, &factory).unwrap();
    assert_eq!(s.state(), SupervisorState::Ready);
    assert_eq!(s.session_generation(), 1);
    assert!(s.has_client());
    assert_eq!(s.live_handles(), 1);
    assert_eq!(s.installed_total(), 1);
    assert!(s.state().allows_publish());
}

#[test]
fn happy_path_restore_to_ready() {
    let mut s = MatrixSupervisor::new();
    let factory = TestClientFactory::new();
    harness_restore_ready(&mut s, &factory).unwrap();
    assert_eq!(s.state(), SupervisorState::Ready);
    assert_eq!(s.session_generation(), 1);
    assert!(s.has_client());
}

#[test]
fn open_close_logout_cycle_leaks_no_handles_and_bumps_generation() {
    let mut s = MatrixSupervisor::new();
    let factory = TestClientFactory::new();

    // Cycle i: BeginOpen → gen 2*i-1; CompleteLogout → gen 2*i.
    for cycle in 1..=3 {
        harness_login_ready(&mut s, &factory).unwrap();
        assert_eq!(s.live_handles(), 1);
        let gen_ready = s.session_generation();
        assert_eq!(gen_ready, 2 * cycle - 1);
        assert!(s.may_publish(gen_ready));

        s.apply(SupervisorCommand::BeginStop).unwrap();
        assert!(!s.may_publish(gen_ready));

        s.apply(SupervisorCommand::CompleteLogout).unwrap();
        assert_eq!(s.state(), SupervisorState::LoggedOut);
        assert!(!s.has_client());
        assert_eq!(s.live_handles(), 0);
        assert_eq!(s.installed_total(), cycle);
        assert_eq!(s.shutdown_total(), cycle);
        assert_eq!(s.session_generation(), 2 * cycle);
        assert!(!s.is_live_generation(gen_ready));
        assert!(!s.may_publish(gen_ready));
    }
    // After 3 logouts generation is 6; next open → 7.
    s.apply(SupervisorCommand::BeginOpen).unwrap();
    assert_eq!(s.session_generation(), 7);
    assert_eq!(s.state(), SupervisorState::Opening);
}

#[test]
fn wipe_cycle_returns_empty_and_bumps_generation() {
    let mut s = MatrixSupervisor::new();
    let factory = TestClientFactory::new();
    harness_login_ready(&mut s, &factory).unwrap();
    let gen_ready = s.session_generation();
    s.apply(SupervisorCommand::BeginWipe).unwrap();
    assert_eq!(s.state(), SupervisorState::Wiping);
    // R0.5 / REV-001: client is dropped at BeginWipe, not CompleteWipe.
    assert!(!s.has_client());
    s.apply(SupervisorCommand::CompleteWipe).unwrap();
    assert_eq!(s.state(), SupervisorState::Empty);
    assert!(!s.has_client());
    assert_eq!(s.live_handles(), 0);
    assert_eq!(s.shutdown_total(), 1);
    assert_eq!(s.session_generation(), gen_ready + 1);
}

#[test]
fn logout_and_wipe_are_distinct_paths() {
    let mut s = MatrixSupervisor::new();
    let factory = TestClientFactory::new();
    harness_login_ready(&mut s, &factory).unwrap();
    s.apply(SupervisorCommand::BeginStop).unwrap();
    s.apply(SupervisorCommand::CompleteLogout).unwrap();
    assert_eq!(s.state(), SupervisorState::LoggedOut);
    s.apply(SupervisorCommand::BeginWipe).unwrap();
    s.apply(SupervisorCommand::CompleteWipe).unwrap();
    assert_eq!(s.state(), SupervisorState::Empty);
}

#[test]
fn fail_drops_client_and_records_category() {
    let mut s = MatrixSupervisor::new();
    let factory = TestClientFactory::new();
    harness_login_ready(&mut s, &factory).unwrap();
    s.fail(
        MatrixIpcErrorCategory::Connectivity,
        "p2.1-test-connectivity",
    )
    .unwrap();
    assert_eq!(s.state(), SupervisorState::Failed);
    assert!(!s.has_client());
    let f = s.last_failure().expect("failure recorded");
    assert_eq!(f.category, MatrixIpcErrorCategory::Connectivity);
    assert_eq!(f.diagnostic_id, "p2.1-test-connectivity");
}

#[test]
fn illegal_transitions_rejected() {
    let mut s = MatrixSupervisor::new();
    assert!(s.apply(SupervisorCommand::BeginAuthenticate).is_err());
    assert!(s.apply(SupervisorCommand::BeginSync).is_err());
    assert!(s.apply(SupervisorCommand::MarkReady).is_err());
    assert!(s.apply(SupervisorCommand::InstallClient).is_err());
    assert!(s.apply(SupervisorCommand::BeginWipe).is_err());

    s.apply(SupervisorCommand::BeginOpen).unwrap();
    assert!(s.apply(SupervisorCommand::BeginSync).is_err());
    s.apply(SupervisorCommand::BeginAuthenticate).unwrap();
    assert!(s.apply(SupervisorCommand::BeginRestore).is_err());
}

#[test]
fn sync_and_ready_require_installed_client() {
    let mut s = MatrixSupervisor::new();
    s.apply(SupervisorCommand::BeginOpen).unwrap();
    s.apply(SupervisorCommand::BeginAuthenticate).unwrap();
    let err = s.apply(SupervisorCommand::BeginSync).unwrap_err();
    assert!(matches!(err, SupervisorError::ClientMissing));
}

#[test]
fn install_client_is_sole_construction_path() {
    let mut s = MatrixSupervisor::new();
    let factory = TestClientFactory::new();
    s.apply(SupervisorCommand::BeginOpen).unwrap();
    s.apply(SupervisorCommand::BeginAuthenticate).unwrap();
    let err = s.apply(SupervisorCommand::InstallClient).unwrap_err();
    assert!(matches!(
        err,
        SupervisorError::ConstructionFailed {
            diagnostic_id: "p2.1-install-without-factory",
            ..
        }
    ));
    assert!(!s.has_client());
    s.apply_with_factory(SupervisorCommand::InstallClient, &factory)
        .unwrap();
    assert!(s.has_client());
    let err = s
        .apply_with_factory(SupervisorCommand::InstallClient, &factory)
        .unwrap_err();
    assert!(matches!(err, SupervisorError::ClientAlreadyPresent));
}

#[test]
fn factory_failure_does_not_change_lifecycle_or_leak() {
    let mut s = MatrixSupervisor::new();
    s.apply(SupervisorCommand::BeginOpen).unwrap();
    s.apply(SupervisorCommand::BeginRestore).unwrap();
    assert_eq!(s.state(), SupervisorState::Restoring);
    let err = s
        .apply_with_factory(SupervisorCommand::InstallClient, &FailingClientFactory)
        .unwrap_err();
    assert!(matches!(
        err,
        SupervisorError::ConstructionFailed {
            category: MatrixIpcErrorCategory::StoreUnavailable,
            ..
        }
    ));
    assert_eq!(s.state(), SupervisorState::Restoring);
    assert!(!s.has_client());
    assert_eq!(s.installed_total(), 0);
    assert_eq!(s.shutdown_total(), 0);
}

#[test]
fn retry_after_fail_bumps_generation() {
    let mut s = MatrixSupervisor::new();
    let factory = TestClientFactory::new();
    harness_login_ready(&mut s, &factory).unwrap();
    assert_eq!(s.session_generation(), 1);
    s.fail(MatrixIpcErrorCategory::SdkInvariant, "p2.1-test-fail")
        .unwrap();
    s.apply(SupervisorCommand::BeginOpen).unwrap();
    assert_eq!(s.session_generation(), 2);
    assert_eq!(s.state(), SupervisorState::Opening);
    assert!(s.last_failure().is_none());
}

#[test]
fn wipe_blocks_other_commands() {
    let mut s = MatrixSupervisor::new();
    let factory = TestClientFactory::new();
    harness_login_ready(&mut s, &factory).unwrap();
    s.apply(SupervisorCommand::BeginWipe).unwrap();
    assert!(s.apply(SupervisorCommand::BeginStop).is_err());
    assert!(s.apply(SupervisorCommand::BeginOpen).is_err());
    // P2.6: Fail is legal from Wiping so failed wipe I/O can exit without
    // CompleteWipe (no auto-delete completion).
    s.apply(SupervisorCommand::Fail).unwrap();
    assert_eq!(s.state(), SupervisorState::Failed);
}

#[test]
fn wipe_complete_after_begin_reaches_empty() {
    let mut s = MatrixSupervisor::new();
    let factory = TestClientFactory::new();
    harness_login_ready(&mut s, &factory).unwrap();
    s.apply(SupervisorCommand::BeginWipe).unwrap();
    s.apply(SupervisorCommand::CompleteWipe).unwrap();
    assert_eq!(s.state(), SupervisorState::Empty);
}

#[test]
fn snapshot_matches_live_fields() {
    let mut s = MatrixSupervisor::new();
    let factory = TestClientFactory::new();
    harness_restore_ready(&mut s, &factory).unwrap();
    let snap = s.snapshot();
    assert_eq!(snap.state, SupervisorState::Ready);
    assert_eq!(snap.lifecycle, SessionLifecycle::Ready);
    assert_eq!(snap.session_generation, 1);
    assert!(snap.has_client);
    assert_eq!(snap.live_handles, 1);
    assert!(snap.last_failure.is_none());
}

#[test]
fn can_apply_preflight_matches_apply() {
    let s = MatrixSupervisor::new();
    assert!(s.can_apply(SupervisorCommand::BeginOpen).is_ok());
    assert!(s.can_apply(SupervisorCommand::MarkReady).is_err());
}

#[test]
fn null_factory_refuses_construction() {
    let mut s = MatrixSupervisor::new();
    s.apply(SupervisorCommand::BeginOpen).unwrap();
    s.apply(SupervisorCommand::BeginAuthenticate).unwrap();
    let err = s
        .apply_with_factory(SupervisorCommand::InstallClient, &NullClientFactory)
        .unwrap_err();
    assert!(matches!(
        err,
        SupervisorError::ConstructionFailed {
            diagnostic_id: "p2.1-null-factory-no-client",
            ..
        }
    ));
}
