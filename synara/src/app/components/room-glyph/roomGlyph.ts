import type { RoomSummary } from '../../features/matrix-dto/room';
import { nameInitials } from '../../utils/common';
import { getMxIdLocalPart } from '../../utils/matrix';

export type PersonRoomGlyph = {
  kind: 'person';
  userId: string;
  /** Raw content URI; the avatar component resolves it through synara-media. */
  avatarUrl?: string;
  initial: string;
};

export type RoomGlyph = { kind: 'channel' } | PersonRoomGlyph;

type RoomGlyphSummary = Pick<RoomSummary, 'isDirect' | 'directUserId' | 'avatarUrl' | 'heroes'>;

/**
 * The leading glyph for a room row or header (iOS V3): a direct room with one
 * known peer shows that person, and everything else keeps `#`. Core already
 * falls back to the peer's avatar for a DM without a room avatar.
 */
export const roomGlyphFor = (
  summary: RoomGlyphSummary | undefined,
  roomName?: string
): RoomGlyph => {
  const userId = summary?.isDirect === true ? summary.directUserId : undefined;
  if (!userId) return { kind: 'channel' };
  const heroName = summary?.heroes?.find((hero) => hero.userId === userId)?.displayName;
  const name = heroName?.trim() || roomName?.trim() || getMxIdLocalPart(userId) || userId;
  return {
    kind: 'person',
    userId,
    avatarUrl: summary?.avatarUrl || undefined,
    initial: nameInitials(name),
  };
};
