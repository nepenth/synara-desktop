//! [`SessionLifecycleOwner`]: the session slot and every lock and policy
//! state that guards it.
//!
//! Lock order, always: `transition` → `session` → `store_recovery`.
//! `recover_gate` is taken before `session` and released before the session
//! is reacquired (see [`super::recover_installed_session_owner`]).
//! `maintenance` is a std mutex never held across an await.

use std::future::Future;
use std::time::Instant;

use tokio::sync::{Mutex, MutexGuard};

use super::maintenance::{MaintenanceReport, RejectedGeneration, SessionMaintenance};
use super::{
    handle_authentication_rejection_tick, recover_installed_session_owner, HasDiagnosticId,
    PendingLogoutCleanup, RecoverGate, SessionFault, SessionGenerations, SessionLocator,
    StoreRecoveryState,
};
use crate::app::store::AccountIdentity;

/// What the owner needs to know about the shell's installed-session record.
pub trait InstalledSession {
    fn session_generation(&self) -> u64;
}

/// How a logout ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogoutOutcome {
    /// No session was installed; only orphan cleanup ran.
    NoSession,
    /// The installed session was taken out of the slot and torn down.
    TornDown,
}

/// The session slot plus the transition gate, generation counter, pending
/// credential cleanup, store-recovery capability, recover gate and periodic
/// maintenance state that every platform keeps around it.
pub struct SessionLifecycleOwner<S> {
    /// The installed session. Ordinary commands lock it briefly and fail
    /// closed when it is empty; they never take the transition gate.
    pub session: Mutex<Option<S>>,
    /// Logout holds it through the whole teardown; login, register, restore
    /// and store recovery take it before `session`.
    transition: Mutex<()>,
    /// Locator whose credential cleanup failed after its session left the
    /// slot. Restore finishes it first, so a retired session cannot return.
    pending_cleanup: PendingLogoutCleanup,
    store_recovery: Mutex<StoreRecoveryState>,
    generations: SessionGenerations,
    recover_gate: Mutex<RecoverGate>,
    maintenance: std::sync::Mutex<SessionMaintenance>,
}

impl<S> Default for SessionLifecycleOwner<S> {
    fn default() -> Self {
        Self {
            session: Mutex::new(None),
            transition: Mutex::new(()),
            pending_cleanup: PendingLogoutCleanup::default(),
            store_recovery: Mutex::new(StoreRecoveryState::default()),
            generations: SessionGenerations::default(),
            recover_gate: Mutex::new(RecoverGate::default()),
            maintenance: std::sync::Mutex::new(SessionMaintenance::default()),
        }
    }
}

/// Transition gate plus session slot, held for one install attempt.
pub struct SessionInstallGuard<'a, S> {
    owner: &'a SessionLifecycleOwner<S>,
    _transition: MutexGuard<'a, ()>,
    session: MutexGuard<'a, Option<S>>,
}

impl<'a, S> SessionInstallGuard<'a, S> {
    /// The session already installed, if any.
    pub fn current(&self) -> Option<&S> {
        self.session.as_ref()
    }

    /// The slot, for rollback after a failed Core wiring.
    pub fn slot_mut(&mut self) -> &mut Option<S> {
        &mut self.session
    }

    /// Publish the new session. A successful install supersedes every pending
    /// or awaiting store-recovery capability.
    pub async fn publish(&mut self, session: S) {
        self.owner.clear_store_recovery().await;
        *self.session = Some(session);
    }

    /// Finish a pending credential cleanup before reading any identity.
    pub fn retry_pending_cleanup<E: From<SessionFault>>(
        &self,
        cleanup: impl FnOnce(&SessionLocator) -> Result<(), E>,
    ) -> Result<(), E> {
        self.owner.pending_cleanup.retry(cleanup)
    }

    /// A new install replaces whatever the failed cleanup was retrying.
    pub fn settle_pending_cleanup<E: From<SessionFault>>(
        &self,
        cleanup: impl FnOnce(&SessionLocator) -> Result<(), E>,
    ) {
        self.owner.pending_cleanup.settle_before_install(cleanup);
    }
}

impl<S> SessionLifecycleOwner<S> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Serialize session installs and logout teardown. Always taken before
    /// `session`, never while holding it.
    pub async fn lock_transition(&self) -> MutexGuard<'_, ()> {
        self.transition.lock().await
    }

    /// Take the transition gate, then the slot, for one install attempt.
    pub async fn begin_install(&self) -> SessionInstallGuard<'_, S> {
        let transition = self.transition.lock().await;
        let session = self.session.lock().await;
        SessionInstallGuard {
            owner: self,
            _transition: transition,
            session,
        }
    }

    pub fn next_generation(&self) -> u64 {
        self.generations.allocate()
    }

    pub fn current_generation(&self) -> u64 {
        self.generations.current()
    }

    pub fn record_pending_cleanup(&self, locator: SessionLocator) {
        self.pending_cleanup.record(locator);
    }

    pub fn has_pending_cleanup(&self) -> bool {
        self.pending_cleanup.is_pending()
    }

    /// Retry a pending cleanup. Callers outside an install hold the
    /// transition gate so a new install cannot interleave.
    pub fn retry_pending_cleanup<E: From<SessionFault>>(
        &self,
        cleanup: impl FnOnce(&SessionLocator) -> Result<(), E>,
    ) -> Result<(), E> {
        self.pending_cleanup.retry(cleanup)
    }

    /// Clears a process-local capability only; never touches files or keys.
    pub async fn clear_store_recovery(&self) {
        self.store_recovery.lock().await.clear();
    }

    pub async fn arm_store_recovery(&self, identity: AccountIdentity) {
        self.store_recovery.lock().await.arm(identity);
    }

    pub async fn prepare_store_recovery_confirmation(&self) -> Result<String, SessionFault> {
        self.store_recovery.lock().await.prepare_confirmation()
    }

    pub async fn take_confirmed_store_recovery(
        &self,
        confirmation_id: &str,
        confirmation_text: &str,
    ) -> Result<AccountIdentity, SessionFault> {
        self.store_recovery
            .lock()
            .await
            .take_confirmed(confirmation_id, confirmation_text)
    }

    /// Resume the installed session's owner after wake; see
    /// [`recover_installed_session_owner`].
    pub async fn recover_live<Owner, Snapshot, Error, ResumeFuture>(
        &self,
        ignore_cooldown: bool,
        owner: impl FnOnce(&S) -> Owner,
        observe: impl FnOnce(Option<&S>) -> Snapshot,
        resume: impl FnOnce(Owner) -> ResumeFuture,
    ) -> Result<Snapshot, Error>
    where
        ResumeFuture: Future<Output = Result<Snapshot, Error>>,
    {
        recover_installed_session_owner(
            &self.session,
            &self.recover_gate,
            ignore_cooldown,
            owner,
            observe,
            resume,
        )
        .await
    }

    /// The one logout shape every platform runs.
    ///
    /// Under the transition gate: validate the installed session with
    /// `prepare` and move it out of the slot before any teardown await (so
    /// ordinary commands fail closed and the slot mutex is free while the
    /// remote call and SDK stop run), revoke any store-recovery capability,
    /// then `teardown` the taken session. With no session installed only
    /// `orphan` cleanup runs. A `prepare` failure leaves the session installed.
    pub async fn logout<E, Plan, Orphan, OrphanFuture, Teardown, TeardownFuture>(
        &self,
        prepare: impl FnOnce(&S) -> Result<Plan, E>,
        orphan: Orphan,
        teardown: Teardown,
    ) -> Result<LogoutOutcome, E>
    where
        Orphan: FnOnce() -> OrphanFuture,
        OrphanFuture: Future<Output = Result<(), E>>,
        Teardown: FnOnce(S, Plan) -> TeardownFuture,
        TeardownFuture: Future<Output = Result<(), E>>,
    {
        let _transition = self.transition.lock().await;
        let taken = super::take_session_for_logout(&self.session, prepare).await?;
        self.clear_store_recovery().await;
        match taken {
            None => {
                orphan().await?;
                Ok(LogoutOutcome::NoSession)
            }
            Some((session, plan)) => {
                teardown(session, plan).await?;
                Ok(LogoutOutcome::TornDown)
            }
        }
    }

    /// Whether a failed-save retry is due (iOS calls this from its status poll).
    pub fn save_retry_due(&self, now: Instant) -> bool {
        self.maintenance
            .lock()
            .map(|maintenance| maintenance.save_retry_due(now))
            .unwrap_or(false)
    }

    pub fn record_save_retry(&self, now: Instant, saved: bool) {
        if let Ok(mut maintenance) = self.maintenance.lock() {
            maintenance.record_save_retry(now, saved);
        }
    }

    /// One periodic maintenance pass, in a fixed order:
    ///
    /// 1. When due, `retry_save` re-saves the current in-memory tokens of a
    ///    failed save (`None` when nothing needed saving). It never replays an
    ///    old refresh token.
    /// 2. When a cleanup is pending and due, `retry_cleanup` runs under the
    ///    transition gate so a new login cannot interleave with the delete.
    /// 3. When `rejected` names a rejected generation, retire it through the
    ///    generation-fenced local `retire`. Lines passed to `log` are static.
    ///
    /// The transition gate is released before `retire`, which takes it itself.
    pub async fn maintenance_tick<E, Save, SaveFuture, Retire, RetireFuture>(
        &self,
        now: Instant,
        retry_save: Save,
        retry_cleanup: impl FnOnce(&SessionLocator) -> Result<(), E>,
        rejected: Option<RejectedGeneration>,
        log: impl FnMut(&str),
        retire: Retire,
    ) -> MaintenanceReport
    where
        E: From<SessionFault> + HasDiagnosticId,
        Save: FnOnce() -> SaveFuture,
        SaveFuture: Future<Output = Option<bool>>,
        Retire: FnOnce(u64) -> RetireFuture,
        RetireFuture: Future<Output = Result<(), E>>,
    {
        let mut report = MaintenanceReport::default();
        if self.save_retry_due(now) {
            report.saved = retry_save().await;
            if let Some(saved) = report.saved {
                self.record_save_retry(Instant::now(), saved);
            }
        }
        let cleanup_due = self.has_pending_cleanup()
            && self
                .maintenance
                .lock()
                .map(|maintenance| maintenance.cleanup_retry_due(now))
                .unwrap_or(false);
        if cleanup_due {
            let _transition = self.transition.lock().await;
            let cleaned = self.pending_cleanup.retry(retry_cleanup).is_ok();
            report.cleaned = Some(cleaned);
            if let Ok(mut maintenance) = self.maintenance.lock() {
                maintenance.record_cleanup_retry(Instant::now(), cleaned);
            }
        }
        if let Some(rejected) = rejected {
            // The watch is single-ticker state; move it out so no std lock is
            // held across the retirement await.
            let mut watch = self
                .maintenance
                .lock()
                .map(|mut maintenance| std::mem::take(&mut maintenance.rejection))
                .unwrap_or_default();
            let retired = handle_authentication_rejection_tick(
                &mut watch,
                rejected.generation,
                rejected.core_present,
                log,
                retire,
            )
            .await;
            if let Ok(mut maintenance) = self.maintenance.lock() {
                maintenance.rejection = watch;
            }
            report.retired = Some(retired);
        }
        report
    }
}

impl<S: InstalledSession> SessionLifecycleOwner<S> {
    /// Run `accept` only while `expected_generation` is installed, keeping
    /// the transition gate so a logout or replacement cannot interleave. The
    /// callback must not call back into this owner.
    pub async fn with_session_generation<T, Accept, AcceptFuture>(
        &self,
        expected_generation: u64,
        accept: Accept,
    ) -> Option<T>
    where
        Accept: FnOnce() -> AcceptFuture,
        AcceptFuture: Future<Output = T>,
    {
        super::with_generation_bound_acceptance(
            &self.session,
            expected_generation,
            |active| active.session_generation(),
            accept,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Session(u64);

    impl InstalledSession for Session {
        fn session_generation(&self) -> u64 {
            self.0
        }
    }

    fn locator() -> SessionLocator {
        SessionLocator {
            user_id: "@alice:example.org".to_owned(),
            device_id: "DEVICE".to_owned(),
            homeserver_url: "https://matrix.example.org".to_owned(),
        }
    }

    #[tokio::test]
    async fn logout_takes_the_session_before_teardown_and_frees_the_slot() {
        let owner = SessionLifecycleOwner::<Session>::new();
        owner.begin_install().await.publish(Session(1)).await;
        let outcome = owner
            .logout(
                |active| Ok::<_, SessionFault>(active.0),
                || async { panic!("a session is installed") },
                |session, plan| {
                    assert_eq!(plan, 1);
                    let owner = &owner;
                    async move {
                        // Teardown runs with the slot already empty and unlocked.
                        assert!(owner.session.try_lock().unwrap().is_none());
                        assert_eq!(session.0, 1);
                        Ok(())
                    }
                },
            )
            .await;
        assert_eq!(outcome, Ok(LogoutOutcome::TornDown));
        let orphan = owner
            .logout(
                |_| Ok::<_, SessionFault>(()),
                || async { Ok(()) },
                |_, ()| async { panic!("no session to tear down") },
            )
            .await;
        assert_eq!(orphan, Ok(LogoutOutcome::NoSession));
    }

    #[tokio::test]
    async fn a_failed_prepare_leaves_the_session_installed() {
        let owner = SessionLifecycleOwner::<Session>::new();
        owner.begin_install().await.publish(Session(4)).await;
        let stale = SessionFault::unavailable("d0.1-session-rejection-stale");
        let result = owner
            .logout(
                |_| Err::<(), _>(stale),
                || async { Ok(()) },
                |_, ()| async { Ok(()) },
            )
            .await;
        assert_eq!(result, Err(stale));
        assert_eq!(owner.session.lock().await.as_ref().map(|s| s.0), Some(4));
    }

    #[tokio::test]
    async fn publish_revokes_store_recovery() {
        let owner = SessionLifecycleOwner::<Session>::new();
        owner
            .arm_store_recovery(
                AccountIdentity::new("@alice:example.org", "https://matrix.example.org").unwrap(),
            )
            .await;
        owner.begin_install().await.publish(Session(1)).await;
        assert!(owner.prepare_store_recovery_confirmation().await.is_err());
    }

    #[tokio::test]
    async fn generation_bound_acceptance_follows_the_installed_session() {
        let owner = SessionLifecycleOwner::<Session>::new();
        assert_eq!(owner.with_session_generation(1, || async { 1 }).await, None);
        owner.begin_install().await.publish(Session(2)).await;
        assert_eq!(
            owner.with_session_generation(2, || async { 2 }).await,
            Some(2)
        );
        assert_eq!(owner.with_session_generation(1, || async { 1 }).await, None);
    }

    #[tokio::test]
    async fn maintenance_retries_cleanup_then_retires_and_backs_off_saves() {
        let owner = SessionLifecycleOwner::<Session>::new();
        owner.record_pending_cleanup(locator());
        let retires = AtomicUsize::new(0);
        let now = Instant::now();
        let report = owner
            .maintenance_tick(
                now,
                || async { Some(false) },
                |pending: &SessionLocator| {
                    assert_eq!(pending, &locator());
                    Ok::<_, SessionFault>(())
                },
                Some(RejectedGeneration {
                    generation: 3,
                    core_present: true,
                }),
                |_| {},
                |generation| {
                    assert_eq!(generation, 3);
                    retires.fetch_add(1, Ordering::SeqCst);
                    async { Ok::<(), SessionFault>(()) }
                },
            )
            .await;
        assert_eq!(
            report,
            MaintenanceReport {
                saved: Some(false),
                cleaned: Some(true),
                retired: Some(true),
            }
        );
        assert!(!owner.has_pending_cleanup());
        assert_eq!(retires.load(Ordering::SeqCst), 1);
        // A failed save backs off: the next immediate tick does not retry it.
        assert!(!owner.save_retry_due(Instant::now()));
    }

    #[tokio::test]
    async fn missing_core_logs_once_and_does_not_claim_retirement() {
        let owner = SessionLifecycleOwner::<Session>::new();
        let mut lines = Vec::new();
        for _ in 0..2 {
            let report = owner
                .maintenance_tick(
                    Instant::now(),
                    || async { None },
                    |_: &SessionLocator| Ok::<_, SessionFault>(()),
                    Some(RejectedGeneration {
                        generation: 5,
                        core_present: false,
                    }),
                    |line| lines.push(line.to_owned()),
                    |_| async { panic!("no Core to retire with") },
                )
                .await;
            assert_eq!(report.retired, Some(false));
        }
        assert_eq!(
            lines,
            vec![
                super::super::SESSION_AUTHENTICATION_REJECTED_LOG_LINE.to_owned(),
                super::super::SESSION_REJECTION_NO_CORE_DIAGNOSTIC_ID.to_owned(),
            ]
        );
    }
}
