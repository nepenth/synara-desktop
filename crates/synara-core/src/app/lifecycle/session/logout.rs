//! Logout sequencing shared by every platform.
//!
//! The helpers are generic over the shell's error type so desktop keeps its
//! `MatrixAuthCommandError` payloads byte-for-byte and iOS keeps its own.

use std::future::Future;
use std::time::Duration;

use tokio::sync::Mutex;

use super::{SessionFault, SessionLocator};

/// Upper bound for one voluntary remote `/logout` attempt.
pub const VOLUNTARY_REMOTE_LOGOUT_TIMEOUT: Duration = Duration::from_secs(15);

/// One bounded `/logout` attempt. A 401, timeout or transport error is `false`
/// so local cleanup still runs. Rejected-session retirement never calls this.
pub async fn bounded_remote_logout<T, E>(
    timeout: Duration,
    logout: impl Future<Output = Result<T, E>>,
) -> bool {
    matches!(tokio::time::timeout(timeout, logout).await, Ok(Ok(_)))
}

/// Validate the installed session under the slot mutex, then move it out of
/// the slot before any teardown await. From here on, ordinary commands see no
/// session and fail closed, sync recovery has nothing to restart, and the
/// mutex is free while the remote logout and SDK stop run. `Ok(None)` means
/// the slot was already empty.
pub async fn take_session_for_logout<Session, Plan, E>(
    slot: &Mutex<Option<Session>>,
    prepare: impl FnOnce(&Session) -> Result<Plan, E>,
) -> Result<Option<(Session, Plan)>, E> {
    let mut slot = slot.lock().await;
    let Some(active) = slot.as_ref() else {
        return Ok(None);
    };
    let plan = prepare(active)?;
    Ok(slot.take().map(|active| (active, plan)))
}

/// Once teardown begins, stop errors cannot retain a revoked live client.
/// Always attempt credential cleanup, retire live ownership, and close Core.
/// The fixed result contract returns credential failure first, then Core
/// close, then sync stop; all three outcomes are observed before any error.
pub async fn finish_active_logout<E, Begin, BeginFuture, Close, CloseFuture>(
    preflight: impl FnOnce() -> Result<(), E>,
    begin: Begin,
    cleanup: impl FnOnce() -> Result<(), E>,
    retire: impl FnOnce(),
    close: Close,
) -> Result<(), E>
where
    Begin: FnOnce() -> BeginFuture,
    BeginFuture: Future<Output = Result<(), E>>,
    Close: FnOnce() -> CloseFuture,
    CloseFuture: Future<Output = Result<(), E>>,
{
    preflight()?;
    let stop_result = begin().await;
    let cleanup_result = cleanup();
    retire();
    let close_result = close().await;
    cleanup_result?;
    close_result?;
    stop_result
}

/// Logout with no installed session: close any stale Core, then clear
/// whatever persisted material is left. Cleanup failure is reported first.
pub async fn finish_orphan_logout<E>(
    close: impl Future<Output = Result<(), E>>,
    cleanup: impl FnOnce() -> Result<(), E>,
) -> Result<(), E> {
    let close_result = close.await;
    let cleanup_result = cleanup();
    cleanup_result?;
    close_result
}

/// Tear down a session that [`take_session_for_logout`] already removed.
///
/// `revoke` stops credential writes first. Sync stops before the remote call
/// so a running sync loop cannot race the server-side revocation into a
/// refresh 401. `remote_logout` is `None` for a rejected generation; otherwise
/// it is one attempt bounded by `remote_timeout`. A 401, timeout or transport
/// error still runs local cleanup and Core close.
#[allow(clippy::too_many_arguments)]
pub async fn finish_taken_session_logout<
    Session,
    E,
    Remote,
    RemoteFuture,
    RemoteOk,
    RemoteErr,
    Stop,
    StopFuture,
    Close,
    CloseFuture,
>(
    session: Session,
    remote_timeout: Duration,
    revoke: impl FnOnce(),
    remote_logout: Option<Remote>,
    stop_local: Stop,
    cleanup: impl FnOnce() -> Result<(), E>,
    close: Close,
) -> Result<(), E>
where
    Remote: FnOnce() -> RemoteFuture,
    RemoteFuture: Future<Output = Result<RemoteOk, RemoteErr>>,
    Stop: FnOnce() -> StopFuture,
    StopFuture: Future<Output = Result<(), E>>,
    Close: FnOnce() -> CloseFuture,
    CloseFuture: Future<Output = Result<(), E>>,
{
    finish_active_logout(
        || Ok(()),
        || async move {
            revoke();
            let stop_result = stop_local().await;
            if let Some(remote_logout) = remote_logout {
                let _remote_logout_succeeded =
                    bounded_remote_logout(remote_timeout, remote_logout()).await;
            }
            stop_result
        },
        cleanup,
        move || drop(session),
        close,
    )
    .await
}

/// Locator whose credential cleanup failed after its live session left the
/// slot. Restore must finish that cleanup before reading an identity, and the
/// watcher retries it, so a retired session cannot come back.
#[derive(Debug, Default)]
pub struct PendingLogoutCleanup(std::sync::Mutex<Option<SessionLocator>>);

impl PendingLogoutCleanup {
    pub fn record(&self, locator: SessionLocator) {
        if let Ok(mut pending) = self.0.lock() {
            *pending = Some(locator);
        }
    }

    /// A poisoned lock counts as pending so callers fail closed.
    pub fn is_pending(&self) -> bool {
        self.0
            .lock()
            .map(|pending| pending.is_some())
            .unwrap_or(true)
    }

    /// Run `cleanup` for the pending locator, if any, and clear it on success.
    pub fn retry<E: From<SessionFault>>(
        &self,
        cleanup: impl FnOnce(&SessionLocator) -> Result<(), E>,
    ) -> Result<(), E> {
        let mut pending = self
            .0
            .lock()
            .map_err(|_| E::from(SessionFault::unavailable("d0.1-session-clear-failed")))?;
        let Some(locator) = pending.as_ref() else {
            return Ok(());
        };
        cleanup(locator)?;
        *pending = None;
        Ok(())
    }

    /// Before a new install: one last cleanup attempt, then forget the record
    /// so a later retry cannot erase the new session's credentials.
    pub fn settle_before_install<E: From<SessionFault>>(
        &self,
        cleanup: impl FnOnce(&SessionLocator) -> Result<(), E>,
    ) {
        let _ = self.retry(cleanup);
        if let Ok(mut pending) = self.0.lock() {
            *pending = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn locator() -> SessionLocator {
        SessionLocator {
            user_id: "@alice:example.org".to_owned(),
            device_id: "DEVICE".to_owned(),
            homeserver_url: "https://matrix.example.org".to_owned(),
        }
    }

    #[tokio::test]
    async fn remote_logout_that_never_returns_is_bounded() {
        let pending = std::future::pending::<Result<(), ()>>();
        assert!(!bounded_remote_logout(Duration::from_millis(50), pending).await);
        assert!(bounded_remote_logout(Duration::from_secs(15), async { Ok::<(), ()>(()) }).await);
        assert!(!bounded_remote_logout(Duration::from_secs(15), async { Err::<(), ()>(()) }).await);
    }

    #[tokio::test]
    async fn taken_session_teardown_frees_the_slot_while_remote_logout_hangs() {
        let slot = Arc::new(Mutex::new(Some(7_u64)));
        let (session, plan) =
            take_session_for_logout(&slot, |active| Ok::<_, SessionFault>(*active))
                .await
                .unwrap()
                .unwrap();
        assert_eq!((session, plan), (7, 7));
        assert!(slot.lock().await.is_none(), "slot is empty before teardown");

        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let record = |step: &'static str| {
            let order = Arc::clone(&order);
            move || order.lock().unwrap().push(step)
        };
        let stop = record("stop");
        let close = record("close");
        let teardown = finish_taken_session_logout(
            session,
            Duration::from_millis(200),
            record("revoke"),
            Some({
                let remote = record("remote");
                move || {
                    remote();
                    std::future::pending::<Result<(), ()>>()
                }
            }),
            || async move {
                stop();
                Ok::<(), SessionFault>(())
            },
            || Ok(()),
            || async move {
                close();
                Ok(())
            },
        );
        tokio::pin!(teardown);
        tokio::select! {
            _ = &mut teardown => panic!("hung remote logout must not finish early"),
            guard = slot.lock() => assert!(guard.is_none(), "slot mutex is free during the wait"),
        }
        teardown.await.unwrap();
        assert_eq!(
            *order.lock().unwrap(),
            ["revoke", "stop", "remote", "close"],
            "sync stops before the remote /logout"
        );
    }

    #[tokio::test]
    async fn active_logout_observes_every_outcome_and_reports_cleanup_first() {
        let closed = AtomicUsize::new(0);
        let result = finish_active_logout(
            || Ok(()),
            || async { Err(SessionFault::unavailable("d0.1-sync-stop-failed")) },
            || Err(SessionFault::unavailable("d0.1-session-clear-failed")),
            || {},
            || async {
                closed.fetch_add(1, Ordering::Relaxed);
                Err(SessionFault::unavailable("d0.1-core-close-failed"))
            },
        )
        .await;
        assert_eq!(
            result.unwrap_err().diagnostic_id,
            "d0.1-session-clear-failed"
        );
        assert_eq!(closed.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn orphan_logout_closes_then_cleans_and_reports_cleanup_first() {
        let result = finish_orphan_logout(
            async { Err(SessionFault::unavailable("d0.1-core-close-failed")) },
            || Err(SessionFault::unavailable("d0.1-session-clear-failed")),
        )
        .await;
        assert_eq!(
            result.unwrap_err().diagnostic_id,
            "d0.1-session-clear-failed"
        );
    }

    #[test]
    fn pending_cleanup_retries_until_it_succeeds_and_settles_before_install() {
        let pending = PendingLogoutCleanup::default();
        assert!(!pending.is_pending());
        pending
            .retry(|_| -> Result<(), SessionFault> { panic!("nothing pending") })
            .unwrap();

        pending.record(locator());
        assert!(pending.is_pending());
        assert!(pending
            .retry(|_| Err(SessionFault::unavailable("d0.1-session-clear-failed")))
            .is_err());
        assert!(pending.is_pending(), "failed cleanup stays pending");
        pending
            .retry(|seen| {
                assert_eq!(*seen, locator());
                Ok::<(), SessionFault>(())
            })
            .unwrap();
        assert!(!pending.is_pending());

        pending.record(locator());
        pending
            .settle_before_install(|_| Err(SessionFault::unavailable("d0.1-session-clear-failed")));
        assert!(
            !pending.is_pending(),
            "a new install forgets the old record"
        );
    }
}
