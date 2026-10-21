//! Restart-on-exit supervision for long-lived session owner tasks.
//!
//! An owner task that returns early (a failed SDK call before its loop) or
//! panics (tokio swallows the panic into the `JoinHandle`) used to stop its
//! updates for the rest of the session with nothing observing the handle.
//! [`spawn_supervised`] runs the task again with doubling back-off and records
//! each exit so callers can report it.

use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::task::JoinHandle;

/// Back-off between restarts. A run that lasted at least `max` resets the
/// delay, so a task that fails once a day restarts promptly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestartPolicy {
    pub initial: Duration,
    pub max: Duration,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self {
            initial: Duration::from_secs(1),
            max: Duration::from_secs(60),
        }
    }
}

/// How one run of a supervised task ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskExit {
    /// The task's future completed, which a long-lived owner never should.
    Returned,
    Panicked,
}

/// Exits observed so far. Shared with the supervisor loop.
#[derive(Debug, Default)]
pub struct TaskHealth {
    restarts: AtomicU64,
    last_exit: Mutex<Option<TaskExit>>,
}

impl TaskHealth {
    fn record(&self, exit: TaskExit) {
        self.restarts.fetch_add(1, Ordering::AcqRel);
        if let Ok(mut last) = self.last_exit.lock() {
            *last = Some(exit);
        }
    }

    /// Number of times the task exited and was started again.
    pub fn restarts(&self) -> u64 {
        self.restarts.load(Ordering::Acquire)
    }

    pub fn last_exit(&self) -> Option<TaskExit> {
        self.last_exit.lock().ok().and_then(|last| *last)
    }
}

/// Handle to a supervised task. Dropping or aborting it stops the current
/// run as well as the supervisor.
pub struct SupervisedTask {
    supervisor: JoinHandle<()>,
    health: Arc<TaskHealth>,
}

impl SupervisedTask {
    pub fn health(&self) -> &Arc<TaskHealth> {
        &self.health
    }

    pub fn abort(&self) {
        self.supervisor.abort();
    }
}

impl Drop for SupervisedTask {
    fn drop(&mut self) {
        self.supervisor.abort();
    }
}

/// Aborts the current run when the supervisor itself is aborted; awaiting a
/// `JoinHandle` does not cancel the task behind it.
struct AbortOnDrop(JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Run `make()` on the runtime, and run it again whenever it returns or
/// panics, waiting `policy.initial` doubling to `policy.max` in between.
pub fn spawn_supervised<F, Fut>(policy: RestartPolicy, mut make: F) -> SupervisedTask
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let health = Arc::new(TaskHealth::default());
    let task_health = Arc::clone(&health);
    let supervisor = tokio::spawn(async move {
        let mut delay = policy.initial;
        loop {
            let started = tokio::time::Instant::now();
            let mut run = AbortOnDrop(tokio::spawn(make()));
            let exit = match (&mut run.0).await {
                Ok(()) => TaskExit::Returned,
                Err(error) if error.is_panic() => TaskExit::Panicked,
                // Cancelled from outside the supervisor; nothing to restart.
                Err(_) => return,
            };
            task_health.record(exit);
            if started.elapsed() >= policy.max {
                delay = policy.initial;
            }
            tokio::time::sleep(delay).await;
            delay = (delay * 2).min(policy.max);
        }
    });
    SupervisedTask { supervisor, health }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    fn fast() -> RestartPolicy {
        RestartPolicy {
            initial: Duration::from_millis(10),
            max: Duration::from_millis(40),
        }
    }

    async fn wait_until(mut done: impl FnMut() -> bool) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !done() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("condition within 5 s");
    }

    #[tokio::test]
    async fn an_early_return_is_restarted_and_recorded() {
        let runs = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&runs);
        let task = spawn_supervised(fast(), move || {
            let counter = Arc::clone(&counter);
            async move {
                // The first two runs fail before their loop; the third stays up.
                if counter.fetch_add(1, Ordering::AcqRel) >= 2 {
                    std::future::pending::<()>().await;
                }
            }
        });
        wait_until(|| runs.load(Ordering::Acquire) == 3).await;
        assert_eq!(task.health().restarts(), 2);
        assert_eq!(task.health().last_exit(), Some(TaskExit::Returned));
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(runs.load(Ordering::Acquire), 3, "a live run is left alone");
    }

    #[tokio::test]
    async fn a_panic_is_restarted_and_recorded() {
        let runs = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&runs);
        let task = spawn_supervised(fast(), move || {
            let counter = Arc::clone(&counter);
            async move {
                if counter.fetch_add(1, Ordering::AcqRel) == 0 {
                    panic!("owner task bug");
                }
                std::future::pending::<()>().await;
            }
        });
        wait_until(|| runs.load(Ordering::Acquire) == 2).await;
        assert_eq!(task.health().restarts(), 1);
        assert_eq!(task.health().last_exit(), Some(TaskExit::Panicked));
    }

    #[tokio::test]
    async fn dropping_the_handle_stops_the_current_run() {
        struct Alive(Arc<AtomicUsize>);
        impl Drop for Alive {
            fn drop(&mut self) {
                self.0.fetch_sub(1, Ordering::AcqRel);
            }
        }
        let alive = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&alive);
        let task = spawn_supervised(fast(), move || {
            let counter = Arc::clone(&counter);
            async move {
                counter.fetch_add(1, Ordering::AcqRel);
                let _alive = Alive(counter);
                std::future::pending::<()>().await;
            }
        });
        wait_until(|| alive.load(Ordering::Acquire) == 1).await;
        drop(task);
        wait_until(|| alive.load(Ordering::Acquire) == 0).await;
    }
}
