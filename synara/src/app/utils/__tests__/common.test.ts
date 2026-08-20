import assert from 'node:assert/strict';
import test from 'node:test';
import { roomAvatarTone, roomNameInitials } from '../common';

test('roomNameInitials uses the first letter of the first two meaningful words', () => {
  assert.equal(roomNameInitials('Project - Media Management'), 'PM');
  assert.equal(roomNameInitials('Project - Tech'), 'PT');
  assert.equal(roomNameInitials('Project - Bug Bounty Hunting'), 'PB');
  assert.equal(roomNameInitials('General - Family'), 'GF');
  assert.equal(roomNameInitials('cronjob-output'), 'CO');
});

test('roomNameInitials skips filler words and still returns two letters for a single word', () => {
  assert.equal(roomNameInitials('The Alerts'), 'AL');
  assert.equal(roomNameInitials('Alerts'), 'AL');
  assert.equal(roomNameInitials('#channel'), 'CH');
  assert.equal(roomNameInitials(''), 'S');
});

test('roomAvatarTone is stable per room id and stays in a muted grey palette', () => {
  const first = roomAvatarTone('!room:example.org');
  assert.equal(roomAvatarTone('!room:example.org').background, first.background);
  assert.notEqual(roomAvatarTone('!other:example.org').background, first.background);
  assert.match(first.background, /^#[0-9a-f]{6}$/i);
});
