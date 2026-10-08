//! Serialize sync recovery after wake with logout and session replacement.

use std::time::SystemTime;

use tokio::sync::Mutex;

use crate::app::sync::{recover_cooldown_active, RECOVER_COOLDOWN};

/// In-flight flag and wall-clock cooldown for sync recovery after wake.
/// Concurrent renderer and watchdog requests share one stop/start.
#[derive(Debug, Default)]
pub struct RecoverGate {
    in_flight: bool,
    last_success_wall: Option<SystemTime>,
}

/// Run `resume` for the installed session's owner unless a recovery is already
/// in flight or (without `ignore_cooldown`) one succeeded within the cooldown;
/// then return `observe` of the current slot instead.
///
/// Recovery observers acquire recovery then session; completion releases the
/// session before reacquiring recovery, so the lock order never reverses. The
/// session guard is held through `resume`, which serializes recovery with
/// logout and replacement of the installed owner.
pub async fn recover_installed_session_owner<Session, Owner, Snapshot, Error, ResumeFuture>(
    session: &Mutex<Option<Session>>,
    recover_gate: &Mutex<RecoverGate>,
    ignore_cooldown: bool,
    owner: impl FnOnce(&Session) -> Owner,
    observe: impl FnOnce(Option<&Session>) -> Snapshot,
    resume: impl FnOnce(Owner) -> ResumeFuture,
) -> Result<Snapshot, Error>
where
    ResumeFuture: std::future::Future<Output = Result<Snapshot, Error>>,
{
    let mut gate = recover_gate.lock().await;
    if gate.in_flight
        || (!ignore_cooldown
            && recover_cooldown_active(gate.last_success_wall, SystemTime::now(), RECOVER_COOLDOWN))
    {
        drop(gate);
        let guard = session.lock().await;
        return Ok(observe(guard.as_ref()));
    }
    let guard = session.lock().await;
    let Some(active) = guard.as_ref() else {
        return Ok(observe(None));
    };
    let owner = owner(active);
    gate.in_flight = true;
    drop(gate);
    let result = resume(owner).await;
    drop(guard);
    let mut gate = recover_gate.lock().await;
    gate.in_flight = false;
    if result.is_ok() {
        gate.last_success_wall = Some(SystemTime::now());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn a_successful_recovery_starts_the_cooldown_unless_ignored() {
        let session = Mutex::new(Some(7_u64));
        let gate = Mutex::new(RecoverGate::default());
        let resumes = AtomicUsize::new(0);
        for ignore_cooldown in [false, false, true] {
            let observed: Result<u64, ()> = recover_installed_session_owner(
                &session,
                &gate,
                ignore_cooldown,
                |value| *value,
                |value| value.copied().unwrap_or(0),
                |value| {
                    resumes.fetch_add(1, Ordering::SeqCst);
                    async move { Ok(value) }
                },
            )
            .await;
            assert_eq!(observed, Ok(7));
        }
        assert_eq!(resumes.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn an_empty_slot_observes_without_resuming() {
        let session = Mutex::new(None::<u64>);
        let gate = Mutex::new(RecoverGate::default());
        let observed: Result<u64, ()> = recover_installed_session_owner(
            &session,
            &gate,
            true,
            |value| *value,
            |value| value.copied().unwrap_or(0),
            |_| async { panic!("no owner to resume") },
        )
        .await;
        assert_eq!(observed, Ok(0));
    }

    // Moved from the desktop shell with the recover gate itself.
    #[tokio::test]
    async fn suspend_resume_holds_session_gate_until_completion_and_selects_replacement_owner() {
        use std::cell::Cell;
        use std::sync::{
            atomic::{AtomicU64, Ordering},
            Mutex as StdMutex,
        };
        let session = tokio::sync::Mutex::new(Some(1_u64));
        let recovery = tokio::sync::Mutex::new(RecoverGate::default());
        let current = AtomicU64::new(1);
        let events = StdMutex::new(Vec::new());
        let (started_send, started_receive) = tokio::sync::oneshot::channel();
        let (release_send, release_receive) = tokio::sync::oneshot::channel();
        let resume = recover_installed_session_owner(
            &session,
            &recovery,
            true,
            |generation| *generation,
            |active| active.copied(),
            |generation| {
                let current = &current;
                let events = &events;
                async move {
                    assert_eq!(generation, 1);
                    events.lock().unwrap().push("old-resume-start");
                    started_send.send(()).unwrap();
                    release_receive.await.unwrap();
                    assert_eq!(
                        current.load(Ordering::Acquire),
                        generation,
                        "logout/replacement cannot retire a currently resuming owner"
                    );
                    events.lock().unwrap().push("old-resume-complete");
                    Ok::<_, ()>(Some(generation))
                }
            },
        );
        let replace = async {
            started_receive.await.unwrap();
            assert!(
                session.try_lock().is_err(),
                "teardown is blocked until old resume finishes"
            );
            // SDK work releases recovery. An observer may then wait for session
            // while holding recovery; completion must drop session first.
            let gate = recovery.lock().await;
            assert!(gate.in_flight);
            release_send.send(()).unwrap();
            let mut active = session.lock().await;
            events.lock().unwrap().push("old-stop-new-install");
            current.store(2, Ordering::Release);
            *active = Some(2);
            drop(active);
            drop(gate);
        };
        let (result, ()) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            tokio::join!(resume, replace)
        })
        .await
        .expect("resume releases session before reacquiring recovery bookkeeping");
        assert_eq!(result.unwrap(), Some(1));
        assert_eq!(
            *events.lock().unwrap(),
            [
                "old-resume-start",
                "old-resume-complete",
                "old-stop-new-install"
            ]
        );
        let called = Cell::new(false);
        let observed = recover_installed_session_owner(
            &session,
            &recovery,
            false,
            |generation| *generation,
            |active| active.copied(),
            |_| async {
                called.set(true);
                Ok::<_, ()>(None)
            },
        )
        .await
        .unwrap();
        assert_eq!(observed, Some(2));
        assert!(
            !called.get(),
            "successful recovery preserves renderer wall-clock cooldown"
        );
        let resumed = recover_installed_session_owner(
            &session,
            &recovery,
            true,
            |generation| *generation,
            |active| active.copied(),
            |generation| async move {
                assert_eq!(
                    generation, 2,
                    "after replacement only the installed owner resumes"
                );
                assert_eq!(current.load(Ordering::Acquire), generation);
                Ok::<_, ()>(Some(generation))
            },
        )
        .await
        .unwrap();
        assert_eq!(resumed, Some(2));
        assert!(!recovery.lock().await.in_flight);
        assert!(recovery.lock().await.last_success_wall.is_some());
        *session.lock().await = None;
        let logged_out = recover_installed_session_owner(
            &session,
            &recovery,
            true,
            |generation| *generation,
            |active| active.copied(),
            |_| async {
                called.set(true);
                Ok::<_, ()>(None)
            },
        )
        .await
        .unwrap();
        assert!(
            logged_out.is_none() && !called.get(),
            "no retained old owner resumes after logout"
        );
    }

    #[tokio::test]
    async fn suspend_resume_inflight_and_error_preserve_recovery_gate_contract() {
        use std::cell::Cell;
        let session = tokio::sync::Mutex::new(Some(1_u64));
        let recovery = tokio::sync::Mutex::new(RecoverGate {
            in_flight: true,
            last_success_wall: None,
        });
        let called = Cell::new(false);
        assert_eq!(
            recover_installed_session_owner(
                &session,
                &recovery,
                true,
                |generation| *generation,
                |active| active.copied(),
                |_| async {
                    called.set(true);
                    Ok::<_, &'static str>(None)
                },
            )
            .await
            .unwrap(),
            Some(1)
        );
        assert!(
            !called.get(),
            "concurrent recovery observes instead of issuing a second resume"
        );
        recovery.lock().await.in_flight = false;
        let error = recover_installed_session_owner(
            &session,
            &recovery,
            false,
            |generation| *generation,
            |active| active.copied(),
            |_| async { Err("fixture-resume-failed") },
        )
        .await
        .unwrap_err();
        assert_eq!(error, "fixture-resume-failed");
        let gate = recovery.lock().await;
        assert!(!gate.in_flight && gate.last_success_wall.is_none());
        drop(gate);
        let retried = recover_installed_session_owner(
            &session,
            &recovery,
            false,
            |generation| *generation,
            |active| active.copied(),
            |generation| async move { Ok::<_, &'static str>(Some(generation)) },
        )
        .await
        .unwrap();
        assert_eq!(
            retried,
            Some(1),
            "ordinary retry remains allowed after resume error"
        );
    }
}
