//! Unit + temp-dir integration tests for P2.6 destructive lifecycle.

use super::*;
use crate::matrix::diagnostics::{MatrixMetrics, StoreHealthStatus};
use crate::matrix::ipc::MatrixIpcErrorCategory;
use crate::matrix::store::{
    get_or_create_store_key, AccountIdentity, InMemoryStoreKeyVault, StoreKeyId, StoreKeyVault,
    StorePaths,
};
use crate::matrix::supervisor::{
    harness_login_ready, MatrixSupervisor, SupervisorState, TestClientFactory,
};
use crate::matrix::tasks::{TaskKind, TaskSupervisor};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn temp_root(label: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "synara-p2.6-{}-{}-{}",
        label,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp root");
    fs::canonicalize(&dir).unwrap_or(dir)
}

fn alice() -> AccountIdentity {
    AccountIdentity::new("@alice:example.org", "https://example.org").unwrap()
}

fn bob() -> AccountIdentity {
    AccountIdentity::new("@bob:example.org", "https://example.org").unwrap()
}

fn seed_account_store(root: &Path, identity: &AccountIdentity) -> StorePaths {
    let paths = StorePaths::derive(root, identity).unwrap();
    paths.ensure_dirs().unwrap();
    fs::write(paths.state_dir().join("state.db"), b"state-blob").unwrap();
    fs::write(paths.crypto_dir().join("crypto.db"), b"crypto-blob").unwrap();
    fs::write(paths.cache_dir().join("cache.bin"), b"cache-blob").unwrap();
    fs::write(paths.media_dir().join("m1"), b"media-blob").unwrap();
    paths
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_lifecycle_markers(), MATRIX_LIFECYCLE_MARKER);
}

#[test]
fn recovery_policy_never_requests_wipe() {
    for kind in [
        StoreFailureKind::Corrupt,
        StoreFailureKind::Unavailable,
        StoreFailureKind::Locked,
    ] {
        let action = recovery_action_for(&StoreFailure::new(kind));
        assert!(!action.requests_wipe());
        assert_eq!(action.category, kind.ipc_category());
    }
}

#[test]
fn store_failure_surfaces_categories_without_deleting_dirs() {
    let root = temp_root("no-auto-wipe");
    let paths = seed_account_store(&root, &alice());
    let marker = paths.state_dir().join("state.db");
    assert_eq!(fs::read(&marker).unwrap(), b"state-blob");

    let mut metrics = MatrixMetrics::new();
    let mut supervisor = MatrixSupervisor::new();
    let factory = TestClientFactory::new();
    harness_login_ready(&mut supervisor, &factory).unwrap();

    let action = apply_store_failure(
        &StoreFailure::new(StoreFailureKind::Corrupt),
        Some(&mut metrics),
        Some(&mut supervisor),
    )
    .unwrap();

    assert!(!action.requests_wipe());
    assert_eq!(action.category, MatrixIpcErrorCategory::StoreCorrupt);
    assert_eq!(supervisor.state(), SupervisorState::Failed);
    assert_eq!(metrics.snapshot().store.status, StoreHealthStatus::Corrupt);
    assert!(metrics.snapshot().store.open_failures >= 1);
    assert_eq!(fs::read(&marker).unwrap(), b"state-blob");
    assert!(paths.crypto_dir().join("crypto.db").is_file());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn ensure_dirs_failure_is_not_a_wipe_signal() {
    let root = temp_root("ensure-not-wipe");
    let paths = seed_account_store(&root, &alice());

    let mut metrics = MatrixMetrics::new();
    let action = surface_store_unavailable(Some(&mut metrics), None).unwrap();
    assert!(!action.requests_wipe());
    assert_eq!(action.category, MatrixIpcErrorCategory::StoreUnavailable);
    assert!(paths.state_dir().join("state.db").is_file());
    assert!(paths.account_root().is_dir());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn wipe_target_refuses_relative_app_data_root() {
    let err = WipeTarget::resolve(Path::new("relative/root"), alice()).unwrap_err();
    assert!(matches!(
        err,
        LifecycleError::InvalidTarget {
            diagnostic_id: "p2.6-relative-app-data-root"
        }
    ));
}

#[test]
fn wipe_target_refuses_empty_app_data_root() {
    let err = WipeTarget::resolve(Path::new(""), alice()).unwrap_err();
    assert!(matches!(
        err,
        LifecycleError::InvalidTarget {
            diagnostic_id: "p2.6-empty-app-data-root"
        }
    ));
}

#[test]
fn wrong_path_refused_by_exact_target_checks() {
    let root = temp_root("wrong-path");
    let target = WipeTarget::resolve(&root, alice()).unwrap();

    let sibling = root.join("matrix").join("not-alice-segment");
    assert!(assert_path_is_wipe_allowed(&target, &sibling).is_err());
    assert!(assert_path_is_wipe_allowed(&target, target.matrix_root()).is_err());
    assert!(assert_path_is_wipe_allowed(&target, target.app_data_root()).is_err());
    assert!(assert_path_is_wipe_allowed(&target, Path::new("/tmp")).is_err());

    // Exact account root is the only allowed wipe path.
    assert!(assert_exact_account_root(&target).is_ok());
    assert!(assert_path_is_wipe_allowed(&target, target.account_root()).is_ok());
    // Child subdirs alone are not the wipe entrypoint (whole account root is).
    assert!(assert_path_is_wipe_allowed(&target, target.paths().state_dir()).is_err());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn wipe_removes_exact_account_only_sibling_untouched() {
    let root = temp_root("sibling");
    let alice_paths = seed_account_store(&root, &alice());
    let bob_paths = seed_account_store(&root, &bob());

    let target = WipeTarget::resolve(&root, alice()).unwrap();
    let vault = InMemoryStoreKeyVault::new();
    let report = wipe_account_store(&target, Some(&vault)).unwrap();

    assert_eq!(report.account_segment, alice_paths.account_segment());
    assert!(!alice_paths.account_root().exists());
    assert!(!alice_paths.state_dir().exists());

    assert!(bob_paths.account_root().is_dir());
    assert_eq!(
        fs::read(bob_paths.state_dir().join("state.db")).unwrap(),
        b"state-blob"
    );
    assert_eq!(
        fs::read(bob_paths.crypto_dir().join("crypto.db")).unwrap(),
        b"crypto-blob"
    );
    assert!(root.join("matrix").is_dir());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn wipe_is_idempotent_when_already_absent() {
    let root = temp_root("idempotent");
    let target = WipeTarget::resolve(&root, alice()).unwrap();
    let vault = InMemoryStoreKeyVault::new();
    let report = wipe_account_store(&target, Some(&vault)).unwrap();
    assert!(report.account_root_removed);

    let report2 = wipe_account_store(&target, Some(&vault)).unwrap();
    assert!(report2.account_root_removed);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn session_material_service_distinct_from_store_key() {
    let id = SessionMaterialId::from_identity(&alice());
    let store_id = StoreKeyId::from_identity(&alice());
    assert_eq!(id.service(), SESSION_MATERIAL_SERVICE);
    assert_ne!(id.service(), store_id.service());
    assert!(id.account().starts_with("matrix-session:"));
    assert!(store_id.account().starts_with("store-key:"));
}

#[test]
fn session_material_vault_clear_hooks() {
    let vault = InMemorySessionMaterialVault::new();
    let id = SessionMaterialId::from_identity(&alice());
    vault
        .set(&id, &SessionMaterial::from_placeholder(b"placeholder"))
        .unwrap();
    assert!(vault.get(&id).unwrap().is_some());
    assert!(clear_session_material(&vault, &alice()).unwrap());
    assert!(vault.get(&id).unwrap().is_none());
    assert!(!clear_session_material(&vault, &alice()).unwrap());
}

#[tokio::test]
async fn logout_drops_client_retires_tasks_clears_session_not_stores() {
    let root = temp_root("logout");
    let paths = seed_account_store(&root, &alice());

    let mut supervisor = MatrixSupervisor::new();
    let factory = TestClientFactory::new();
    harness_login_ready(&mut supervisor, &factory).unwrap();
    assert!(supervisor.has_client());
    let gen_before = supervisor.session_generation();

    let mut tasks = TaskSupervisor::new();
    tasks.set_live_generation(gen_before);
    let cancelled = Arc::new(AtomicUsize::new(0));
    let c = Arc::clone(&cancelled);
    let _id = tasks
        .spawn(TaskKind::Sync, gen_before, async move {
            std::future::pending::<()>().await;
            c.fetch_add(1, Ordering::SeqCst);
        })
        .unwrap();
    assert_eq!(tasks.registered_count(), 1);

    let session_vault = InMemorySessionMaterialVault::new();
    session_vault
        .set(
            &SessionMaterialId::from_identity(&alice()),
            &SessionMaterial::from_placeholder(b"sess"),
        )
        .unwrap();

    let mut metrics = MatrixMetrics::new();
    let outcome = perform_logout(
        &mut supervisor,
        &mut tasks,
        &session_vault,
        &alice(),
        Some(&mut metrics),
    )
    .await
    .unwrap();

    assert_eq!(supervisor.state(), SupervisorState::LoggedOut);
    assert!(!supervisor.has_client());
    assert!(outcome.stores_retained);
    assert!(outcome.session_material_cleared);
    assert!(outcome.session_generation > gen_before);
    assert_eq!(tasks.registered_count(), 0);

    assert_eq!(
        fs::read(paths.state_dir().join("state.db")).unwrap(),
        b"state-blob"
    );
    assert!(paths.crypto_dir().join("crypto.db").is_file());
    assert!(session_vault
        .get(&SessionMaterialId::from_identity(&alice()))
        .unwrap()
        .is_none());
    assert_eq!(
        metrics.snapshot().lifecycle.state,
        SupervisorState::LoggedOut.as_str()
    );

    let _ = fs::remove_dir_all(&root);
}

#[tokio::test]
async fn local_wipe_coordinates_supervisor_tasks_and_exact_paths() {
    let root = temp_root("full-wipe");
    let alice_paths = seed_account_store(&root, &alice());
    let bob_paths = seed_account_store(&root, &bob());

    let mut supervisor = MatrixSupervisor::new();
    let factory = TestClientFactory::new();
    harness_login_ready(&mut supervisor, &factory).unwrap();
    let gen_ready = supervisor.session_generation();

    let mut tasks = TaskSupervisor::new();
    tasks.set_live_generation(gen_ready);
    let _id = tasks
        .spawn(TaskKind::Listener, gen_ready, async {
            std::future::pending::<()>().await;
        })
        .unwrap();

    let session_vault = InMemorySessionMaterialVault::new();
    session_vault
        .set(
            &SessionMaterialId::from_identity(&alice()),
            &SessionMaterial::from_placeholder(b"sess"),
        )
        .unwrap();

    let store_vault = InMemoryStoreKeyVault::new();
    let store_key_id = StoreKeyId::from_identity(&alice());
    let _key = get_or_create_store_key(&store_vault, &store_key_id).unwrap();
    assert!(store_vault.get(&store_key_id).unwrap().is_some());

    let target = WipeTarget::resolve(&root, alice()).unwrap();
    let mut metrics = MatrixMetrics::new();

    let outcome = perform_local_wipe(
        &mut supervisor,
        &mut tasks,
        &target,
        &store_vault,
        &session_vault,
        Some(&mut metrics),
    )
    .await
    .unwrap();

    assert_eq!(supervisor.state(), SupervisorState::Empty);
    assert!(!supervisor.has_client());
    assert!(outcome.wipe.store_key_removed);
    assert!(outcome.session_material_cleared);
    assert!(!alice_paths.account_root().exists());
    assert!(bob_paths.account_root().is_dir());
    assert!(store_vault.get(&store_key_id).unwrap().is_none());
    assert_eq!(tasks.registered_count(), 0);
    assert_eq!(metrics.snapshot().store.status, StoreHealthStatus::Missing);

    let _ = fs::remove_dir_all(&root);
}

#[tokio::test]
async fn logout_then_explicit_wipe_is_distinct_path() {
    let root = temp_root("logout-then-wipe");
    let paths = seed_account_store(&root, &alice());

    let mut supervisor = MatrixSupervisor::new();
    let factory = TestClientFactory::new();
    harness_login_ready(&mut supervisor, &factory).unwrap();

    let mut tasks = TaskSupervisor::new();
    tasks.set_live_generation(supervisor.session_generation());

    let session_vault = InMemorySessionMaterialVault::new();
    session_vault
        .set(
            &SessionMaterialId::from_identity(&alice()),
            &SessionMaterial::from_placeholder(b"sess"),
        )
        .unwrap();
    let store_vault = InMemoryStoreKeyVault::new();

    perform_logout(&mut supervisor, &mut tasks, &session_vault, &alice(), None)
        .await
        .unwrap();
    assert_eq!(supervisor.state(), SupervisorState::LoggedOut);
    assert!(paths.state_dir().join("state.db").is_file());

    let target = WipeTarget::resolve(&root, alice()).unwrap();
    perform_local_wipe(
        &mut supervisor,
        &mut tasks,
        &target,
        &store_vault,
        &session_vault,
        None,
    )
    .await
    .unwrap();

    assert_eq!(supervisor.state(), SupervisorState::Empty);
    assert!(!paths.account_root().exists());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn store_failure_does_not_call_wipe_even_when_paths_exist() {
    let root = temp_root("failure-no-wipe-api");
    let paths = seed_account_store(&root, &alice());
    let before = fs::read(paths.crypto_dir().join("crypto.db")).unwrap();

    let mut metrics = MatrixMetrics::new();
    let action = surface_store_corrupt(Some(&mut metrics), None).unwrap();
    assert!(!action.requests_wipe());
    assert_eq!(action.category, MatrixIpcErrorCategory::StoreCorrupt);
    assert_eq!(
        fs::read(paths.crypto_dir().join("crypto.db")).unwrap(),
        before
    );

    let target = WipeTarget::resolve(&root, alice()).unwrap();
    let vault = InMemoryStoreKeyVault::new();
    wipe_account_store(&target, Some(&vault)).unwrap();
    assert!(!paths.account_root().exists());

    let _ = fs::remove_dir_all(&root);
}

#[tokio::test]
async fn wipe_refused_from_empty_supervisor_state() {
    let mut supervisor = MatrixSupervisor::new();
    let mut tasks = TaskSupervisor::new();
    let session_vault = InMemorySessionMaterialVault::new();
    let store_vault = InMemoryStoreKeyVault::new();

    let root = temp_root("empty-wipe");
    let target = WipeTarget::resolve(&root, alice()).unwrap();
    let err = perform_local_wipe(
        &mut supervisor,
        &mut tasks,
        &target,
        &store_vault,
        &session_vault,
        None,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, LifecycleError::Supervisor { .. }));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn recovery_action_categories_match_ipc() {
    assert_eq!(
        StoreFailureKind::Corrupt.ipc_category(),
        MatrixIpcErrorCategory::StoreCorrupt
    );
    assert_eq!(
        StoreFailureKind::Unavailable.ipc_category(),
        MatrixIpcErrorCategory::StoreUnavailable
    );
    assert_eq!(
        StoreFailureKind::Locked.ipc_category(),
        MatrixIpcErrorCategory::StoreLocked
    );
}

/// R0.5 / REV-001: BeginWipe drops the client *before* disk deletion, and
/// generation-stamped tasks are cancelled before the account root is removed.
#[tokio::test]
async fn wipe_quiesces_client_and_tasks_before_store_deletion() {
    let root = temp_root("wipe-order");
    let alice_paths = seed_account_store(&root, &alice());

    let mut supervisor = MatrixSupervisor::new();
    let factory = TestClientFactory::new();
    harness_login_ready(&mut supervisor, &factory).unwrap();
    let gen_ready = supervisor.session_generation();
    assert!(supervisor.has_client());

    let mut tasks = TaskSupervisor::new();
    tasks.set_live_generation(gen_ready);
    let _id = tasks
        .spawn(TaskKind::Sync, gen_ready, async {
            std::future::pending::<()>().await;
        })
        .unwrap();
    assert_eq!(tasks.running_count(), 1);

    // Enter Wiping without calling full wipe yet — client must drop immediately.
    supervisor
        .apply(crate::matrix::supervisor::SupervisorCommand::BeginWipe)
        .unwrap();
    assert_eq!(supervisor.state(), SupervisorState::Wiping);
    assert!(
        !supervisor.has_client(),
        "client must be dropped at BeginWipe before store deletion"
    );
    // Account store still present until wipe_account_store runs.
    assert!(alice_paths.account_root().is_dir());

    let session_vault = InMemorySessionMaterialVault::new();
    session_vault
        .set(
            &SessionMaterialId::from_identity(&alice()),
            &SessionMaterial::from_placeholder(b"sess"),
        )
        .unwrap();
    let store_vault = InMemoryStoreKeyVault::new();
    let _ = get_or_create_store_key(
        &store_vault,
        &StoreKeyId::from_identity(&alice()),
    )
    .unwrap();

    let target = WipeTarget::resolve(&root, alice()).unwrap();
    let outcome = perform_local_wipe(
        &mut supervisor,
        &mut tasks,
        &target,
        &store_vault,
        &session_vault,
        None,
    )
    .await
    .unwrap();

    assert_eq!(supervisor.state(), SupervisorState::Empty);
    assert!(!supervisor.has_client());
    assert!(!alice_paths.account_root().exists());
    assert_eq!(tasks.registered_count(), 0);
    assert!(outcome.tasks_retired >= 1);
    assert!(outcome.session_generation > gen_ready);

    let _ = fs::remove_dir_all(&root);
}

/// R0.5: session-vault failure during wipe must not leave tasks running against
/// a deleted store. We clear vault before disk wipe, so a failing vault aborts
/// with client already dropped and store still present.
#[tokio::test]
async fn wipe_session_vault_failure_fails_supervisor_without_deleting_store() {
    struct FailingSessionVault;
    impl SessionMaterialVault for FailingSessionVault {
        fn get(
            &self,
            _id: &SessionMaterialId,
        ) -> Result<Option<SessionMaterial>, LifecycleError> {
            Ok(None)
        }
        fn set(
            &self,
            _id: &SessionMaterialId,
            _material: &SessionMaterial,
        ) -> Result<(), LifecycleError> {
            Ok(())
        }
        fn clear(&self, _id: &SessionMaterialId) -> Result<bool, LifecycleError> {
            Err(LifecycleError::Vault {
                diagnostic_id: "r0.5-test-session-vault-fail",
                category: MatrixIpcErrorCategory::StoreUnavailable,
            })
        }
    }

    let root = temp_root("wipe-vault-fail");
    let paths = seed_account_store(&root, &alice());

    let mut supervisor = MatrixSupervisor::new();
    let factory = TestClientFactory::new();
    harness_login_ready(&mut supervisor, &factory).unwrap();

    let mut tasks = TaskSupervisor::new();
    tasks.set_live_generation(supervisor.session_generation());
    let _id = tasks
        .spawn(TaskKind::Listener, supervisor.session_generation(), async {
            std::future::pending::<()>().await;
        })
        .unwrap();

    let store_vault = InMemoryStoreKeyVault::new();
    let target = WipeTarget::resolve(&root, alice()).unwrap();
    let err = perform_local_wipe(
        &mut supervisor,
        &mut tasks,
        &target,
        &store_vault,
        &FailingSessionVault,
        None,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, LifecycleError::Vault { .. }));
    assert_eq!(supervisor.state(), SupervisorState::Failed);
    assert!(!supervisor.has_client());
    // Store must still exist — vault failed before wipe_account_store.
    assert!(paths.account_root().is_dir());
    assert_eq!(fs::read(paths.state_dir().join("state.db")).unwrap(), b"state-blob");
    // Tasks for pre-wipe generation were retired before vault clear.
    assert_eq!(tasks.running_count(), 0);

    let _ = fs::remove_dir_all(&root);
}
