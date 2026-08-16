//! Deterministic unit tests for P2.4 task supervision and cancellation (src-tauri side).
//!
//! SNC-P1-4: `super::*` resolves against the core `synara_core::task` types
//! re-exported by this module plus the desktop `bridge` adapter; the 3
//! supervisor-coupled follow/mirror tests run here (they need the desktop
//! `MatrixSupervisor`).
//!
//! No live homeserver network. Pure registry + Tokio mock futures only.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use super::*;
use crate::matrix::supervisor::{
    harness_login_ready, MatrixSupervisor, SupervisorCommand, TestClientFactory,
};

#[test]
fn marker_and_kinds_non_empty() {
    assert_eq!(matrix_tasks_markers(), MATRIX_TASKS_MARKER);
    assert_eq!(TaskKind::ALL.len(), 5);
    assert_eq!(TaskKind::Sync.as_str(), "sync");
    assert_eq!(TaskKind::Listener.as_str(), "listener");
    assert_eq!(TaskKind::Upload.as_str(), "upload");
    assert_eq!(TaskKind::Search.as_str(), "search");
    assert_eq!(TaskKind::Generic.as_str(), "generic");
}

#[test]
fn fresh_supervisor_is_idle() {
    let s = TaskSupervisor::new();
    assert_eq!(s.live_generation(), 0);
    assert_eq!(s.registered_count(), 0);
    assert_eq!(s.running_count(), 0);
    assert_eq!(s.spawned_total(), 0);
    assert_eq!(s.joined_total(), 0);
    assert!(s.accept_result(0).is_ok());
    assert!(s.accept_result(1).unwrap_err().is_stale_generation());
}

#[test]
fn register_and_list_by_kind() {
    let mut s = TaskSupervisor::new();
    s.set_live_generation(1);
    let a = s.register(TaskKind::Sync, 1).unwrap();
    let b = s.register(TaskKind::Upload, 1).unwrap();
    let c = s.register(TaskKind::Search, 1).unwrap();
    assert_ne!(a, b);
    assert_ne!(b, c);
    assert_eq!(s.registered_count(), 3);
    assert_eq!(s.count_for_kind(TaskKind::Sync), 1);
    assert_eq!(s.count_for_kind(TaskKind::Listener), 0);
    assert_eq!(s.running_count(), 3);

    let info = s.get(a).unwrap();
    assert_eq!(info.kind, TaskKind::Sync);
    assert_eq!(info.generation, 1);
    assert_eq!(info.state, TaskRunState::Running);
    assert_eq!(s.list().len(), 3);
}

#[test]
fn refuse_register_stale_generation() {
    let mut s = TaskSupervisor::new();
    s.set_live_generation(2);
    let err = s.register(TaskKind::Generic, 1).unwrap_err();
    assert!(matches!(
        err,
        TaskError::SpawnStaleGeneration {
            observed: 1,
            live: 2
        }
    ));
    assert_eq!(s.registered_count(), 0);
}

#[test]
fn double_cancel_is_idempotent_placeholder() {
    let mut s = TaskSupervisor::new();
    s.set_live_generation(1);
    let id = s.register(TaskKind::Listener, 1).unwrap();
    s.cancel(id).unwrap();
    s.cancel(id).unwrap(); // second cancel no-ops
    assert_eq!(s.cancelled_requests(), 1);
    assert_eq!(s.get(id).unwrap().state, TaskRunState::Cancelled);
}

#[tokio::test]
async fn join_placeholder_after_cancel() {
    let mut s = TaskSupervisor::new();
    s.set_live_generation(1);
    let id = s.register(TaskKind::Generic, 1).unwrap();
    s.cancel(id).unwrap();
    let outcome = s.join(id).await.unwrap();
    assert_eq!(outcome, TaskOutcome::Cancelled);
    assert_eq!(s.registered_count(), 0);
    assert_eq!(s.joined_total(), 1);
}

#[tokio::test]
async fn spawn_complete_and_join() {
    let mut s = TaskSupervisor::new();
    s.set_live_generation(1);
    let flag = Arc::new(AtomicUsize::new(0));
    let flag2 = Arc::clone(&flag);
    let id = s
        .spawn(TaskKind::Search, 1, async move {
            flag2.fetch_add(1, Ordering::SeqCst);
        })
        .unwrap();
    let outcome = s.join(id).await.unwrap();
    assert_eq!(outcome, TaskOutcome::Completed);
    assert_eq!(flag.load(Ordering::SeqCst), 1);
    assert_eq!(s.registered_count(), 0);
}

#[tokio::test]
async fn race_cancel_long_running_task() {
    let mut s = TaskSupervisor::new();
    s.set_live_generation(1);
    let started = Arc::new(AtomicUsize::new(0));
    let started2 = Arc::clone(&started);
    let id = s
        .spawn(TaskKind::Sync, 1, async move {
            started2.fetch_add(1, Ordering::SeqCst);
            // Long enough that cancel wins the race under normal scheduling.
            tokio::time::sleep(Duration::from_secs(60)).await;
        })
        .unwrap();

    // Yield so the task is likely running before cancel.
    tokio::task::yield_now().await;
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(started.load(Ordering::SeqCst), 1);

    let outcome = s.cancel_and_join(id).await.unwrap();
    assert_eq!(outcome, TaskOutcome::Cancelled);
    assert_eq!(s.registered_count(), 0);
    assert_eq!(s.running_count(), 0);
}

#[tokio::test]
async fn double_cancel_spawned_task_then_join() {
    let mut s = TaskSupervisor::new();
    s.set_live_generation(1);
    let id = s
        .spawn(TaskKind::Upload, 1, async {
            tokio::time::sleep(Duration::from_secs(60)).await;
        })
        .unwrap();
    s.cancel(id).unwrap();
    s.cancel(id).unwrap(); // double-cancel
    assert_eq!(s.cancelled_requests(), 1);
    let outcome = s.join(id).await.unwrap();
    assert_eq!(outcome, TaskOutcome::Cancelled);
}

#[tokio::test]
async fn stale_generation_isolation_refuses_results_and_spawn() {
    let mut s = TaskSupervisor::new();
    s.set_live_generation(1);
    let id = s
        .spawn(TaskKind::Listener, 1, async {
            tokio::time::sleep(Duration::from_secs(60)).await;
        })
        .unwrap();

    // Simulate logout/wipe generation bump without yet retiring.
    s.bump_generation();
    assert_eq!(s.live_generation(), 2);

    assert!(s.accept_result(1).unwrap_err().is_stale_generation());
    assert!(s.accept_task_result(id).unwrap_err().is_stale_generation());
    assert!(s.accept_result(2).is_ok());

    // New work at old generation is refused.
    let err = s.spawn(TaskKind::Generic, 1, async {}).unwrap_err();
    assert!(err.is_stale_generation());

    // Live generation may spawn.
    let id2 = s.spawn(TaskKind::Generic, 2, async {}).unwrap();
    assert!(s.accept_task_result(id2).is_ok());

    // Retire stale (gen 1) only.
    let retired = s.retire_stale().await;
    assert_eq!(retired, 1);
    assert!(s.get(id).is_none());
    assert!(s.get(id2).is_some());

    let _ = s.shutdown_all().await;
}

#[tokio::test]
async fn retire_generation_cancels_only_target_gen() {
    let mut s = TaskSupervisor::new();
    s.set_live_generation(1);
    let old = s
        .spawn(TaskKind::Sync, 1, async {
            tokio::time::sleep(Duration::from_secs(60)).await;
        })
        .unwrap();
    // Manually place a "future epoch" by bumping, spawning, then rewinding live
    // is not allowed for spawn — instead register placeholder then set gen.
    // Pattern: bump, spawn live, retire previous.
    s.bump_generation();
    let live = s
        .spawn(TaskKind::Listener, 2, async {
            tokio::time::sleep(Duration::from_secs(60)).await;
        })
        .unwrap();

    let n = s.retire_generation(1).await;
    assert_eq!(n, 1);
    assert!(s.get(old).is_none());
    assert!(s.get(live).is_some());
    assert_eq!(s.count_for_generation(2), 1);

    let _ = s.shutdown_all().await;
}

#[tokio::test]
async fn leak_free_open_close_cycles() {
    let mut s = TaskSupervisor::new();

    for cycle in 1..=5 {
        s.set_live_generation(cycle);
        // Mixed kinds per cycle.
        for kind in [
            TaskKind::Sync,
            TaskKind::Listener,
            TaskKind::Upload,
            TaskKind::Search,
        ] {
            s.spawn(kind, cycle, async {
                tokio::time::sleep(Duration::from_secs(60)).await;
            })
            .unwrap();
        }
        assert_eq!(s.running_count(), 4);
        assert_eq!(s.count_for_generation(cycle), 4);

        // Generation bump (logout / wipe / fail-reopen).
        s.bump_generation();
        let retired = s.retire_stale().await;
        assert_eq!(retired, 4);
        assert_eq!(s.registered_count(), 0);
        assert_eq!(s.running_count(), 0);
        // Live gen is cycle+1 after bump; accept only that.
        assert!(s.accept_result(cycle).unwrap_err().is_stale_generation());
        assert!(s.accept_result(cycle + 1).is_ok());
    }

    assert_eq!(s.spawned_total(), 20);
    assert_eq!(s.joined_total(), 20);
    assert_eq!(s.cancelled_requests(), 20);
}

#[tokio::test]
async fn shutdown_all_clears_registry() {
    let mut s = TaskSupervisor::new();
    s.set_live_generation(3);
    for _ in 0..3 {
        s.spawn(TaskKind::Generic, 3, async {
            tokio::time::sleep(Duration::from_secs(30)).await;
        })
        .unwrap();
    }
    let n = s.shutdown_all().await;
    assert_eq!(n, 3);
    assert_eq!(s.registered_count(), 0);
    assert_eq!(s.running_count(), 0);
}

#[tokio::test]
async fn follow_supervisor_generation_on_logout() {
    let mut actor = MatrixSupervisor::new();
    let factory = TestClientFactory::new();
    let mut tasks = TaskSupervisor::new();

    harness_login_ready(&mut actor, &factory).unwrap();
    let gen = actor.session_generation();
    assert_eq!(gen, 1);
    mirror_generation(&mut tasks, &actor);

    let id = tasks
        .spawn(TaskKind::Sync, gen, async {
            tokio::time::sleep(Duration::from_secs(60)).await;
        })
        .unwrap();
    assert_eq!(tasks.running_count(), 1);

    actor.apply(SupervisorCommand::BeginStop).unwrap();
    actor.apply(SupervisorCommand::CompleteLogout).unwrap();
    assert_eq!(actor.session_generation(), 2);

    let retired = follow_supervisor_generation(&mut tasks, &actor).await;
    assert_eq!(retired, 1);
    assert!(tasks.get(id).is_none());
    assert_eq!(tasks.live_generation(), 2);
    assert!(tasks.accept_result(1).unwrap_err().is_stale_generation());
    assert!(tasks.accept_result(2).is_ok());
}

#[tokio::test]
async fn follow_supervisor_generation_on_wipe() {
    let mut actor = MatrixSupervisor::new();
    let factory = TestClientFactory::new();
    let mut tasks = TaskSupervisor::new();

    harness_login_ready(&mut actor, &factory).unwrap();
    mirror_generation(&mut tasks, &actor);
    let gen = tasks.live_generation();

    tasks
        .spawn(TaskKind::Upload, gen, async {
            tokio::time::sleep(Duration::from_secs(60)).await;
        })
        .unwrap();
    tasks
        .spawn(TaskKind::Search, gen, async {
            tokio::time::sleep(Duration::from_secs(60)).await;
        })
        .unwrap();

    actor.apply(SupervisorCommand::BeginWipe).unwrap();
    actor.apply(SupervisorCommand::CompleteWipe).unwrap();
    assert_eq!(actor.session_generation(), 2);

    let retired = follow_supervisor_generation(&mut tasks, &actor).await;
    assert_eq!(retired, 2);
    assert_eq!(tasks.registered_count(), 0);
    assert_eq!(tasks.live_generation(), 2);
}

#[tokio::test]
async fn fail_reopen_bumps_and_retires_via_follow() {
    let mut actor = MatrixSupervisor::new();
    let factory = TestClientFactory::new();
    let mut tasks = TaskSupervisor::new();

    harness_login_ready(&mut actor, &factory).unwrap();
    mirror_generation(&mut tasks, &actor);
    tasks
        .spawn(TaskKind::Listener, 1, async {
            tokio::time::sleep(Duration::from_secs(60)).await;
        })
        .unwrap();

    // Fail does not bump generation (P2.1); reopen via BeginOpen does.
    actor.apply(SupervisorCommand::Fail).unwrap();
    let retired_same = follow_supervisor_generation(&mut tasks, &actor).await;
    assert_eq!(retired_same, 0);
    assert_eq!(tasks.running_count(), 1);

    actor.apply(SupervisorCommand::BeginOpen).unwrap();
    assert_eq!(actor.session_generation(), 2);
    let retired = follow_supervisor_generation(&mut tasks, &actor).await;
    assert_eq!(retired, 1);
    assert_eq!(tasks.registered_count(), 0);
    assert_eq!(tasks.live_generation(), 2);
}

#[test]
fn cancel_unknown_task_errors() {
    let mut s = TaskSupervisor::new();
    let err = s.cancel(TaskId::from_raw(99)).unwrap_err();
    assert!(matches!(err, TaskError::UnknownTask { id } if id.get() == 99));
}
