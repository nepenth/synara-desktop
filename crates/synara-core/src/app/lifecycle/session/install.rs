//! Install and rollback primitives for a new or restored session.
//!
//! Generic over the shell's error type so desktop payloads stay byte-for-byte.

use std::future::Future;

use tokio::sync::Mutex;

use super::{finish_active_logout, SessionPersistenceLease};

/// Where a tentative install came from. Only newly minted authentication is
/// revoked remotely on rollback; a restored session belongs to the account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionInstallOrigin {
    NewAuthentication,
    Restored,
}

/// Run `rollback` when preparation failed, then return the original error.
pub async fn finish_session_preparation<T, E, Rollback, RollbackFuture>(
    preparation: Result<T, E>,
    rollback: Rollback,
) -> Result<T, E>
where
    Rollback: FnOnce() -> RollbackFuture,
    RollbackFuture: Future<Output = Result<(), E>>,
{
    match preparation {
        Ok(prepared) => Ok(prepared),
        Err(error) => {
            rollback().await?;
            Err(error)
        }
    }
}

/// Core wiring failed after the session was published: same rollback rule.
pub async fn finish_session_wiring<E, Rollback, RollbackFuture>(
    wiring: Result<(), E>,
    rollback: Rollback,
) -> Result<(), E>
where
    Rollback: FnOnce() -> RollbackFuture,
    RollbackFuture: Future<Output = Result<(), E>>,
{
    finish_session_preparation(wiring, rollback).await
}

/// A local wiring error must not revoke an existing restored account session.
/// Both origins retire local owners; only newly minted authentication is undone
/// remotely, and its attempted vault writes are cleared only after the lease
/// confirms this attempt wrote credentials. Caller keeps the transition gate
/// through Core close.
pub async fn finish_install_rollback<
    E,
    Remote,
    RemoteFuture,
    Stop,
    StopFuture,
    Close,
    CloseFuture,
>(
    origin: SessionInstallOrigin,
    persistence_lease: &SessionPersistenceLease,
    remote_logout: Remote,
    stop_local: Stop,
    cleanup: impl FnOnce() -> Result<(), E>,
    retire: impl FnOnce(),
    close: Close,
) -> Result<(), E>
where
    Remote: FnOnce() -> RemoteFuture,
    RemoteFuture: Future<Output = ()>,
    Stop: FnOnce() -> StopFuture,
    StopFuture: Future<Output = Result<(), E>>,
    Close: FnOnce() -> CloseFuture,
    CloseFuture: Future<Output = Result<(), E>>,
{
    // Waiting for an active save preserves the latest legitimate refreshed
    // token for restored retry. Late callbacks are rejected for either origin.
    let credential_write_attempted = persistence_lease.revoke();
    if origin == SessionInstallOrigin::NewAuthentication {
        remote_logout().await;
    }
    finish_active_logout(
        || Ok(()),
        stop_local,
        || {
            if origin == SessionInstallOrigin::NewAuthentication && credential_write_attempted {
                cleanup()
            } else {
                Ok(())
            }
        },
        retire,
        close,
    )
    .await
}

/// Revoke a freshly authenticated session whose identity was rejected, then
/// return the rejection unchanged.
pub async fn accept_authenticated_identity<T, E, Revoke, RevokeFuture>(
    identity: Result<T, E>,
    revoke: Revoke,
) -> Result<T, E>
where
    Revoke: FnOnce() -> RevokeFuture,
    RevokeFuture: Future<Output = ()>,
{
    if identity.is_err() {
        revoke().await;
    }
    identity
}

/// Run `accept` only while `expected_generation` is the installed session,
/// holding the slot so the generation cannot change underneath it.
pub async fn with_generation_bound_acceptance<Session, T, Accept, AcceptFuture>(
    session: &Mutex<Option<Session>>,
    expected_generation: u64,
    generation: impl FnOnce(&Session) -> u64,
    accept: Accept,
) -> Option<T>
where
    Accept: FnOnce() -> AcceptFuture,
    AcceptFuture: Future<Output = T>,
{
    let guard = session.lock().await;
    let active = guard.as_ref()?;
    if generation(active) != expected_generation {
        return None;
    }
    let accepted = accept().await;
    drop(guard);
    Some(accepted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[tokio::test]
    async fn restored_rollback_never_revokes_remotely_or_clears_credentials() {
        for origin in [
            SessionInstallOrigin::NewAuthentication,
            SessionInstallOrigin::Restored,
        ] {
            let lease = SessionPersistenceLease::default();
            let _ = lease.save_credentials(
                || Ok::<_, super::super::SessionFault>(()),
                || Ok::<_, super::super::SessionFault>(()),
            );
            let steps = RefCell::new(Vec::new());
            let result: Result<(), ()> = finish_install_rollback(
                origin,
                &lease,
                || async { steps.borrow_mut().push("remote") },
                || async {
                    steps.borrow_mut().push("stop");
                    Ok(())
                },
                || {
                    steps.borrow_mut().push("cleanup");
                    Ok(())
                },
                || steps.borrow_mut().push("retire"),
                || async {
                    steps.borrow_mut().push("close");
                    Ok(())
                },
            )
            .await;
            assert!(result.is_ok());
            let expected: &[&str] = match origin {
                SessionInstallOrigin::NewAuthentication => {
                    &["remote", "stop", "cleanup", "retire", "close"]
                }
                SessionInstallOrigin::Restored => &["stop", "retire", "close"],
            };
            assert_eq!(steps.into_inner(), expected);
        }
    }

    #[tokio::test]
    async fn preparation_failure_rolls_back_and_keeps_the_error() {
        let rolled_back = RefCell::new(false);
        let result: Result<(), &str> = finish_session_preparation(Err("prepare"), || async {
            *rolled_back.borrow_mut() = true;
            Ok(())
        })
        .await;
        assert_eq!(result, Err("prepare"));
        assert!(rolled_back.into_inner());
    }

    #[tokio::test]
    async fn generation_bound_acceptance_refuses_a_replaced_session() {
        let slot = Mutex::new(Some(3_u64));
        assert_eq!(
            with_generation_bound_acceptance(&slot, 3, |g| *g, || async { "ok" }).await,
            Some("ok")
        );
        assert_eq!(
            with_generation_bound_acceptance(&slot, 2, |g| *g, || async { "ok" }).await,
            None
        );
    }
}
