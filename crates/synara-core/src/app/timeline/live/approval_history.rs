//! Inbox history shares the SDK room event cache with open views. A view
//! protects that cache before opening, stops further inbox pages, and waits
//! for any SDK-owned pagination request to settle.
use super::*;
use tokio::sync::watch;

#[derive(Default)]
pub(super) struct ApprovalHistory {
    rooms: std::sync::Mutex<HashMap<String, Weak<RoomHistory>>>,
}
impl ApprovalHistory {
    pub(super) fn room(&self, room_id: &str) -> Result<Arc<RoomHistory>, &'static str> {
        let mut rooms = self
            .rooms
            .lock()
            .map_err(|_| "approval-history-registry-poisoned")?;
        rooms.retain(|_, room| room.strong_count() > 0);
        if let Some(room) = rooms.get(room_id).and_then(Weak::upgrade) {
            return Ok(room);
        }
        let room = Arc::new(RoomHistory {
            protection: watch::channel(0).0,
            operation: AsyncMutex::new(()),
        });
        rooms.insert(room_id.to_owned(), Arc::downgrade(&room));
        Ok(room)
    }
}

pub(super) struct RoomHistory {
    protection: watch::Sender<usize>,
    operation: AsyncMutex<()>,
}
impl RoomHistory {
    pub(super) fn protected(&self) -> bool {
        *self.protection.borrow() > 0
    }
    pub(super) fn subscribe(&self) -> watch::Receiver<usize> {
        self.protection.subscribe()
    }

    pub(super) async fn protect(self: &Arc<Self>, room: &Room) -> HistoryProtection {
        // Construct the guard before awaiting so cancelled opens release their
        // protection, too. Concurrent views are reference-counted.
        self.protection.send_modify(|count| *count += 1);
        let guard = HistoryProtection(self.clone());
        let _quiescent = self.operation.lock().await;
        // SDK 0.19 retains a spawned shared pagination task after its caller
        // is dropped. Wait for the *cache's* status (shared with
        // `BackPaginationQueue`), not just our caller's mutex. Waiting is
        // correct; failing the user's room open is not. After the timeout
        // the view proceeds and the inbox defers while this guard lives.
        let _ = timeout(Duration::from_secs(10), wait_for_cache_pagination(room)).await;
        guard
    }

    pub(super) async fn run(&self, operation: impl std::future::Future<Output = ()>) -> bool {
        let mut changes = self.subscribe();
        if self.protected() {
            return false;
        }
        let _exclusive = self.operation.lock().await;
        if self.protected() {
            return false;
        }
        tokio::pin!(operation);
        loop {
            tokio::select! {
                biased;
                result = changes.changed() => {
                    if result.is_err() || self.protected() { return false; }
                }
                _ = &mut operation => return true,
            }
        }
    }
}

async fn wait_for_cache_pagination(room: &Room) -> Result<(), &'static str> {
    let client = room.client();
    let event_cache = client.event_cache();
    event_cache
        .subscribe()
        .map_err(|_| "approval-history-cache-unavailable")?;
    // User/inbox `paginate_backwards` and automatic latest-event backfill
    // share this room cache status (and `BackPaginationQueue` when enabled).
    let _shared_queue = event_cache.back_pagination_queue();
    let (cache, _subscription) = room
        .event_cache()
        .await
        .map_err(|_| "approval-history-cache-unavailable")?;
    let mut status = cache.pagination().status();
    while matches!(status.next_now(), PaginationStatus::Paginating) {
        status
            .next()
            .await
            .ok_or("approval-history-status-unavailable")?;
    }
    Ok(())
}

pub(super) struct HistoryProtection(Arc<RoomHistory>);
impl Drop for HistoryProtection {
    fn drop(&mut self) {
        self.0.protection.send_modify(|count| *count -= 1);
    }
}
