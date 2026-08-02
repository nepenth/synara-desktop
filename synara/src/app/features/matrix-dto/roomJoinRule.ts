/**
 * Neutral room join-rule values used by presentation boundaries.
 *
 * This contract intentionally has no matrix-js-sdk dependency. Producers may
 * supply SDK runtime values, native/wire values, or unknown input; callers
 * must normalize before passing a value into presentation code.
 */

export type RoomJoinRulePresentation =
  | 'public'
  | 'invite'
  | 'knock'
  | 'private'
  | 'restricted'
  | 'knock_restricted';

const ROOM_JOIN_RULE_PRESENTATIONS: readonly RoomJoinRulePresentation[] = [
  'public',
  'invite',
  'knock',
  'private',
  'restricted',
  'knock_restricted',
];

const ROOM_JOIN_RULE_PRESENTATION_SET = new Set<string>(ROOM_JOIN_RULE_PRESENTATIONS);

export function isRoomJoinRulePresentation(input: unknown): input is RoomJoinRulePresentation {
  return typeof input === 'string' && ROOM_JOIN_RULE_PRESENTATION_SET.has(input);
}

/**
 * Normalize SDK-like or native/wire join-rule input without coercion.
 * Unsupported, malformed, and future values fail closed as null.
 */
export function normalizeRoomJoinRulePresentation(input: unknown): RoomJoinRulePresentation | null {
  return isRoomJoinRulePresentation(input) ? input : null;
}
