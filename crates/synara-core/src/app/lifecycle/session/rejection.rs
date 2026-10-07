//! Retirement policy for a session whose refresh token the homeserver rejected.

use std::future::Future;

use super::HasDiagnosticId;
use crate::app::sync::SYNC_AUTHENTICATION_FAILURE_DIAGNOSTIC_ID;

/// Fixed lifecycle log line written once per rejected generation.
pub const SESSION_AUTHENTICATION_REJECTED_LOG_LINE: &str = "session-authentication-rejected";

/// Logged once per generation when a rejection is seen but Core is absent, so
/// retirement cannot run and must not be reported as done.
pub const SESSION_REJECTION_NO_CORE_DIAGNOSTIC_ID: &str = "d0.1-session-rejection-no-core";

/// Per-generation log de-duplication for the rejection watcher. Concurrency
/// with a user logout is the shell's transition gate, not this struct's: every
/// tick that still sees the rejected generation retries retirement.
#[derive(Debug, Default)]
pub struct AuthenticationRejectionWatch {
    logged_rejection: Option<u64>,
    logged_missing_core: Option<u64>,
    logged_failure: Option<(u64, String)>,
}

impl AuthenticationRejectionWatch {
    pub fn should_log_rejection(&mut self, generation: u64) -> bool {
        if self.logged_rejection == Some(generation) {
            return false;
        }
        self.logged_rejection = Some(generation);
        true
    }

    pub fn should_log_missing_core(&mut self, generation: u64) -> bool {
        if self.logged_missing_core == Some(generation) {
            return false;
        }
        self.logged_missing_core = Some(generation);
        true
    }

    fn should_log_failure(&mut self, generation: u64, diagnostic_id: &str) -> bool {
        if self
            .logged_failure
            .as_ref()
            .is_some_and(|(logged, id)| *logged == generation && id == diagnostic_id)
        {
            return false;
        }
        self.logged_failure = Some((generation, diagnostic_id.to_owned()));
        true
    }
}

/// One watcher tick for a sync snapshot that reports a rejected refresh.
///
/// `retire` is the local, generation-fenced logout; it is called only when
/// Core is present. Every line passed to `log` is a fixed lifecycle word or a
/// static `d0.1-*` / `p4.1-*` id. Returns whether retirement succeeded.
pub async fn handle_authentication_rejection_tick<E, Retire, RetireFuture>(
    watch: &mut AuthenticationRejectionWatch,
    generation: u64,
    core_present: bool,
    mut log: impl FnMut(&str),
    retire: Retire,
) -> bool
where
    E: HasDiagnosticId,
    Retire: FnOnce(u64) -> RetireFuture,
    RetireFuture: Future<Output = Result<(), E>>,
{
    if watch.should_log_rejection(generation) {
        log(SESSION_AUTHENTICATION_REJECTED_LOG_LINE);
    }
    if !core_present {
        if watch.should_log_missing_core(generation) {
            log(SESSION_REJECTION_NO_CORE_DIAGNOSTIC_ID);
        }
        return false;
    }
    match retire(generation).await {
        Ok(()) => true,
        Err(error) => {
            if let Some(diagnostic_id) = static_rejection_logout_diagnostic(error.diagnostic_id()) {
                if watch.should_log_failure(generation, diagnostic_id) {
                    log(diagnostic_id);
                }
            }
            false
        }
    }
}

/// A logout failure id is logged only when it is already a static session
/// diagnostic. Anything else, including tokens and URLs, is dropped.
pub fn static_rejection_logout_diagnostic(diagnostic_id: &str) -> Option<&str> {
    let static_id = diagnostic_id.starts_with("d0.1-") || diagnostic_id.starts_with("p4.1-");
    let closed_alphabet = diagnostic_id
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '.')
        && diagnostic_id.len() <= 80;
    (static_id && closed_alphabet).then_some(diagnostic_id)
}

/// Voluntary logout may attempt one remote `/logout`. A generation whose
/// refresh was rejected never does, whether a watcher or the user asked.
pub fn remote_logout_allowed(
    expected_session_generation: Option<u64>,
    failure_diagnostic_id: Option<&str>,
) -> bool {
    expected_session_generation.is_none()
        && failure_diagnostic_id != Some(SYNC_AUTHENTICATION_FAILURE_DIAGNOSTIC_ID)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::lifecycle::session::SessionFault;
    use std::cell::Cell;

    #[test]
    fn remote_logout_is_skipped_for_any_rejected_generation() {
        assert!(remote_logout_allowed(None, None));
        assert!(remote_logout_allowed(None, Some("p4-sync-other")));
        assert!(!remote_logout_allowed(Some(3), None));
        assert!(!remote_logout_allowed(
            None,
            Some(SYNC_AUTHENTICATION_FAILURE_DIAGNOSTIC_ID)
        ));
    }

    #[test]
    fn rejection_logout_diagnostics_are_static_ids_only() {
        assert_eq!(
            static_rejection_logout_diagnostic("d0.1-session-rejection-stale"),
            Some("d0.1-session-rejection-stale")
        );
        assert_eq!(
            static_rejection_logout_diagnostic("p4.1-session-authentication-rejected"),
            Some("p4.1-session-authentication-rejected")
        );
        assert_eq!(
            static_rejection_logout_diagnostic("https://private.example/token"),
            None
        );
        assert_eq!(static_rejection_logout_diagnostic("not-a-session-id"), None);
    }

    #[tokio::test]
    async fn rejection_tick_retires_once_logs_once_and_never_claims_success_without_core() {
        let mut watch = AuthenticationRejectionWatch::default();
        let mut lines = Vec::new();
        let retired = Cell::new(0);

        for _ in 0..2 {
            let succeeded = handle_authentication_rejection_tick(
                &mut watch,
                4,
                false,
                |line| lines.push(line.to_owned()),
                |_| async {
                    retired.set(retired.get() + 1);
                    Ok::<(), SessionFault>(())
                },
            )
            .await;
            assert!(!succeeded, "missing Core never claims retirement");
        }
        assert_eq!(retired.get(), 0);
        assert_eq!(
            lines,
            [
                SESSION_AUTHENTICATION_REJECTED_LOG_LINE,
                SESSION_REJECTION_NO_CORE_DIAGNOSTIC_ID
            ]
        );

        lines.clear();
        let failing = |_| async { Err(SessionFault::unavailable("d0.1-session-clear-failed")) };
        assert!(
            !handle_authentication_rejection_tick(
                &mut watch,
                4,
                true,
                |line| lines.push(line.to_owned()),
                failing
            )
            .await
        );
        assert!(
            !handle_authentication_rejection_tick(
                &mut watch,
                4,
                true,
                |line| lines.push(line.to_owned()),
                failing
            )
            .await
        );
        assert_eq!(lines, ["d0.1-session-clear-failed"], "failure logged once");

        lines.clear();
        let unsafe_id = |_| async { Err(SessionFault::unavailable("token=secret")) };
        assert!(
            !handle_authentication_rejection_tick(
                &mut watch,
                4,
                true,
                |line| lines.push(line.to_owned()),
                unsafe_id
            )
            .await
        );
        assert!(lines.is_empty(), "non-static ids are never logged");

        assert!(
            handle_authentication_rejection_tick(
                &mut watch,
                5,
                true,
                |line| lines.push(line.to_owned()),
                |generation| async move {
                    assert_eq!(generation, 5);
                    Ok::<(), SessionFault>(())
                }
            )
            .await
        );
        assert_eq!(lines, [SESSION_AUTHENTICATION_REJECTED_LOG_LINE]);
    }
}
