//! Deny-by-default widget capability policy.
//!
//! v1 never grants OpenID-adjacent to-device, delayed events, `download_file`,
//! or `org.matrix.msc4515.rtc_transports`. Send is limited to `m.room.message`
//! after an explicit per-widget grant.

use matrix_sdk::widget::{Capabilities, Filter, MessageLikeEventFilter};

/// Persisted / presenter-approved grants for one widget origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WidgetGrantPolicy {
    /// Receive timeline/state the user already sees (never to-device).
    pub receive_room: bool,
    /// Send `m.room.message` only.
    pub send_room_message: bool,
}

fn is_room_message_send_filter(filter: &Filter) -> bool {
    match filter {
        Filter::MessageLike(MessageLikeEventFilter::WithType(event_type)) => {
            event_type.to_string() == "m.room.message"
        }
        Filter::MessageLike(MessageLikeEventFilter::RoomMessageWithMsgtype(_)) => true,
        _ => false,
    }
}

fn is_to_device_filter(filter: &Filter) -> bool {
    matches!(filter, Filter::ToDevice(_))
}

/// Intersect requested SDK capabilities with the v1 product policy.
pub fn filter_requested_capabilities(
    requested: Capabilities,
    policy: &WidgetGrantPolicy,
) -> Capabilities {
    let read = if policy.receive_room {
        requested
            .read
            .into_iter()
            .filter(|filter| !is_to_device_filter(filter))
            .collect()
    } else {
        Vec::new()
    };
    let send = if policy.send_room_message {
        requested
            .send
            .into_iter()
            .filter(is_room_message_send_filter)
            .collect()
    } else {
        Vec::new()
    };
    Capabilities {
        read,
        send,
        requires_client: requested.requires_client,
        update_delayed_event: false,
        send_delayed_event: false,
        download_file: false,
        rtc_transports: false,
    }
}

pub struct SynaraWidgetCapabilitiesProvider {
    policy: WidgetGrantPolicy,
}

impl SynaraWidgetCapabilitiesProvider {
    pub fn new(policy: WidgetGrantPolicy) -> Self {
        Self { policy }
    }
}

impl matrix_sdk::widget::CapabilitiesProvider for SynaraWidgetCapabilitiesProvider {
    async fn acquire_capabilities(&self, requested: Capabilities) -> Capabilities {
        filter_requested_capabilities(requested, &self.policy)
    }
}
