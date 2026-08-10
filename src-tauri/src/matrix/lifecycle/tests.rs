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
    assert_eq!(report.wipe_target_kind, WIPE_TARGET_KIND_ACCOUNT_ROOT);
    // R0.6 / REV-003: wipe report must not embed absolute paths.
    let report_dbg = format!("{report:?}");
    assert!(!report_dbg.contains(root.to_string_lossy().as_ref()));
    assert!(!report_dbg.contains(alice_paths.account_root().to_string_lossy().as_ref()));
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
    let _ = get_or_create_store_key(&store_vault, &StoreKeyId::from_identity(&alice())).unwrap();

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
        fn get(&self, _id: &SessionMaterialId) -> Result<Option<SessionMaterial>, LifecycleError> {
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
    assert_eq!(
        fs::read(paths.state_dir().join("state.db")).unwrap(),
        b"state-blob"
    );
    // Tasks for pre-wipe generation were retired before vault clear.
    assert_eq!(tasks.running_count(), 0);

    let _ = fs::remove_dir_all(&root);
}

// --- R0.7 slice 3: composed encrypted-store + supervisor lifecycle (real SDK) ---

/// Identity for R0.7 composed residual. Loopback-shaped homeserver URL (no
/// live network required for unauthenticated Client open). Optional live
/// disposable-Synapse URL via `SYNARA_MATRIX_HOMESERVER_URL` when gated.
fn r0_7_identity() -> AccountIdentity {
    let hs = std::env::var("SYNARA_MATRIX_HOMESERVER_URL")
        .ok()
        .filter(|u| {
            std::env::var("SYNARA_RUN_MATRIX_RUST_AUTH_LIVE")
                .ok()
                .as_deref()
                == Some("1")
                && {
                    let Ok(parsed) = url::Url::parse(u) else {
                        return false;
                    };
                    parsed.scheme() == "http"
                        && matches!(
                            parsed.host_str(),
                            Some("127.0.0.1") | Some("localhost") | Some("::1")
                        )
                        && parsed.username().is_empty()
                        && parsed.password().is_none()
                }
        })
        .unwrap_or_else(|| "http://127.0.0.1:8008".to_owned());
    AccountIdentity::new("@r07-lifecycle:localhost", &hs).unwrap()
}

/// Factory that opens a **real** encrypted SQLite Client via P2.3 builder and
/// wraps it in [`SdkClientHandle`]. Reuses a fixed store key so reopen works.
struct R07SdkFactory {
    root: PathBuf,
    identity: AccountIdentity,
    key_bytes: [u8; 32],
    next_id: AtomicUsize,
}

impl crate::matrix::supervisor::ClientFactory for R07SdkFactory {
    fn build(
        &self,
        _generation: u64,
    ) -> Result<
        Box<dyn crate::matrix::supervisor::ClientHandle>,
        crate::matrix::supervisor::FactoryError,
    > {
        use crate::matrix::client_builder::{build_unauthenticated_client, ClientBuildConfig};
        use crate::matrix::store::StoreKeyMaterial;

        let key = StoreKeyMaterial::from_bytes(self.key_bytes);
        let cfg = ClientBuildConfig::product_default(&self.root, self.identity.clone(), Some(key))
            .map_err(|e| e.to_factory_error())?;
        // Caller must hold a multi-thread Tokio runtime enter guard.
        let client = tokio::runtime::Handle::current()
            .block_on(build_unauthenticated_client(&cfg))
            .map_err(|e| e.to_factory_error())?;
        assert!(
            client.session().is_none(),
            "R0.7 residual must not install a production session (login is P3.2)"
        );
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) as u64 + 1;
        Ok(Box::new(
            crate::matrix::client_builder::SdkClientHandle::new(id, client),
        ))
    }
}

/// R0.7 slice 3 — always-on composed residual:
/// encrypted store open (real SDK) → Ready (sync-ready state) → logout
/// (stores retained) → reopen same key → local wipe (exact account gone).
///
/// Does **not** call banned production login/sync APIs. Live disposable-Synapse
/// authenticated sync remains an explicit later residual / P3.2 path.
#[test]
fn r0_7_encrypted_store_open_ready_logout_reopen_wipe() {
    use crate::matrix::client_builder::{build_unauthenticated_client, ClientBuildConfig};
    use crate::matrix::store::StoreKeyMaterial;
    use crate::matrix::supervisor::SupervisorCommand;

    let root = temp_root("r07-composed");
    let identity = r0_7_identity();
    let key_bytes = *StoreKeyMaterial::generate().unwrap().as_bytes();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("multi-thread runtime for SDK Client drop");
    let _enter = rt.enter();

    // --- open encrypted store (first process) ---
    let key1 = StoreKeyMaterial::from_bytes(key_bytes);
    let cfg1 = ClientBuildConfig::product_default(&root, identity.clone(), Some(key1)).unwrap();
    let c1 = rt
        .block_on(build_unauthenticated_client(&cfg1))
        .expect("first encrypted open");
    assert!(c1.session().is_none());
    assert!(cfg1.state_store_path().is_dir());
    drop(c1);

    // --- install real SDK handle → sync-ready state machine ---
    let factory = R07SdkFactory {
        root: root.clone(),
        identity: identity.clone(),
        key_bytes,
        next_id: AtomicUsize::new(0),
    };
    let mut supervisor = MatrixSupervisor::new();
    supervisor
        .apply(SupervisorCommand::BeginOpen)
        .expect("begin open");
    supervisor
        .apply(SupervisorCommand::BeginAuthenticate)
        .expect("auth");
    supervisor
        .apply_with_factory(SupervisorCommand::InstallClient, &factory)
        .expect("install sdk client");
    assert!(supervisor.has_client());
    // "Sync readiness" without starting a live dual-sync loop: mark Ready.
    supervisor
        .apply(SupervisorCommand::BeginSync)
        .expect("begin sync");
    supervisor
        .apply(SupervisorCommand::MarkReady)
        .expect("mark ready");
    assert_eq!(supervisor.state(), SupervisorState::Ready);
    let gen_ready = supervisor.session_generation();
    assert!(gen_ready >= 1);

    let mut tasks = TaskSupervisor::new();
    tasks.set_live_generation(gen_ready);
    let _task = tasks
        .spawn(TaskKind::Sync, gen_ready, async {
            std::future::pending::<()>().await;
        })
        .expect("spawn generation-stamped task");

    let session_vault = InMemorySessionMaterialVault::new();
    session_vault
        .set(
            &SessionMaterialId::from_identity(&identity),
            &SessionMaterial::from_placeholder(b"r07-sess"),
        )
        .unwrap();

    // --- logout: drop client, retire tasks, retain stores ---
    let logout = rt
        .block_on(perform_logout(
            &mut supervisor,
            &mut tasks,
            &session_vault,
            &identity,
            None,
        ))
        .expect("logout");
    assert_eq!(supervisor.state(), SupervisorState::LoggedOut);
    assert!(!supervisor.has_client());
    assert!(logout.stores_retained);
    assert!(logout.session_material_cleared);
    assert!(
        cfg1.state_store_path().is_dir(),
        "logout must retain stores"
    );
    assert_eq!(tasks.registered_count(), 0);

    // --- crash/restart simulation: reopen encrypted store with same key ---
    let key2 = StoreKeyMaterial::from_bytes(key_bytes);
    let cfg2 = ClientBuildConfig::product_default(&root, identity.clone(), Some(key2)).unwrap();
    let c2 = rt
        .block_on(build_unauthenticated_client(&cfg2))
        .expect("reopen encrypted store after logout");
    assert!(c2.session().is_none());
    drop(c2);

    // Re-install and reach Ready again (post-restart path).
    supervisor
        .apply(SupervisorCommand::BeginOpen)
        .expect("reopen begin");
    supervisor
        .apply(SupervisorCommand::BeginAuthenticate)
        .expect("reopen auth");
    supervisor
        .apply_with_factory(SupervisorCommand::InstallClient, &factory)
        .expect("reinstall sdk client");
    supervisor
        .apply(SupervisorCommand::BeginSync)
        .expect("reopen sync");
    supervisor
        .apply(SupervisorCommand::MarkReady)
        .expect("reopen ready");
    assert_eq!(supervisor.state(), SupervisorState::Ready);
    assert!(supervisor.has_client());
    let gen_after_reopen = supervisor.session_generation();
    assert!(gen_after_reopen > gen_ready);

    // --- local wipe: exact account root removed ---
    tasks.set_live_generation(gen_after_reopen);
    let store_vault = InMemoryStoreKeyVault::new();
    let store_key_id = StoreKeyId::from_identity(&identity);
    let _ = get_or_create_store_key(&store_vault, &store_key_id).unwrap();
    let target = WipeTarget::resolve(&root, identity.clone()).unwrap();
    let account_root = target.account_root().to_path_buf();
    assert!(account_root.is_dir());

    let wipe = rt
        .block_on(perform_local_wipe(
            &mut supervisor,
            &mut tasks,
            &target,
            &store_vault,
            &session_vault,
            None,
        ))
        .expect("local wipe");
    assert_eq!(supervisor.state(), SupervisorState::Empty);
    assert!(!supervisor.has_client());
    assert!(wipe.wipe.account_root_removed);
    assert!(wipe.wipe.store_key_removed);
    assert!(
        !account_root.exists(),
        "wipe must remove exact account root"
    );
    assert!(wipe.session_generation > gen_after_reopen);

    drop(supervisor);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}

/// R0.7 slice 4 — always-on residual: real SDK install → Ready → generation-
/// stamped task → logout retires work → stale generation refused → reinstall
/// at new generation succeeds.
///
/// Complements unit-level task isolation tests by composing them with a real
/// `SdkClientHandle` and the logout barrier (R0.5 / R0.7 failure + stale cases).
#[test]
fn r0_7_stale_generation_after_real_sdk_logout() {
    use crate::matrix::client_builder::{build_unauthenticated_client, ClientBuildConfig};
    use crate::matrix::store::StoreKeyMaterial;
    use crate::matrix::supervisor::SupervisorCommand;

    let root = temp_root("r07-stale-gen");
    let identity = r0_7_identity();
    let key_bytes = *StoreKeyMaterial::generate().unwrap().as_bytes();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("multi-thread runtime for SDK Client drop");
    let _enter = rt.enter();

    let key = StoreKeyMaterial::from_bytes(key_bytes);
    let cfg = ClientBuildConfig::product_default(&root, identity.clone(), Some(key)).unwrap();
    let client = rt
        .block_on(build_unauthenticated_client(&cfg))
        .expect("encrypted open");
    assert!(client.session().is_none());
    drop(client);

    let factory = R07SdkFactory {
        root: root.clone(),
        identity: identity.clone(),
        key_bytes,
        next_id: AtomicUsize::new(0),
    };
    let mut supervisor = MatrixSupervisor::new();
    supervisor
        .apply(SupervisorCommand::BeginOpen)
        .expect("begin open");
    supervisor
        .apply(SupervisorCommand::BeginAuthenticate)
        .expect("auth");
    supervisor
        .apply_with_factory(SupervisorCommand::InstallClient, &factory)
        .expect("install sdk client");
    supervisor
        .apply(SupervisorCommand::BeginSync)
        .expect("begin sync");
    supervisor
        .apply(SupervisorCommand::MarkReady)
        .expect("mark ready");
    let gen_ready = supervisor.session_generation();
    assert!(gen_ready >= 1);

    let mut tasks = TaskSupervisor::new();
    tasks.set_live_generation(gen_ready);
    let stale_task = tasks
        .spawn(TaskKind::Sync, gen_ready, async {
            std::future::pending::<()>().await;
        })
        .expect("spawn at live generation");

    let session_vault = InMemorySessionMaterialVault::new();
    session_vault
        .set(
            &SessionMaterialId::from_identity(&identity),
            &SessionMaterial::from_placeholder(b"r07-stale"),
        )
        .unwrap();

    let logout = rt
        .block_on(perform_logout(
            &mut supervisor,
            &mut tasks,
            &session_vault,
            &identity,
            None,
        ))
        .expect("logout");
    assert_eq!(supervisor.state(), SupervisorState::LoggedOut);
    assert!(!supervisor.has_client());
    assert!(logout.tasks_retired >= 1);
    assert_eq!(tasks.registered_count(), 0);
    assert!(tasks.get(stale_task).is_none());

    let gen_after = supervisor.session_generation();
    assert!(
        gen_after > gen_ready,
        "logout must advance session generation"
    );
    // Task supervisor follows supervisor generation after logout.
    assert_eq!(tasks.live_generation(), gen_after);

    // Stale generation refused for both result acceptance and new work.
    assert!(tasks
        .accept_result(gen_ready)
        .unwrap_err()
        .is_stale_generation());
    assert!(tasks
        .spawn(TaskKind::Generic, gen_ready, async {})
        .unwrap_err()
        .is_stale_generation());

    // Live generation may spawn after reinstall.
    supervisor
        .apply(SupervisorCommand::BeginOpen)
        .expect("reopen begin");
    supervisor
        .apply(SupervisorCommand::BeginAuthenticate)
        .expect("reopen auth");
    supervisor
        .apply_with_factory(SupervisorCommand::InstallClient, &factory)
        .expect("reinstall sdk client");
    supervisor
        .apply(SupervisorCommand::BeginSync)
        .expect("reopen sync");
    supervisor
        .apply(SupervisorCommand::MarkReady)
        .expect("reopen ready");
    assert_eq!(supervisor.state(), SupervisorState::Ready);
    assert!(supervisor.has_client());
    let gen_reopen = supervisor.session_generation();
    assert!(gen_reopen >= gen_after);
    tasks.set_live_generation(gen_reopen);
    let live_task = tasks
        .spawn(TaskKind::Generic, gen_reopen, async {})
        .expect("spawn at post-logout live generation");
    assert!(tasks.accept_task_result(live_task).is_ok());

    drop(supervisor);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}

/// R0.7 slice 4 — always-on residual: wrong store-key reopen fails with a
/// privacy-safe builder error (no absolute paths, no key material, no raw SDK
/// dump). Complements happy-path reopen in slice 3.
#[test]
fn r0_7_wrong_store_key_reopen_fails_privately() {
    use crate::matrix::client_builder::{
        build_unauthenticated_client, ClientBuildConfig, ClientBuilderError,
    };
    use crate::matrix::store::StoreKeyMaterial;

    let root = temp_root("r07-wrong-key");
    let identity = r0_7_identity();
    let key_a = StoreKeyMaterial::generate().unwrap();
    let key_b = StoreKeyMaterial::generate().unwrap();
    assert!(!key_a.equals(&key_b));
    let key_a_hex = key_a
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let key_b_hex = key_b
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let root_display = root.display().to_string();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("multi-thread runtime for SDK Client drop");
    let _enter = rt.enter();

    let cfg_ok = ClientBuildConfig::product_default(&root, identity.clone(), Some(key_a)).unwrap();
    let client = rt
        .block_on(build_unauthenticated_client(&cfg_ok))
        .expect("first encrypted open with key A");
    assert!(client.session().is_none());
    drop(client);

    let cfg_bad = ClientBuildConfig::product_default(&root, identity.clone(), Some(key_b)).unwrap();
    let err = rt
        .block_on(build_unauthenticated_client(&cfg_bad))
        .expect_err("wrong store key must fail reopen");

    match &err {
        ClientBuilderError::SdkBuild {
            diagnostic_id,
            message,
            category: _,
        } => {
            assert!(
                diagnostic_id.starts_with("p2.3-"),
                "expected redacted p2.3 diagnostic id, got {diagnostic_id}"
            );
            assert!(!message.is_empty(), "safe message must be non-empty");
            // Privacy: Display surface must not leak paths or key material.
            let surface = err.to_string();
            assert!(
                !surface.contains(&root_display),
                "error must not contain absolute store root"
            );
            assert!(
                !surface.contains(&key_a_hex),
                "error must not contain key A"
            );
            assert!(
                !surface.contains(&key_b_hex),
                "error must not contain key B"
            );
            assert!(
                !surface.contains("sqlite"),
                "error must not leak raw engine detail: {surface}"
            );
        }
        other => panic!("expected SdkBuild error, got {other:?}"),
    }

    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}

// --- P3.8 remote logout + recovery copy ---

#[test]
fn p3_8_marker_includes_remote_logout() {
    assert_eq!(matrix_lifecycle_markers(), MATRIX_LIFECYCLE_MARKER);
    assert!(MATRIX_LIFECYCLE_MARKER.contains("p3.8"));
}

#[test]
fn p3_8_remote_this_device_then_local_cleanup() {
    let mut flow = RemoteLogoutFlow::new(4);
    flow.begin(RemoteLogoutScope::ThisDevice).unwrap();
    assert_eq!(flow.phase(), RemoteLogoutPhase::RequestingRemote);
    assert!(flow.is_busy());
    flow.complete_remote().unwrap();
    assert_eq!(flow.phase(), RemoteLogoutPhase::LocalCleanupPending);
    let out = flow.complete_local_cleanup().unwrap();
    assert!(out.remote_succeeded);
    assert!(!out.remote_skipped);
    assert!(out.local_cleanup_applied);
    assert_eq!(out.session_generation, 4);
    assert_eq!(flow.phase(), RemoteLogoutPhase::Complete);
    let key = copy_for_remote_outcome(
        out.remote_succeeded,
        out.remote_skipped,
        out.scope,
        out.local_policy,
    );
    assert_eq!(key, RecoveryCopyKey::RemoteLogoutThisDeviceOk);
}

#[test]
fn p3_8_skip_remote_offline_then_wipe_policy() {
    let mut flow = RemoteLogoutFlow::new(1);
    flow.set_local_policy(LocalCleanupPolicy::WipeAccountStore)
        .unwrap();
    flow.begin(RemoteLogoutScope::ThisDevice).unwrap();
    flow.skip_remote("p3.8-homeserver-unreachable").unwrap();
    assert_eq!(flow.phase(), RemoteLogoutPhase::LocalCleanupPending);
    let out = flow.complete_local_cleanup().unwrap();
    assert!(out.remote_skipped);
    assert!(!out.remote_succeeded);
    assert_eq!(out.local_policy, LocalCleanupPolicy::WipeAccountStore);
    let key = copy_for_remote_outcome(
        out.remote_succeeded,
        out.remote_skipped,
        out.scope,
        out.local_policy,
    );
    assert_eq!(key, RecoveryCopyKey::RemoteLogoutSkippedOffline);
}

#[test]
fn p3_8_fail_remote_forbids_secret_diagnostics() {
    let mut flow = RemoteLogoutFlow::new(1);
    flow.begin(RemoteLogoutScope::AllDevices).unwrap();
    let err = flow.fail_remote("leaked-access_token").unwrap_err();
    match err {
        LifecycleError::InvalidTarget { diagnostic_id } => {
            assert_eq!(diagnostic_id, "p3.8-forbidden-diagnostic");
        }
        other => panic!("unexpected {other:?}"),
    }
    flow.fail_remote("p3.8-server-rejected").unwrap();
    assert_eq!(flow.phase(), RemoteLogoutPhase::Failed);
    flow.clear_failure().unwrap();
    flow.begin(RemoteLogoutScope::AllDevices).unwrap();
    assert_eq!(flow.attempts(), 2);
}

#[test]
fn p3_8_busy_and_skip_disallowed() {
    let mut flow = RemoteLogoutFlow::new(1);
    flow.set_allow_skip_remote(false);
    flow.begin(RemoteLogoutScope::ThisDevice).unwrap();
    let err = flow.begin(RemoteLogoutScope::ThisDevice).unwrap_err();
    match err {
        LifecycleError::InvalidTarget { diagnostic_id } => {
            assert_eq!(diagnostic_id, "p3.8-remote-logout-busy");
        }
        other => panic!("unexpected {other:?}"),
    }
    let err = flow.skip_remote("p3.8-offline").unwrap_err();
    match err {
        LifecycleError::InvalidTarget { diagnostic_id } => {
            assert_eq!(diagnostic_id, "p3.8-skip-remote-disallowed");
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn p3_8_retire_generation_cancels_in_flight() {
    let mut flow = RemoteLogoutFlow::new(1);
    flow.begin(RemoteLogoutScope::ThisDevice).unwrap();
    flow.retire_generation(9);
    assert_eq!(flow.session_generation(), 9);
    assert_eq!(flow.phase(), RemoteLogoutPhase::Failed);
    assert_eq!(
        flow.failure_diagnostic_id(),
        Some("p3.8-stale-generation-cancelled")
    );
}

#[test]
fn p3_8_recovery_copy_keys_stable_and_safe() {
    assert_eq!(RecoveryCopyKey::ALL.len(), 9);
    for key in RecoveryCopyKey::ALL {
        let s = recovery_copy_en(*key);
        assert!(!s.is_empty());
        assert_eq!(s, key.default_en());
        let id = key.as_str();
        assert!(!id.is_empty());
        assert!(!id.contains(' '));
        assert!(!s.to_ascii_lowercase().contains("access_token"));
    }
}
