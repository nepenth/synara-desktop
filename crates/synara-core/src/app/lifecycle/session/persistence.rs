//! The fence between SDK token-rotation saves and session teardown.

use std::sync::Arc;

use super::SessionFault;

const PERSISTENCE_RETIRED: SessionFault =
    SessionFault::unavailable("d0.1-session-persistence-retired");

#[derive(Debug, Default)]
struct SessionPersistenceState {
    revoked: bool,
    credential_write_attempted: bool,
    save_failed: bool,
}

/// Synchronous SDK save callbacks and teardown share this per-client fence.
/// Revocation waits for a current save, then permanently rejects late saves,
/// so a refresh that lands after logout cannot write credentials back.
#[derive(Debug, Default)]
pub struct SessionPersistenceLease {
    state: std::sync::Mutex<SessionPersistenceState>,
}

impl SessionPersistenceLease {
    /// Locator preflight and credential persistence share the retirement
    /// fence. Provenance is recorded immediately before the writer, including
    /// partial errors.
    pub fn save_credentials<T, E: From<SessionFault>>(
        &self,
        preflight: impl FnOnce() -> Result<(), E>,
        persist: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, E> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| E::from(PERSISTENCE_RETIRED))?;
        if state.revoked {
            return Err(E::from(PERSISTENCE_RETIRED));
        }
        let result = preflight().and_then(|()| {
            state.credential_write_attempted = true;
            persist()
        });
        state.save_failed = result.is_err();
        result
    }

    /// Save with no preflight.
    pub fn save<T, E: From<SessionFault>>(
        &self,
        operation: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, E> {
        self.save_credentials(|| Ok(()), operation)
    }

    /// Whether the last save failed. A poisoned lock counts as failed.
    pub fn save_failed(&self) -> bool {
        self.state.lock().map_or(true, |state| state.save_failed)
    }

    pub fn is_revoked(&self) -> bool {
        self.state.lock().map_or(true, |state| state.revoked)
    }

    /// Wait for in-flight writes, permanently reject callbacks, and return
    /// this client's write provenance so rollback of a new login cannot clear
    /// an older login's credentials.
    pub fn revoke(&self) -> bool {
        // Poison is also revoked: a writer panic cannot enable a later save.
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.revoked = true;
        state.credential_write_attempted
    }
}

/// Moves from pre-install custody into the installed session owner. Dropping
/// any unsuccessful preparation retires its SDK save callbacks too.
#[derive(Debug, Default)]
pub struct SessionPersistenceOwner {
    lease: Arc<SessionPersistenceLease>,
}

impl SessionPersistenceOwner {
    pub fn new() -> Self {
        Self::default()
    }

    /// The lease the SDK save callback holds.
    pub fn callback_lease(&self) -> Arc<SessionPersistenceLease> {
        Arc::clone(&self.lease)
    }

    pub fn lease(&self) -> &SessionPersistenceLease {
        &self.lease
    }
}

impl Drop for SessionPersistenceOwner {
    fn drop(&mut self) {
        self.lease.revoke();
    }
}

/// Save once through the lease; on failure revoke it and, when this client
/// attempted a write, run `cleanup` so a failed preparation leaves nothing.
pub fn persist_with_client_lease<E: From<SessionFault>>(
    lease: &SessionPersistenceLease,
    preflight: impl FnOnce() -> Result<(), E>,
    persist: impl FnOnce() -> Result<(), E>,
    cleanup: impl FnOnce() -> Result<(), E>,
) -> Result<(), E> {
    let saved = lease.save_credentials(preflight, persist);
    if let Err(error) = saved {
        if lease.revoke() {
            cleanup()?;
        }
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fault(id: &'static str) -> SessionFault {
        SessionFault::unavailable(id)
    }

    #[test]
    fn revoke_reports_write_provenance_and_blocks_late_saves() {
        let lease = SessionPersistenceLease::default();
        assert!(!lease.revoke(), "nothing written");
        let late = lease.save(|| -> Result<(), SessionFault> { panic!("late save must not run") });
        assert_eq!(
            late.unwrap_err().diagnostic_id,
            "d0.1-session-persistence-retired"
        );

        let lease = SessionPersistenceLease::default();
        lease.save(|| Ok::<(), SessionFault>(())).unwrap();
        assert!(lease.revoke(), "a credential write was attempted");
    }

    #[test]
    fn failed_save_is_tracked_and_cleared_by_success() {
        let lease = SessionPersistenceLease::default();
        assert!(lease
            .save(|| Err::<(), _>(fault("d0.1-session-rotation-persist-failed")))
            .is_err());
        assert!(lease.save_failed());
        lease.save(|| Ok::<(), SessionFault>(())).unwrap();
        assert!(!lease.save_failed());
    }

    #[test]
    fn failed_preflight_does_not_count_as_a_write() {
        let lease = SessionPersistenceLease::default();
        let result = lease.save_credentials(
            || Err(fault("d0.1-session-locator-mismatch")),
            || -> Result<(), SessionFault> { panic!("writer must not run") },
        );
        assert!(result.is_err());
        assert!(!lease.revoke());
    }

    #[test]
    fn dropping_the_owner_revokes_its_callback_lease() {
        let owner = SessionPersistenceOwner::new();
        let callback = owner.callback_lease();
        drop(owner);
        assert!(callback.is_revoked());
        assert!(callback.save(|| Ok::<(), SessionFault>(())).is_err());
    }

    #[test]
    fn failed_persist_revokes_and_cleans_up_only_after_a_write() {
        let lease = SessionPersistenceLease::default();
        let mut cleaned = false;
        let result = persist_with_client_lease(
            &lease,
            || Ok(()),
            || Err(fault("d0.1-session-rotation-persist-failed")),
            || {
                cleaned = true;
                Ok::<(), SessionFault>(())
            },
        );
        assert!(result.is_err());
        assert!(cleaned, "a partial write is cleaned up");
        assert!(lease.is_revoked());

        let lease = SessionPersistenceLease::default();
        let result = persist_with_client_lease(
            &lease,
            || Err(fault("d0.1-session-locator-mismatch")),
            || Ok(()),
            || -> Result<(), SessionFault> { panic!("nothing was written") },
        );
        assert!(result.is_err());
    }
}
