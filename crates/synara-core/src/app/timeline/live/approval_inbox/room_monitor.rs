//! One cheap session subscription maintains cached permission and invalidates
//! paused history. It owns no SDK timeline and never performs pagination.
use super::*;
use matrix_sdk::ruma::OwnedRoomId;
use matrix_sdk::sync::State;
use tokio::sync::{broadcast, mpsc};

type States = Arc<std::sync::Mutex<HashMap<String, Weak<std::sync::Mutex<RoomSnapshot>>>>>;
pub(super) struct RoomMonitor {
    states: States,
    initial: mpsc::UnboundedSender<(Room, Weak<std::sync::Mutex<RoomSnapshot>>)>,
    task: JoinHandle<()>,
}
impl Drop for RoomMonitor {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl RoomMonitor {
    pub(super) fn new(client: &Client) -> Self {
        let states: States = Default::default();
        let shared = states.clone();
        let client = client.clone();
        let mut updates = client.subscribe_to_all_room_updates();
        let (initial, mut requests) =
            mpsc::unbounded_channel::<(Room, Weak<std::sync::Mutex<RoomSnapshot>>)>();
        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    request = requests.recv() => {
                        let Some((room, state)) = request else { break; };
                        refresh_permission(&room, state).await;
                    }
                    update = updates.recv() => {
                        let batch = match update {
                            Ok(batch) => batch,
                            Err(broadcast::error::RecvError::Closed) => break,
                            Err(broadcast::error::RecvError::Lagged(_)) => {
                                let rooms: Vec<_> = match shared.lock() {
                                    Ok(states) => states
                                        .iter()
                                        .map(|(id, state)| (id.clone(), state.clone()))
                                        .collect(),
                                    Err(_) => continue,
                                };
                                for (id, weak) in rooms {
                                    if let Some(state) = weak.upgrade() {
                                        if let Ok(mut state) = state.lock() {
                                            state.checked = false;
                                            state.can_send_reaction = false;
                                        }
                                    }
                                    if let Ok(id) = id.parse::<OwnedRoomId>() {
                                        if let Some(room) = client.get_room(&id) { refresh_permission(&room, weak).await; }
                                    }
                                }
                                continue;
                            }
                        };
                        for id in batch.left.keys().chain(batch.invited.keys()).chain(batch.knocked.keys()) {
                            let Some(state) = (match shared.lock() {
                                Ok(states) => states.get(id.as_str()).and_then(Weak::upgrade),
                                Err(_) => continue,
                            }) else {
                                continue;
                            };
                            {
                                let Ok(mut state) = state.lock() else { continue; };
                                state.can_send_reaction = false;
                                state.checked = false;
                            }
                        }
                        for (id, update) in batch.joined {
                            let Some(weak) = (match shared.lock() {
                                Ok(states) => states.get(id.as_str()).cloned(),
                                Err(_) => continue,
                            }) else {
                                continue;
                            };
                            let Some(state) = weak.upgrade() else { continue; };
                            {
                                let Ok(mut state) = state.lock() else { continue; };
                                if !state.observing && (update.timeline.limited || !update.timeline.events.is_empty()) {
                                    // Cache invalidation changes coverage, not health.
                                    state.checked = false;
                                }
                            }
                            let state_events = match &update.state { State::Before(events) | State::After(events) => events };
                            let permission_changed = state_events.iter().any(|event| permission_event(event.json().get()))
                                || update.timeline.events.iter().any(|event| permission_event(event.raw().json().get()));
                            if permission_changed {
                                // Fail closed before awaiting the native store. Refresh is
                                // outside the inbox-owner and projection locks.
                                if let Ok(mut snapshot) = state.lock() {
                                    snapshot.can_send_reaction = false;
                                }
                                if let Some(room) = client.get_room(&id) { refresh_permission(&room, weak).await; }
                            }
                        }
                    }
                }
            }
        });
        Self {
            states,
            initial,
            task,
        }
    }
    pub(super) fn register(
        &self,
        room: &Room,
        state: &Arc<std::sync::Mutex<RoomSnapshot>>,
    ) -> Result<(), &'static str> {
        let weak = Arc::downgrade(state);
        let mut states = self
            .states
            .lock()
            .map_err(|_| "agent-approval-inbox-state-poisoned")?;
        states.retain(|_, state| state.strong_count() > 0);
        if states
            .insert(room.room_id().to_string(), weak.clone())
            .is_none_or(|old| !Weak::ptr_eq(&old, &weak))
        {
            let _ = self.initial.send((room.clone(), weak));
        }
        Ok(())
    }
}
fn permission_event(raw: &str) -> bool {
    #[derive(Deserialize)]
    struct Kind {
        #[serde(rename = "type")]
        event_type: String,
    }
    serde_json::from_str::<Kind>(raw).is_ok_and(|kind| {
        matches!(
            kind.event_type.as_str(),
            "m.room.power_levels" | "m.room.member" | "m.room.create"
        )
    })
}
async fn refresh_permission(room: &Room, state: Weak<std::sync::Mutex<RoomSnapshot>>) {
    let Some(state) = state.upgrade() else {
        return;
    };
    let can_send = if room.state() == matrix_sdk::RoomState::Joined {
        match (room.power_levels().await, room.client().user_id()) {
            (Ok(levels), Some(user_id)) => levels.user_can_send_message(
                user_id,
                matrix_sdk::ruma::events::MessageLikeEventType::Reaction,
            ),
            _ => false,
        }
    } else {
        false
    };
    {
        let Ok(mut state) = state.lock() else {
            return;
        };
        state.can_send_reaction = can_send;
    }
}
