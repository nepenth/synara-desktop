//! Shared DM / invite avatar selection on `human_member_ids_no_sync`.
//!
//! 0.19 `heroes()` already drops functional members; this helper is the single
//! product owner so invite cards and joined room-list summaries pick the same
//! peer MXC. `human_member_ids_no_sync` includes self — callers must exclude it.

use matrix_sdk::{
    ruma::{OwnedMxcUri, UserId},
    Room, RoomMemberships,
};

/// Which MXC source a DM / invite projection should use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmAvatarSourceKind {
    /// Room avatar (non-direct, or more than two human members).
    RoomAvatar,
    /// The other human's member avatar, else the room avatar.
    PeerMember,
}

/// Pure table used by unit tests. `human_member_count` includes self.
pub fn select_dm_avatar_source(is_direct: bool, human_member_count: usize) -> DmAvatarSourceKind {
    if !is_direct || human_member_count > 2 {
        DmAvatarSourceKind::RoomAvatar
    } else {
        DmAvatarSourceKind::PeerMember
    }
}

/// Invite + joined DM avatar MXC without a membership fan-out.
///
/// Active memberships first; if that set is empty, retry with `empty()` to
/// match the previous two-party invite fallback.
pub async fn dm_avatar_source(
    room: &Room,
    current_user: &UserId,
    is_direct: bool,
) -> Option<OwnedMxcUri> {
    let room_avatar = room.avatar_url();
    if !is_direct {
        return room_avatar;
    }

    let mut human_ids = match room.human_member_ids_no_sync(RoomMemberships::ACTIVE).await {
        Ok(ids) => ids,
        Err(_) => return room_avatar,
    };
    if human_ids.is_empty() {
        human_ids = match room
            .human_member_ids_no_sync(RoomMemberships::empty())
            .await
        {
            Ok(ids) => ids,
            Err(_) => return room_avatar,
        };
    }

    if select_dm_avatar_source(true, human_ids.len()) == DmAvatarSourceKind::RoomAvatar {
        return room_avatar;
    }

    let peer = human_ids
        .into_iter()
        .find(|id| id.as_str() != current_user.as_str());
    let Some(peer) = peer else {
        return room_avatar;
    };
    match room.get_member_no_sync(&peer).await {
        Ok(Some(member)) => member.avatar_url().map(ToOwned::to_owned).or(room_avatar),
        _ => room_avatar,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_direct_always_uses_room_avatar() {
        assert_eq!(
            select_dm_avatar_source(false, 1),
            DmAvatarSourceKind::RoomAvatar
        );
        assert_eq!(
            select_dm_avatar_source(false, 2),
            DmAvatarSourceKind::RoomAvatar
        );
        assert_eq!(
            select_dm_avatar_source(false, 8),
            DmAvatarSourceKind::RoomAvatar
        );
    }

    #[test]
    fn one_to_one_uses_peer_member() {
        // Self + peer, including a 1:1 where a functional bot was already
        // dropped by `human_member_ids_no_sync`.
        assert_eq!(
            select_dm_avatar_source(true, 2),
            DmAvatarSourceKind::PeerMember
        );
        assert_eq!(
            select_dm_avatar_source(true, 1),
            DmAvatarSourceKind::PeerMember
        );
        assert_eq!(
            select_dm_avatar_source(true, 0),
            DmAvatarSourceKind::PeerMember
        );
    }

    #[test]
    fn three_humans_use_room_avatar() {
        assert_eq!(
            select_dm_avatar_source(true, 3),
            DmAvatarSourceKind::RoomAvatar
        );
        assert_eq!(
            select_dm_avatar_source(true, 4),
            DmAvatarSourceKind::RoomAvatar
        );
    }
}
