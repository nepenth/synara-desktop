//! Session-scoped send-queue recovery.
//!
//! matrix-sdk 0.19 disables a room's send queue after *any* send error
//! (`send_queue/mod.rs` `sending_task`). `wait_for_queued_send` re-enables it
//! once; if the SDK's retry fails again, nothing re-enables the room and the
//! request sits until the room queue is next touched — typically the next
//! launch, where it would send long after the user stopped expecting it.
//!
//! This owner re-enables the send queue with bounded backoff while sync is
//! running, and immediately when sync recovers. Wedged (unrecoverable)
//! requests stay wedged: the SDK skips them until the user picks Retry
//! (unwedge) or Discard (abort) on the timeline row.

use std::time::Duration;

use futures_util::{Stream, StreamExt};
use matrix_sdk::Client;
use tokio::sync::broadcast::error::RecvError;
use tokio::time::Instant;

const FIRST_RETRY: Duration = Duration::from_secs(5);
const MAX_RETRY: Duration = Duration::from_secs(60);

/// Delay before re-enabling after the `failures`-th consecutive recoverable
/// error: 5 s, 10 s, 20 s, 40 s, then 60 s.
pub fn send_queue_retry_delay(failures: u32) -> Duration {
    let exponent = failures.saturating_sub(1).min(4);
    (FIRST_RETRY * 2u32.pow(exponent)).min(MAX_RETRY)
}

/// Spawn the recovery owner for one session generation. `sync_running`
/// yields whether sync is currently running; the task ends when it closes or
/// when the returned handle is aborted (logout / generation change).
pub fn spawn_send_queue_recovery<S>(client: Client, sync_running: S) -> tokio::task::JoinHandle<()>
where
    S: Stream<Item = bool> + Send + Unpin + 'static,
{
    // Subscribe before spawning so an error raised meanwhile is not missed.
    let mut errors = client.send_queue().subscribe_errors();
    tokio::spawn(async move {
        let mut sync_running = sync_running;
        let mut running = false;
        let mut failures: u32 = 0;
        let mut retry_at: Option<Instant> = None;
        loop {
            let retry = async {
                match retry_at {
                    Some(at) => tokio::time::sleep_until(at).await,
                    None => std::future::pending().await,
                }
            };
            tokio::select! {
                state = sync_running.next() => {
                    let Some(now_running) = state else { break };
                    if now_running && !running {
                        // Sync recovered: wake every room queue a send error
                        // disabled, and respawn rooms with persisted requests.
                        client.send_queue().set_enabled(true).await;
                        failures = 0;
                        retry_at = None;
                    }
                    running = now_running;
                }
                error = errors.recv() => match error {
                    Ok(error) if error.is_recoverable => {
                        failures = failures.saturating_add(1);
                        retry_at = Some(Instant::now() + send_queue_retry_delay(failures));
                    }
                    // Unrecoverable: wedged until the user retries or discards.
                    Ok(_) => {}
                    Err(RecvError::Lagged(_)) => {
                        failures = failures.saturating_add(1);
                        retry_at = Some(Instant::now() + send_queue_retry_delay(failures));
                    }
                    Err(RecvError::Closed) => break,
                },
                () = retry => {
                    retry_at = None;
                    // While sync is down the recovery edge above re-enables.
                    if running {
                        client.send_queue().set_enabled(true).await;
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_delay_backs_off_and_caps() {
        let delays: Vec<u64> = (1..=7)
            .map(|failures| send_queue_retry_delay(failures).as_secs())
            .collect();
        assert_eq!(delays, vec![5, 10, 20, 40, 60, 60, 60]);
        assert_eq!(send_queue_retry_delay(0), FIRST_RETRY);
    }
}
