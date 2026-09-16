//! Aggregate badge / tab counts from a room-list projection (P4.3).
//!
//! Privacy-safe counters only — no room ids, tokens, or display names.

use serde::{Deserialize, Serialize};

use crate::dto::{Membership, RoomSummary};

use super::filters::{RoomListScope, room_matches_scope};

/// Closed membership input for the scalar room-unread projection exported to
/// iOS. It intentionally carries no room identifier or SDK value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomUnreadMembership {
    Joined,
    Invited,
}

/// Privacy-safe per-row unread presentation.
///
/// The full-width count comes directly from the authoritative platform SDK
/// observation. This pure projection does not retain state or perform I/O.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomUnreadPresentationDto {
    pub unread_count: u64,
    pub has_highlight: bool,
}

/// Derive the iOS row presentation from scalar SDK observations.
///
/// Invites always receive the same visible attention state. Joined rooms use
/// the larger canonical unread source, preserve marked-unread at zero, and
/// expose a highlight only when a mention exists. No arithmetic can overflow.
pub fn room_unread_presentation(
    membership: RoomUnreadMembership,
    num_unread_messages: u64,
    num_unread_notifications: u64,
    num_unread_mentions: u64,
    is_marked_unread: bool,
) -> RoomUnreadPresentationDto {
    match membership {
        RoomUnreadMembership::Invited => RoomUnreadPresentationDto {
            unread_count: 1,
            has_highlight: true,
        },
        RoomUnreadMembership::Joined => {
            let unread_count = num_unread_messages.max(num_unread_notifications);
            RoomUnreadPresentationDto {
                unread_count: if is_marked_unread && unread_count == 0 {
                    1
                } else {
                    unread_count
                },
                has_highlight: num_unread_mentions > 0,
            }
        }
    }
}

/// Tab / nav badge counters derived from room summaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomListBadgeCounts {
    pub joined: u32,
    pub invites: u32,
    pub unread_rooms: u32,
    pub mention_rooms: u32,
    pub direct: u32,
    /// Sum of per-room unread_count across joined rooms (saturates at u32::MAX).
    pub unread_messages: u32,
    /// Sum of per-room highlight_count across joined rooms.
    pub highlight_messages: u32,
    /// Joined rooms with marked_unread set.
    pub marked_unread_rooms: u32,
}

impl RoomListBadgeCounts {
    pub fn from_rooms(rooms: &[RoomSummary]) -> Self {
        let mut out = Self::default();
        for room in rooms {
            match room.membership {
                Membership::Join => {
                    out.joined = out.joined.saturating_add(1);
                    if room.is_direct {
                        out.direct = out.direct.saturating_add(1);
                    }
                    if room_matches_scope(room, RoomListScope::Unread) {
                        out.unread_rooms = out.unread_rooms.saturating_add(1);
                    }
                    if room.highlight_count > 0 {
                        out.mention_rooms = out.mention_rooms.saturating_add(1);
                    }
                    if room.marked_unread {
                        out.marked_unread_rooms = out.marked_unread_rooms.saturating_add(1);
                    }
                    out.unread_messages = out.unread_messages.saturating_add(room.unread_count);
                    out.highlight_messages =
                        out.highlight_messages.saturating_add(room.highlight_count);
                }
                Membership::Invite => {
                    out.invites = out.invites.saturating_add(1);
                }
                Membership::Knock | Membership::Leave | Membership::Ban => {}
            }
        }
        out
    }

    /// Home-tab attention: unread-scope joined rooms + open invites.
    pub fn attention_total(self) -> u32 {
        self.unread_rooms.saturating_add(self.invites)
    }
}

/// One joined-room observation for the SDK `total_unread_notifications` fold.
///
/// This is **not** the dock badge. SDK math is
/// `num_unread_notifications.max(is_marked_unread as u64)` over joined rooms
/// only. It has no mute skip, no space skip, no later-items, no highlight
/// split, and no `max(messages, notifications)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SdkUnreadChecksumRoom {
    pub joined: bool,
    pub num_unread_notifications: u64,
    pub is_marked_unread: bool,
}

/// Synara row presentation inputs used to prove the SDK total cannot replace
/// `room_unread_presentation` / `summarizeNotifications`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SynaraUnreadChecksumRoom {
    pub membership: RoomUnreadMembership,
    pub num_unread_messages: u64,
    pub num_unread_notifications: u64,
    pub num_unread_mentions: u64,
    pub is_marked_unread: bool,
}

/// Pure SDK-style joined-room notification checksum.
pub fn sdk_joined_notification_checksum(
    rooms: impl IntoIterator<Item = SdkUnreadChecksumRoom>,
) -> u64 {
    rooms
        .into_iter()
        .filter(|room| room.joined)
        .map(|room| {
            room.num_unread_notifications
                .max(u64::from(room.is_marked_unread))
        })
        .fold(0, u64::saturating_add)
}

/// Sum of Synara `room_unread_presentation` unread counts for joined rooms.
pub fn synara_joined_unread_presentation_sum(
    rooms: impl IntoIterator<Item = SynaraUnreadChecksumRoom>,
) -> u64 {
    rooms
        .into_iter()
        .filter(|room| room.membership == RoomUnreadMembership::Joined)
        .map(|room| {
            room_unread_presentation(
                room.membership,
                room.num_unread_messages,
                room.num_unread_notifications,
                room.num_unread_mentions,
                room.is_marked_unread,
            )
            .unread_count
        })
        .fold(0, u64::saturating_add)
}

/// Session accessor for the SDK joined-room notification checksum.
///
/// Dock/tray remains a JS fold. This must not write the desktop badge.
pub fn client_total_unread_notifications(client: &matrix_sdk::Client) -> u64 {
    client.total_unread_notifications()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invited_rooms_always_have_attention_without_converting_counters() {
        assert_eq!(
            room_unread_presentation(
                RoomUnreadMembership::Invited,
                u64::MAX,
                u64::MAX,
                u64::MAX,
                true,
            ),
            RoomUnreadPresentationDto {
                unread_count: 1,
                has_highlight: true,
            }
        );
    }

    #[test]
    fn joined_rooms_use_the_larger_message_or_notification_counter() {
        assert_eq!(
            room_unread_presentation(RoomUnreadMembership::Joined, 4, 6, 0, false),
            RoomUnreadPresentationDto {
                unread_count: 6,
                has_highlight: false,
            }
        );
        assert_eq!(
            room_unread_presentation(RoomUnreadMembership::Joined, 7, 3, 0, false),
            RoomUnreadPresentationDto {
                unread_count: 7,
                has_highlight: false,
            }
        );
    }

    #[test]
    fn receipts_that_zero_counts_and_clear_marked_unread_drop_the_badge() {
        assert_eq!(
            room_unread_presentation(RoomUnreadMembership::Joined, 0, 0, 0, false),
            RoomUnreadPresentationDto {
                unread_count: 0,
                has_highlight: false,
            }
        );
    }

    #[test]
    fn marked_unread_only_changes_a_zero_joined_count() {
        assert_eq!(
            room_unread_presentation(RoomUnreadMembership::Joined, 0, 0, 0, true),
            RoomUnreadPresentationDto {
                unread_count: 1,
                has_highlight: false,
            }
        );
        assert_eq!(
            room_unread_presentation(RoomUnreadMembership::Joined, 2, 0, 0, true),
            RoomUnreadPresentationDto {
                unread_count: 2,
                has_highlight: false,
            }
        );
    }

    #[test]
    fn mentions_control_highlight_independently_of_unread_count() {
        assert_eq!(
            room_unread_presentation(RoomUnreadMembership::Joined, 0, 0, 1, false),
            RoomUnreadPresentationDto {
                unread_count: 0,
                has_highlight: true,
            }
        );
    }

    #[test]
    fn joined_projection_preserves_full_width_maximum_counter() {
        assert_eq!(
            room_unread_presentation(RoomUnreadMembership::Joined, u64::MAX, 1, 0, false),
            RoomUnreadPresentationDto {
                unread_count: u64::MAX,
                has_highlight: false,
            }
        );
    }

    #[test]
    fn sdk_checksum_ignores_message_counts_that_synara_badges() {
        let sdk = [SdkUnreadChecksumRoom {
            joined: true,
            num_unread_notifications: 3,
            is_marked_unread: false,
        }];
        let synara = [SynaraUnreadChecksumRoom {
            membership: RoomUnreadMembership::Joined,
            num_unread_messages: 10,
            num_unread_notifications: 3,
            num_unread_mentions: 0,
            is_marked_unread: false,
        }];
        assert_eq!(sdk_joined_notification_checksum(sdk), 3);
        assert_eq!(synara_joined_unread_presentation_sum(synara), 10);
        assert_ne!(
            sdk_joined_notification_checksum(sdk),
            synara_joined_unread_presentation_sum(synara)
        );
    }

    #[test]
    fn sdk_checksum_counts_muted_rooms_that_js_dock_skips() {
        // Mute skip lives in `unreadInfosFromNativeRooms`, not this Core fold.
        // A muted joined room still contributes to the SDK total.
        let sdk = [SdkUnreadChecksumRoom {
            joined: true,
            num_unread_notifications: 4,
            is_marked_unread: false,
        }];
        assert_eq!(sdk_joined_notification_checksum(sdk), 4);
        assert_eq!(
            synara_joined_unread_presentation_sum([SynaraUnreadChecksumRoom {
                membership: RoomUnreadMembership::Joined,
                num_unread_messages: 4,
                num_unread_notifications: 4,
                num_unread_mentions: 0,
                is_marked_unread: false,
            }]),
            4
        );
    }

    #[test]
    fn sdk_checksum_has_no_later_items_layer() {
        // Later-item counts are JS-only (`summarizeNotifications`). The SDK
        // total cannot represent them.
        assert_eq!(
            sdk_joined_notification_checksum([SdkUnreadChecksumRoom {
                joined: true,
                num_unread_notifications: 0,
                is_marked_unread: false,
            }]),
            0
        );
        assert_eq!(
            synara_joined_unread_presentation_sum([SynaraUnreadChecksumRoom {
                membership: RoomUnreadMembership::Joined,
                num_unread_messages: 0,
                num_unread_notifications: 0,
                num_unread_mentions: 0,
                is_marked_unread: false,
            }]),
            0
        );
    }

    #[test]
    fn sdk_checksum_skips_invites_and_non_joined() {
        assert_eq!(
            sdk_joined_notification_checksum([SdkUnreadChecksumRoom {
                joined: false,
                num_unread_notifications: 9,
                is_marked_unread: true,
            }]),
            0
        );
        assert_eq!(
            synara_joined_unread_presentation_sum([SynaraUnreadChecksumRoom {
                membership: RoomUnreadMembership::Invited,
                num_unread_messages: 9,
                num_unread_notifications: 9,
                num_unread_mentions: 1,
                is_marked_unread: true,
            }]),
            0
        );
    }

    #[test]
    fn dock_badge_owner_must_stay_js_summarize_notifications() {
        let live = include_str!("live.rs");
        let counts = include_str!("counts.rs");
        let production = counts
            .split("#[cfg(test)]")
            .next()
            .expect("production counts source");
        assert!(production.contains("client.total_unread_notifications()"));
        assert!(
            !live.contains("set_badge"),
            "Core room-list projection must not write the desktop badge"
        );
        assert!(
            !production.contains("set_badge") && !production.contains("desktop_set_badge"),
            "SDK unread checksum must not write the desktop badge"
        );
    }
}
