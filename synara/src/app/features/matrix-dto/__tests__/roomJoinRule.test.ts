import assert from 'node:assert/strict';
import test from 'node:test';

import {
  isRoomJoinRulePresentation,
  normalizeRoomJoinRulePresentation,
  type RoomJoinRulePresentation,
} from '../index';

test('normalizes SDK-like join-rule strings into the closed presentation contract', () => {
  const sdkLikeValues = ['public', 'invite', 'knock', 'private', 'restricted'] as const;

  for (const input of sdkLikeValues) {
    const normalized: RoomJoinRulePresentation | null = normalizeRoomJoinRulePresentation(input);
    assert.equal(normalized, input);
  }
});

test('accepts the native/wire knock_restricted presentation value', () => {
  assert.equal(normalizeRoomJoinRulePresentation('knock_restricted'), 'knock_restricted');
});

test('fails closed for unknown, malformed, and future values', () => {
  const unsupportedInputs: unknown[] = [
    undefined,
    null,
    '',
    'PUBLIC',
    'custom',
    1,
    {},
    [],
    new String('public'),
  ];

  for (const input of unsupportedInputs) {
    assert.equal(normalizeRoomJoinRulePresentation(input), null);
    assert.equal(isRoomJoinRulePresentation(input), false);
  }
});
