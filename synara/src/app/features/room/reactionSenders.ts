import { createContext, useContext } from 'react';
import { getMxIdLocalPart } from '../../utils/matrix';
import type { RoomMemberListItem } from '../../hooks/useRoomMembers';

/**
 * Display names for the people who reacted, keyed by user id. The timeline
 * presenter fills it from the room's member list; a missing entry falls back to
 * the Matrix ID's localpart.
 */
export type ReactionSenderNames = ReadonlyMap<string, string>;

/** Display names from a member snapshot, skipping ones that are just the MXID. */
export const reactionSenderNamesFromMembers = (
  members: readonly RoomMemberListItem[] | null | undefined
): ReactionSenderNames => {
  const names = new Map<string, string>();
  for (const member of members ?? []) {
    let displayName: string | undefined;
    if ('displayName' in member && typeof member.displayName === 'string') {
      displayName = member.displayName;
    } else if ('rawDisplayName' in member) {
      displayName = member.rawDisplayName;
    }
    if (displayName && displayName !== member.userId) names.set(member.userId, displayName);
  }
  return names;
};

export const ReactionSenderNamesContext = createContext<ReactionSenderNames>(new Map());

export const useReactionSenderNames = (): ReactionSenderNames =>
  useContext(ReactionSenderNamesContext);

/** How a pill and the reactions list name one person. */
export const reactionSenderName = (
  userId: string,
  names: ReactionSenderNames,
  ownUserId?: string
): string => {
  if (ownUserId && userId === ownUserId) return 'You';
  return names.get(userId) ?? getMxIdLocalPart(userId) ?? userId;
};

const MAX_NAMED = 3;

/**
 * "Alice, Bob and 3 others reacted with 👍". Your own reaction comes first as
 * "You". Without sender data (an older readback) it falls back to the count.
 */
export const reactionSendersSummary = ({
  reactionKey,
  count,
  senderIds,
  names,
  ownUserId,
}: {
  reactionKey: string;
  count: number;
  senderIds: readonly string[];
  names: ReactionSenderNames;
  ownUserId?: string;
}): string => {
  if (senderIds.length === 0) {
    return `${count} ${count === 1 ? 'person' : 'people'} reacted with ${reactionKey}`;
  }
  const ordered = [
    ...senderIds.filter((id) => ownUserId && id === ownUserId),
    ...senderIds.filter((id) => !(ownUserId && id === ownUserId)),
  ];
  const total = Math.max(count, ordered.length);
  const shown = ordered
    .slice(0, total > MAX_NAMED + 1 ? MAX_NAMED : MAX_NAMED + 1)
    .map((id) => reactionSenderName(id, names, ownUserId));
  const others = total - shown.length;
  let who: string;
  if (others > 0) {
    who = `${shown.join(', ')} and ${others} ${others === 1 ? 'other' : 'others'}`;
  } else if (shown.length === 1) {
    [who] = shown;
  } else {
    who = `${shown.slice(0, -1).join(', ')} and ${shown[shown.length - 1]}`;
  }
  return `${who} reacted with ${reactionKey}`;
};
