//! Runtime for the Apple UniFFI async exports.
//!
//! UniFFI polls every `async_runtime = "tokio"` export on a Swift thread
//! through async-compat. Swift threads have no tokio context, so async-compat
//! falls back to one process-wide runtime, and every `tokio::spawn` from the
//! SDK and from Synara's owners (sync loops, decryption, event cache, send
//! queue, timeline updates) runs there. Upstream that runtime is
//! `current_thread`: one stall stops everything, including the renewal of the
//! cross-process crypto-store lease the notification extension also takes.
//!
//! The workspace patches async-compat to the fork matrix-rust-sdk uses, which
//! accepts a builder for that runtime. The builder only takes effect if it is
//! installed before the first async export is polled, so every synchronous
//! FFI constructor installs it. Desktop runs inside Tauri's multi-threaded
//! runtime, so async-compat never creates the fallback there.

use tokio::runtime::Builder;

/// Worker threads for the iOS app process.
pub const APP_MAX_WORKER_THREADS: usize = 4;
/// Worker threads for the notification service extension, which has a small
/// memory ceiling and resolves one notification at a time.
pub const NSE_WORKER_THREADS: usize = 2;

/// A multi-threaded runtime builder with between two and `max_workers`
/// workers, bounded by the device's available parallelism.
///
/// Two workers is the floor so a blocking callback or a long decryption burst
/// on one worker cannot stop the others. Blocking work (Keychain, files,
/// SQLite) still belongs on `spawn_blocking`.
pub fn ffi_runtime_builder(max_workers: usize) -> Builder {
    let max_workers = max_workers.max(2);
    let workers = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(2)
        .clamp(2, max_workers);
    let mut builder = Builder::new_multi_thread();
    builder
        .enable_all()
        .worker_threads(workers)
        .thread_name("synara-ffi");
    builder
}

/// Run a synchronous Keychain, keyring or file call that cannot be awaited,
/// such as one inside an SDK callback.
///
/// On a multi-threaded runtime worker it uses `block_in_place`, so the
/// runtime moves the worker's other tasks to another thread first. Anywhere
/// else (a Swift thread polling an export, a blocking-pool thread, no
/// runtime) it simply runs `operation`, which is already off the workers;
/// a `current_thread` runtime cannot hand its tasks over, so it runs there
/// as before.
pub fn block_in_place_if_worker<T>(operation: impl FnOnce() -> T) -> T {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => {
            tokio::task::block_in_place(operation)
        }
        _ => operation(),
    }
}

/// Run blocking Keychain, file or SQLite-adjacent work on the blocking pool
/// and await it, so it holds neither a runtime worker nor the Swift thread
/// that polls the export. Without a runtime (plain unit tests) it runs
/// inline. `None` only if the closure panicked.
pub async fn run_blocking<T: Send + 'static>(
    operation: impl FnOnce() -> T + Send + 'static,
) -> Option<T> {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle.spawn_blocking(operation).await.ok(),
        Err(_) => Some(operation()),
    }
}

/// Install the app's async-compat runtime builder. Idempotent; a no-op once
/// any async export has run.
#[cfg(feature = "full-uniffi")]
pub(crate) fn install_app_ffi_runtime() {
    static INSTALL: std::sync::Once = std::sync::Once::new();
    INSTALL.call_once(|| {
        uniffi::deps::async_compat::set_runtime_builder(Box::new(|| {
            ffi_runtime_builder(APP_MAX_WORKER_THREADS)
        }));
    });
}

#[cfg(all(test, feature = "full-uniffi"))]
mod tests {
    use std::collections::HashSet;
    use std::future::Future;
    use std::pin::pin;
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};
    use std::thread;
    use std::time::{Duration, Instant};

    use uniffi::deps::async_compat::Compat;

    /// Poll a future the way a Swift continuation does: on a plain thread
    /// with no tokio context, woken through a foreign waker.
    fn poll_like_swift<F: Future>(future: F) -> F::Output {
        struct Unpark(thread::Thread);
        impl Wake for Unpark {
            fn wake(self: Arc<Self>) {
                self.0.unpark();
            }
        }
        let waker = Waker::from(Arc::new(Unpark(thread::current())));
        let mut context = Context::from_waker(&waker);
        let mut future = pin!(Compat::new(future));
        loop {
            if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
                return output;
            }
            thread::park();
        }
    }

    #[test]
    fn exports_spawn_onto_a_multi_threaded_runtime() {
        super::install_app_ffi_runtime();
        let (flavor, workers) = thread::spawn(|| {
            poll_like_swift(async {
                let flavor = tokio::runtime::Handle::current().runtime_flavor();
                // Each task blocks its worker briefly, so they can only all
                // finish together if they run on different threads.
                let tasks = (0..4)
                    .map(|_| {
                        tokio::spawn(async {
                            thread::sleep(Duration::from_millis(100));
                            thread::current().id()
                        })
                    })
                    .collect::<Vec<_>>();
                let mut workers = HashSet::new();
                for task in tasks {
                    workers.insert(task.await.unwrap());
                }
                (flavor, workers)
            })
        })
        .join()
        .unwrap();
        assert_eq!(flavor, tokio::runtime::RuntimeFlavor::MultiThread);
        assert!(
            workers.len() >= 2,
            "spawned SDK work must not share one OS thread"
        );
    }

    #[test]
    fn blocking_helpers_run_off_the_async_workers_on_every_runtime_flavor() {
        // A multi-threaded worker hands its other tasks over first.
        let multi = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let (inline, pooled) = multi.block_on(async {
            tokio::spawn(async {
                let worker = thread::current().id();
                let inline = super::block_in_place_if_worker(|| thread::current().id());
                let pooled = super::run_blocking(|| thread::current().id())
                    .await
                    .unwrap();
                (inline == worker, pooled != worker)
            })
            .await
            .unwrap()
        });
        assert!(inline, "block_in_place keeps the call on the same thread");
        assert!(pooled, "run_blocking moves the call to the blocking pool");

        // A current_thread runtime would panic in block_in_place.
        let current = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let value = current.block_on(async {
            let inline = super::block_in_place_if_worker(|| 7);
            inline + super::run_blocking(|| 1).await.unwrap()
        });
        assert_eq!(value, 8);

        // No runtime at all, and a Swift-polled export.
        assert_eq!(super::block_in_place_if_worker(|| 3), 3);
        super::install_app_ffi_runtime();
        let swift = thread::spawn(|| {
            poll_like_swift(async {
                let poller = thread::current().id();
                let inline = super::block_in_place_if_worker(|| thread::current().id());
                let pooled = super::run_blocking(|| thread::current().id())
                    .await
                    .unwrap();
                (inline == poller, pooled != poller)
            })
        })
        .join()
        .unwrap();
        assert_eq!(swift, (true, true));
    }

    #[test]
    fn one_blocked_export_does_not_stall_another() {
        super::install_app_ffi_runtime();
        // A synchronous stall on a runtime thread, like a Keychain callback or
        // a large sync response being parsed.
        let stalled = thread::spawn(|| {
            poll_like_swift(async {
                tokio::spawn(async { thread::sleep(Duration::from_millis(1500)) })
                    .await
                    .unwrap();
            })
        });
        thread::sleep(Duration::from_millis(100));
        let started = Instant::now();
        thread::spawn(|| {
            poll_like_swift(async {
                tokio::spawn(tokio::time::sleep(Duration::from_millis(20)))
                    .await
                    .unwrap();
            })
        })
        .join()
        .unwrap();
        assert!(
            started.elapsed() < Duration::from_millis(1000),
            "a second export waited {:?} behind a stalled runtime thread",
            started.elapsed()
        );
        stalled.join().unwrap();
    }
}
