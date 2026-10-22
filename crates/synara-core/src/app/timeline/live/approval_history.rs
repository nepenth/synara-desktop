//! Inbox history shares the SDK room event cache with open views. A view
//! protects that cache before opening, which stops further inbox pages. It
//! does not wait for an SDK-owned page already in flight: that page only adds
//! older events the view would page in anyway, and waiting made opening an
//! unread room wait on unrelated background backfill.
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

    /// Protect the room's history for one view. Concurrent views are
    /// reference-counted; a running inbox operation sees the change and stops.
    pub(super) fn protect(self: &Arc<Self>) -> HistoryProtection {
        self.protection.send_modify(|count| *count += 1);
        HistoryProtection(self.clone())
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

pub(super) struct HistoryProtection(Arc<RoomHistory>);
impl Drop for HistoryProtection {
    fn drop(&mut self) {
        self.0.protection.send_modify(|count| *count -= 1);
    }
}
