//! Light integration between [`crate::matrix::tasks::TaskSupervisor`] (hosted
//! in `synara_core::task`, SNC-P1-4) and
//! [`crate::matrix::supervisor::MatrixSupervisor`].
//!
//! The lifecycle actor stays pure/synchronous (P2.1). Task cancellation is
//! async, so composition is explicit: after a generation-bumping command,
//! call [`follow_supervisor_generation`] to cancel and await stale work.
//!
//! SNC-P1-4: this adapter stays in src-tauri because the supervisor module is
//! still desktop-local (it moves with the P1.5 "rest" chunk). It consumes the
//! core registry types and keeps the original follow/mirror behavior identical.

use crate::matrix::supervisor::MatrixSupervisor;

use synara_core::task::TaskSupervisor;

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
