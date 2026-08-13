//! Light integration between [`TaskSupervisor`] and
//! [`crate::app::supervisor::MatrixSupervisor`].
//!
//! The lifecycle actor stays pure/synchronous (P2.1). Task cancellation is
//! async, so composition is explicit: after a generation-bumping command,
//! call [`follow_supervisor_generation`] to cancel and await stale work.
//!
//! Supervisor now lives in Core, so this adapter lives next to the task
//! registry. Behavior is identical to the former src-tauri bridge.

use crate::app::supervisor::MatrixSupervisor;

use super::TaskSupervisor;

/// Align task live generation with the Matrix supervisor and retire stale tasks.
///
/// Intended call sites (later product wiring, not production cutover yet):
/// - after `BeginOpen` / `CompleteLogout` / `CompleteWipe` (generation bumps)
/// - after `Fail` + reopen when `BeginOpen` advances the epoch
///
/// Returns how many tasks were cancelled and joined.
pub async fn follow_supervisor_generation(
    tasks: &mut TaskSupervisor,
    supervisor: &MatrixSupervisor,
) -> usize {
    let target = supervisor.session_generation();
    if tasks.live_generation() == target {
        return 0;
    }
    tasks.set_live_generation(target);
    tasks.retire_stale().await
}

/// Snapshot supervisor generation into the task supervisor **without** retiring
/// tasks (tests that set up multi-gen fixtures). Prefer
/// [`follow_supervisor_generation`] in lifecycle paths.
pub fn mirror_generation(tasks: &mut TaskSupervisor, supervisor: &MatrixSupervisor) {
    tasks.set_live_generation(supervisor.session_generation());
}
