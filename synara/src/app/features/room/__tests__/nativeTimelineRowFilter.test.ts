import assert from 'node:assert/strict';
import test from 'node:test';
import {
  isNativeTimelineProfileChangeRow,
  isNativeTimelineRowVisible,
  isNativeTimelineUnsupportedRow,
} from '../nativeTimelineRowFilter';
import type { NativeTimelineViewRow } from '../nativeTimelineView';

const base = {
  itemId: '$e',
  eventId: '$e',
  senderId: '@a:example.org',
  senderName: 'A',
  originServerTs: 1,
  capabilities: {},
};

const state = (stateType: string, summary = 'changed'): NativeTimelineViewRow =>
  ({ ...base, kind: 'state', stateType, summary }) as NativeTimelineViewRow;
const membership = { ...base, kind: 'membership', targetUserId: '@b:x', summary: 'joined' };
const unsupported = { kind: 'other', itemId: '$u', summary: 'Unsupported timeline event' };
const stickerUnavailable = { ...unsupported, forwardTransport: 'media', summary: 'Sticker' };

const defaults = {
  hideMembershipEvents: false,
  hideNickAvatarEvents: true,
  showHiddenEvents: false,
};

test('Hide Profile Change hides display name and avatar changes only (D3)', () => {
  assert.equal(isNativeTimelineProfileChangeRow(state('m.room.member')), true);
  assert.equal(isNativeTimelineRowVisible(state('m.room.member'), defaults), false);
  // Every other state change stays visible with the setting on (its default).
  for (const stateType of [
    'm.room.name',
    'm.room.topic',
    'm.room.avatar',
    'm.room.power_levels',
    'm.room.join_rules',
    'm.room.history_visibility',
    'm.room.encryption',
    'm.room.pinned_events',
    'm.room.server_acl',
    'm.room.tombstone',
  ]) {
    assert.equal(isNativeTimelineRowVisible(state(stateType), defaults), true, stateType);
  }
  assert.equal(
    isNativeTimelineRowVisible(state('m.room.member'), {
      ...defaults,
      hideNickAvatarEvents: false,
    }),
    true
  );
});

test('Hide Membership Change hides joins, leaves and invites', () => {
  const row = membership as NativeTimelineViewRow;
  assert.equal(isNativeTimelineRowVisible(row, defaults), true);
  assert.equal(isNativeTimelineRowVisible(row, { ...defaults, hideMembershipEvents: true }), false);
});

test('Show Hidden Events shows unsupported events and overrides both hide toggles (D16)', () => {
  const hidden = {
    hideMembershipEvents: true,
    hideNickAvatarEvents: true,
    showHiddenEvents: false,
  };
  const shown = { ...hidden, showHiddenEvents: true };
  const rows = [
    unsupported as NativeTimelineViewRow,
    membership as NativeTimelineViewRow,
    state('m.room.member'),
  ];
  assert.equal(isNativeTimelineUnsupportedRow(rows[0]), true);
  assert.deepEqual(
    rows.map((row) => isNativeTimelineRowVisible(row, hidden)),
    [false, false, false]
  );
  assert.deepEqual(
    rows.map((row) => isNativeTimelineRowVisible(row, shown)),
    [true, true, true]
  );
  // An unavailable sticker is content, not an unsupported event.
  const sticker = stickerUnavailable as NativeTimelineViewRow;
  assert.equal(isNativeTimelineUnsupportedRow(sticker), false);
  assert.equal(isNativeTimelineRowVisible(sticker, hidden), true);
});
