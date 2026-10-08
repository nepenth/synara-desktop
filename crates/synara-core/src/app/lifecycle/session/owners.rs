//! Canonical order in which a shell attaches session owners to Core.
//!
//! Desktop and iOS build their owners differently (event sinks, start order
//! with subscription side effects), but both hand the finished set to Core in
//! this one order: sync last, so a command never sees a running sync before
//! the owners it drives are attached.

use std::sync::Arc;

use crate::app::account_data::NativeImagePackOwner;
use crate::app::dehydrated_devices::NativeDehydratedDevicesOwner;
use crate::app::devices::NativeDeviceOwner;
use crate::app::notifications::{NativeHttpPusherOwner, NativeNotificationDecisionOwner};
use crate::app::presence::NativePresenceOwner;
use crate::app::room_profile::NativeRoomJoinRuleOwner;
use crate::app::rtc_transports::NativeRtcTransportsOwner;
use crate::app::sync::SyncServiceOwner;
use crate::app::timeline::NativeTimelineOwner;
use crate::app::typing::NativeTypingOwner;
use crate::app::user_status::NativeUserStatusOwner;
use crate::app::verification::NativeVerificationOwner;
use crate::app::widgets::NativeWidgetOwner;
use crate::core::Core;

/// One session's owners, ready to attach. Platform-only owners are optional:
/// desktop has widgets and notification decisions, iOS has the HTTP pusher.
pub struct SessionOwnerSet {
    pub typing: Arc<NativeTypingOwner>,
    pub presence: Arc<NativePresenceOwner>,
    pub rtc_transports: Arc<NativeRtcTransportsOwner>,
    pub user_status: Arc<NativeUserStatusOwner>,
    pub widgets: Option<Arc<NativeWidgetOwner>>,
    pub verification: Arc<NativeVerificationOwner>,
    pub devices: Arc<NativeDeviceOwner>,
    pub dehydrated_devices: Arc<NativeDehydratedDevicesOwner>,
    pub join_rules: Arc<NativeRoomJoinRuleOwner>,
    pub image_packs: Arc<NativeImagePackOwner>,
    pub http_pusher: Option<Arc<NativeHttpPusherOwner>>,
    pub timelines: Arc<NativeTimelineOwner>,
    pub notification_decisions: Option<Arc<NativeNotificationDecisionOwner>>,
    pub sync: Arc<SyncServiceOwner>,
}

/// Which owner failed to attach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerKind {
    Typing,
    Presence,
    RtcTransports,
    UserStatus,
    Widgets,
    Verification,
    Devices,
    DehydratedDevices,
    JoinRules,
    ImagePacks,
    HttpPusher,
    Timelines,
    NotificationDecisions,
    Sync,
}

impl OwnerKind {
    /// Static attach-failure id. Desktop surfaces it; iOS maps every kind to
    /// its own single attach failure.
    pub const fn attach_diagnostic_id(self) -> &'static str {
        match self {
            Self::Typing => "p2-typing-attach-failed",
            Self::Presence => "p2-presence-attach-failed",
            Self::RtcTransports => "p2-rtc-transports-attach-failed",
            Self::UserStatus => "p2-user-status-attach-failed",
            Self::Widgets => "p2-widgets-attach-failed",
            Self::Verification => "p2-verification-attach-failed",
            Self::Devices => "p2-device-attach-failed",
            Self::DehydratedDevices => "p2-dehydrated-devices-attach-failed",
            Self::JoinRules => "p2-join-rule-attach-failed",
            Self::ImagePacks => "p2-image-pack-attach-failed",
            Self::HttpPusher => "p2-http-pusher-attach-failed",
            Self::Timelines => "p2-timeline-attach-failed",
            Self::NotificationDecisions => "p2-notification-decision-attach-failed",
            Self::Sync => "p2-sync-attach-failed",
        }
    }
}

/// The attach order, exposed for tests and documentation.
pub const OWNER_ATTACH_ORDER: [OwnerKind; 14] = [
    OwnerKind::Typing,
    OwnerKind::Presence,
    OwnerKind::RtcTransports,
    OwnerKind::UserStatus,
    OwnerKind::Widgets,
    OwnerKind::Verification,
    OwnerKind::Devices,
    OwnerKind::DehydratedDevices,
    OwnerKind::JoinRules,
    OwnerKind::ImagePacks,
    OwnerKind::HttpPusher,
    OwnerKind::Timelines,
    OwnerKind::NotificationDecisions,
    OwnerKind::Sync,
];

/// Attach every owner in [`OWNER_ATTACH_ORDER`]; stop at the first failure.
pub fn attach_owner_set(core: &Core, set: SessionOwnerSet) -> Result<(), OwnerKind> {
    let SessionOwnerSet {
        typing,
        presence,
        rtc_transports,
        user_status,
        widgets,
        verification,
        devices,
        dehydrated_devices,
        join_rules,
        image_packs,
        http_pusher,
        timelines,
        notification_decisions,
        sync,
    } = set;
    let step = |kind: OwnerKind, result: Result<(), crate::transport::MatrixIpcError>| {
        result.map_err(|_| kind)
    };
    step(OwnerKind::Typing, core.attach_typing(typing))?;
    step(OwnerKind::Presence, core.attach_presence(presence))?;
    step(
        OwnerKind::RtcTransports,
        core.attach_rtc_transports(rtc_transports),
    )?;
    step(OwnerKind::UserStatus, core.attach_user_status(user_status))?;
    if let Some(widgets) = widgets {
        step(OwnerKind::Widgets, core.attach_widgets(widgets))?;
    }
    step(
        OwnerKind::Verification,
        core.attach_verification(verification),
    )?;
    step(OwnerKind::Devices, core.attach_devices(devices))?;
    step(
        OwnerKind::DehydratedDevices,
        core.attach_dehydrated_devices(dehydrated_devices),
    )?;
    step(OwnerKind::JoinRules, core.attach_join_rules(join_rules))?;
    step(OwnerKind::ImagePacks, core.attach_image_packs(image_packs))?;
    if let Some(http_pusher) = http_pusher {
        step(OwnerKind::HttpPusher, core.attach_http_pusher(http_pusher))?;
    }
    step(OwnerKind::Timelines, core.attach_timelines(timelines))?;
    if let Some(decisions) = notification_decisions {
        step(
            OwnerKind::NotificationDecisions,
            core.attach_notification_decisions(decisions),
        )?;
    }
    step(OwnerKind::Sync, core.attach_sync(sync))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_attaches_last_and_ids_are_unique() {
        assert_eq!(OWNER_ATTACH_ORDER.last(), Some(&OwnerKind::Sync));
        let mut ids: Vec<_> = OWNER_ATTACH_ORDER
            .iter()
            .map(|kind| kind.attach_diagnostic_id())
            .collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), OWNER_ATTACH_ORDER.len());
    }

    #[test]
    fn attach_source_follows_the_published_order() {
        let source = include_str!("owners.rs");
        let body = source
            .split("pub fn attach_owner_set")
            .nth(1)
            .and_then(|rest| rest.split("#[cfg(test)]").next())
            .unwrap();
        let mut last = 0;
        for kind in OWNER_ATTACH_ORDER {
            let needle = format!("OwnerKind::{kind:?},");
            let needle_inline = format!("OwnerKind::{kind:?}, core.");
            let index = body
                .find(&needle_inline)
                .or_else(|| body.find(&needle))
                .unwrap_or_else(|| panic!("{kind:?} is attached"));
            assert!(index > last, "{kind:?} keeps its place");
            last = index;
        }
    }
}
