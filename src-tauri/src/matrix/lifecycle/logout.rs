//! Logout and local-wipe orchestration (`D-LOGOUT-WIPE`, plan §8.3).
//!
//! - **Logout**: drop client handle, retire generation-stamped tasks, clear
//!   session material — **stores remain intact**.
//! - **Local wipe**: exact-target store destruction via [`WipeTarget`], clear
//!   session material + store key, complete wipe epoch.
//!
//! Remote/server logout is **P3.8**. No production Tauri commands here.

use crate::matrix::diagnostics::MatrixMetrics;
use crate::matrix::store::StoreKeyVault;
use crate::matrix::supervisor::{
    MatrixSupervisor, SupervisorCommand, SupervisorError, SupervisorState,
};
use crate::matrix::tasks::{follow_supervisor_generation, TaskSupervisor};

use super::error::LifecycleError;
use super::session_material::{clear_session_material, SessionMaterialVault};
use super::wipe::{wipe_account_store, WipeReport, WipeTarget};

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

/// Perform logout: supervisor stop/logout, task retire, session material clear.
///
/// Does **not** delete store directories or store encryption keys.
pub async fn perform_logout<S>(
    supervisor: &mut MatrixSupervisor,
    tasks: &mut TaskSupervisor,
    session_vault: &S,
    identity: &crate::matrix::store::AccountIdentity,
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

    if supervisor.state() == SupervisorState::Stopping {
        apply_cmd(supervisor, SupervisorCommand::CompleteLogout)?;
    }

    let tasks_retired = follow_supervisor_generation(tasks, supervisor).await;
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

/// Perform exact-target local wipe: BeginWipe → disk wipe → CompleteWipe.
///
/// Wipe I/O runs only after `BeginWipe`. On I/O failure the actor moves to
/// `Failed` without completing wipe (no further auto-delete).
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

    let wipe = match wipe_account_store(target, Some(key_vault)) {
        Ok(r) => r,
        Err(e) => {
            // Fail from Wiping is legal (P2.6): do not CompleteWipe on I/O
            // failure, and never auto-retry delete.
            let _ = supervisor.fail(
                crate::matrix::ipc::MatrixIpcErrorCategory::StoreUnavailable,
                "p2.6-wipe-io-failed",
            );
            return Err(e);
        }
    };

    let session_material_cleared = clear_session_material(session_vault, target.identity())?;

    apply_cmd(supervisor, SupervisorCommand::CompleteWipe)?;
    let tasks_retired = follow_supervisor_generation(tasks, supervisor).await;

    if let Some(m) = metrics {
        m.observe_supervisor(supervisor);
        m.observe_tasks(tasks);
        m.set_store_status(crate::matrix::diagnostics::StoreHealthStatus::Missing);
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
