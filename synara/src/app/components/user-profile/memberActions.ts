import { Membership } from '../../../types/matrix/room';

export type MemberActionPermissions = {
  canKick: boolean;
  canBan: boolean;
  canInvite: boolean;
};

export type MemberActionVisibility = {
  sendMessage: boolean;
  mention: boolean;
  share: boolean;
  ignore: boolean;
  invite: boolean;
  cancelInvite: boolean;
  removeFromRoom: boolean;
  ban: boolean;
  unban: boolean;
  showPermissionsLoading: boolean;
};

/**
 * Room-member profile actions that a Matrix CS-API client can offer.
 *
 * Native desktop rooms do not populate `room.getMember()`, so callers must
 * pass membership from the native members snapshot. Treating a missing JS
 * member as `leave` hides Remove/Kick for every joined user.
 *
 * Homeserver-admin "Deactivate user" is intentionally omitted: it is a Synapse
 * Admin API, not matrix-rust-sdk.
 */
export function memberActionVisibility(input: {
  isSelf: boolean;
  membership: Membership | null;
  permissionsReady: boolean;
  permissions: MemberActionPermissions;
}): MemberActionVisibility {
  const { isSelf, membership, permissionsReady, permissions } = input;
  const showPermissionsLoading = !isSelf && !permissionsReady;
  const ready = permissionsReady && !isSelf;
  const known = membership !== null;

  return {
    sendMessage: !isSelf,
    mention: !isSelf,
    share: true,
    ignore: !isSelf,
    invite: ready && known && membership === Membership.Leave && permissions.canInvite,
    cancelInvite: ready && known && membership === Membership.Invite && permissions.canKick,
    removeFromRoom:
      ready && permissions.canKick && (membership === Membership.Join || membership === null),
    ban: ready && permissions.canBan && membership !== Membership.Ban,
    unban: ready && known && membership === Membership.Ban && permissions.canBan,
    showPermissionsLoading,
  };
}

export function resolveNativeRoomMembership(
  nativeMembers: Array<{ userId: string; membership?: string }> | null | undefined,
  userId: string
): Membership | null {
  if (nativeMembers === null || nativeMembers === undefined) return null;
  const member = nativeMembers.find((item) => item.userId === userId);
  if (!member) return Membership.Leave;
  switch (member.membership) {
    case Membership.Join:
      return Membership.Join;
    case Membership.Invite:
      return Membership.Invite;
    case Membership.Ban:
      return Membership.Ban;
    case Membership.Knock:
      return Membership.Knock;
    case Membership.Leave:
      return Membership.Leave;
    default:
      return Membership.Leave;
  }
}
