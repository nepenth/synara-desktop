import type { CSSProperties } from 'react';
import { toRem } from 'folds';

/**
 * The shared avatar size ladder (DS-11), in px, matching iOS points:
 * row glyph 20, dense 24, lists and the reaction viewer 28, timeline 36–40,
 * profile rows 44, hero 64. Folds `Avatar` sizes stop at 24/36/42/58, so the
 * rungs it lacks are set inline.
 */
export const AVATAR_LADDER = {
  glyph: 20,
  dense: 24,
  list: 28,
  timeline: 36,
  profileRow: 44,
  hero: 64,
} as const;

export type AvatarRung = keyof typeof AVATAR_LADDER;

/** Rounded-square people and room tiles use 22 % corners on both platforms. */
export const AVATAR_SQUARE_RADIUS = '22%';

/** Size (and the shared 22 % corner) for a folds `Avatar` on a ladder rung. */
export const avatarLadderStyle = (rung: AvatarRung): CSSProperties => {
  const size = toRem(AVATAR_LADDER[rung]);
  return { width: size, height: size, borderRadius: AVATAR_SQUARE_RADIUS };
};
