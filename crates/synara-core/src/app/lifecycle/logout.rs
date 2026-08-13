//! Logout and local-wipe orchestration (`D-LOGOUT-WIPE`, plan §8.3).
//!
//! - **Logout**: drop client handle, retire generation-stamped tasks, clear
//!   session material — **stores remain intact**.
//! - **Local wipe**: exact-target store destruction via [`WipeTarget`], clear
//!   session material + store key, complete wipe epoch.
//!
//! Remote/server logout is **P3.8**. No production Tauri commands here.

use crate::app::diagnostics::MatrixMetrics;
use crate::app::store::StoreKeyVault;
use crate::app::supervisor::{
    MatrixSupervisor, SupervisorCommand, SupervisorError, SupervisorState,
};
use crate::task::{follow_supervisor_generation, TaskSupervisor};

use super::LifecycleError;
use super::{clear_session_material, SessionMaterialVault};
use super::{wipe_account_store, WipeReport, WipeTarget};

/// Privacy-safe logout outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogoutOutcome {
    pub session_material_cleared: bool,
    pub stores_retained: bool,
    pub session_generation: u64,
    pub state: SupervisorState,
    pub tasks_retired: usize,
}

/// Privacy-safe local wipe outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WipeOutcome {
    pub wipe: WipeReport,
    pub session_material_cleared: bool,
    pub session_generation: u64,
    pub state: SupervisorState,
    pub tasks_retired: usize,
}

/// Perform logout: stop new work → cancel/join tasks → drop client → clear session material.
///
/// Does **not** delete store directories or store encryption keys.
pub async fn perform_logout<S>(
    supervisor: &mut MatrixSupervisor,
    tasks: &mut TaskSupervisor,
    session_vault: &S,
    identity: &crate::app::store::AccountIdentity,
    metrics: Option<&mut MatrixMetrics>,
) -> Result<LogoutOutcome, LifecycleError>
where
    S: SessionMaterialVault + ?Sized,
{
    match supervisor.state() {
        SupervisorState::Stopping => {}
        SupervisorState::LoggedOut | SupervisorState::Empty | SupervisorState::Failed => {
            // Already terminal / idle — still clear session material.
        }
        _ => {
            apply_cmd(supervisor, SupervisorCommand::BeginStop)?;
        }
    }

    // Cancel and join work *before* CompleteLogout drops the client handle.
    let gen_before = supervisor.session_generation();
    let mut tasks_retired = if matches!(
        supervisor.state(),
        SupervisorState::Stopping | SupervisorState::Ready | SupervisorState::Syncing
    ) {
        tasks.retire_generation(gen_before).await
    } else {
        0
    };

    if supervisor.state() == SupervisorState::Stopping {
        apply_cmd(supervisor, SupervisorCommand::CompleteLogout)?;
    }

    tasks_retired += follow_supervisor_generation(tasks, supervisor).await;
    let session_material_cleared = clear_session_material(session_vault, identity)?;

    if let Some(m) = metrics {
        m.observe_supervisor(supervisor);
        m.observe_tasks(tasks);
    }

    Ok(LogoutOutcome {
        session_material_cleared,
        stores_retained: true,
        session_generation: supervisor.session_generation(),
        state: supervisor.state(),
        tasks_retired,
    })
}

/// Perform exact-target local wipe with transactional ordering (R0.5 / REV-001):
///
/// 1. `BeginWipe` (rejects other work; **drops client handle**)
/// 2. Cancel and join all tasks for the pre-wipe generation
/// 3. Clear session material and remove store key + exact account store
/// 4. `CompleteWipe` (bumps generation) and align task supervisor
///
/// On I/O / vault failure the actor moves to `Failed` without `CompleteWipe`
/// and never auto-retries delete. Client and tasks are already stopped first.
pub async fn perform_local_wipe<K, S>(
    supervisor: &mut MatrixSupervisor,
    tasks: &mut TaskSupervisor,
    target: &WipeTarget,
    key_vault: &K,
    session_vault: &S,
    metrics: Option<&mut MatrixMetrics>,
) -> Result<WipeOutcome, LifecycleError>
where
    K: StoreKeyVault + ?Sized,
    S: SessionMaterialVault + ?Sized,
{
    match supervisor.state() {
        SupervisorState::Wiping => {}
        _ => {
            apply_cmd(supervisor, SupervisorCommand::BeginWipe)?;
        }
    }

    // Client must already be dropped at BeginWipe (REV-001).
    debug_assert!(
        !supervisor.has_client(),
        "BeginWipe must drop ClientHandle before store deletion"
    );

    // Quiesce async work for the generation that still owns the store.
    let gen_pre_wipe = supervisor.session_generation();
    let mut tasks_retired = tasks.retire_generation(gen_pre_wipe).await;

    // Session material first so a vault failure does not delete stores while
    // still holding secrets we meant to keep recoverable.
    let session_material_cleared = match clear_session_material(session_vault, target.identity()) {
        Ok(cleared) => cleared,
        Err(e) => {
            let _ = supervisor.fail(
                crate::transport::MatrixIpcErrorCategory::StoreUnavailable,
                "r0.5-wipe-session-vault-failed",
            );
            return Err(e);
        }
    };

    let wipe = match wipe_account_store(target, Some(key_vault)) {
        Ok(r) => r,
        Err(e) => {
            // Fail from Wiping is legal (P2.6): do not CompleteWipe on I/O
            // failure, and never auto-retry delete.
            let _ = supervisor.fail(
                crate::transport::MatrixIpcErrorCategory::StoreUnavailable,
                "r0.5-wipe-io-failed",
            );
            return Err(e);
        }
    };

    apply_cmd(supervisor, SupervisorCommand::CompleteWipe)?;
    tasks_retired += follow_supervisor_generation(tasks, supervisor).await;

    if let Some(m) = metrics {
        m.observe_supervisor(supervisor);
        m.observe_tasks(tasks);
        m.set_store_status(crate::app::diagnostics::StoreHealthStatus::Missing);
    }

    Ok(WipeOutcome {
        wipe,
        session_material_cleared,
        session_generation: supervisor.session_generation(),
        state: supervisor.state(),
        tasks_retired,
    })
}

fn apply_cmd(
    supervisor: &mut MatrixSupervisor,
    cmd: SupervisorCommand,
) -> Result<(), LifecycleError> {
    supervisor.apply(cmd).map_err(map_supervisor_err)
}

fn map_supervisor_err(err: SupervisorError) -> LifecycleError {
    LifecycleError::Supervisor {
        diagnostic_id: "p2.6-supervisor-transition",
        detail: err.to_string(),
    }
}
