import type { NativeTimelineViewRow } from './nativeTimelineView';

export type NativeTimelineRowFilterSettings = {
  /** "Hide Membership Change": joins, leaves and invites. */
  hideMembershipEvents: boolean;
  /** "Hide Profile Change": display name and avatar changes only. */
  hideNickAvatarEvents: boolean;
  /** "Show Hidden Events": overrides both toggles and shows unsupported events. */
  showHiddenEvents: boolean;
};

/**
 * Core projects display name and avatar changes as `state` rows of type
 * `m.room.member` (`ProfileChange`). Every other state change (name, topic,
 * power levels, join rules, encryption, room upgrades) keeps its own type.
 */
export const isNativeTimelineProfileChangeRow = (row: NativeTimelineViewRow): boolean =>
  row.kind === 'state' && row.stateType === 'm.room.member';

/**
 * An event Core could not project. Core's other content (an unavailable
 * sticker) still carries a forward transport; an unsupported event does not.
 */
export const isNativeTimelineUnsupportedRow = (row: NativeTimelineViewRow): boolean =>
  row.kind === 'other' && row.forwardTransport === undefined;

/** Whether the timeline shows `row` under the user's message settings. */
export const isNativeTimelineRowVisible = (
  row: NativeTimelineViewRow,
  settings: NativeTimelineRowFilterSettings
): boolean => {
  if (settings.showHiddenEvents) return true;
  if (settings.hideMembershipEvents && row.kind === 'membership') return false;
  if (settings.hideNickAvatarEvents && isNativeTimelineProfileChangeRow(row)) return false;
  if (isNativeTimelineUnsupportedRow(row)) return false;
  return true;
};
