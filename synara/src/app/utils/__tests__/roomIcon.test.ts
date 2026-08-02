import assert from 'node:assert/strict';
import test from 'node:test';
import type { IconName, IconSrc } from 'folds';

import { normalizeRoomJoinRulePresentation } from '../../features/matrix-dto/roomJoinRule';
import { RoomType } from '../../../types/matrix/room';
import { getRoomIconSrc } from '../room';

const icons = new Proxy(
  {},
  {
    get: (_target, property: string) => property,
  }
) as Record<IconName, IconSrc>;

test('room icon mapping keeps public, locked, and neutral semantics for every family', () => {
  const cases = [
    ['public', 'SpaceGlobe', 'VolumeHighGlobe', 'HashGlobe'],
    ['invite', 'SpaceLock', 'VolumeHighLock', 'HashLock'],
    ['knock', 'SpaceLock', 'VolumeHighLock', 'HashLock'],
    ['private', 'SpaceLock', 'VolumeHighLock', 'HashLock'],
    ['restricted', 'Space', 'VolumeHigh', 'Hash'],
    ['knock_restricted', 'Space', 'VolumeHigh', 'Hash'],
  ] as const;

  for (const [input, spaceIcon, callIcon, roomIcon] of cases) {
    const joinRule = normalizeRoomJoinRulePresentation(input);
    assert.equal(getRoomIconSrc(icons, RoomType.Space, joinRule), spaceIcon);
    assert.equal(getRoomIconSrc(icons, RoomType.Call, joinRule), callIcon);
    assert.equal(getRoomIconSrc(icons, undefined, joinRule), roomIcon);
  }
});

test('room icon mapping fails closed to the existing neutral fallback', () => {
  const joinRule = normalizeRoomJoinRulePresentation('future_join_rule');

  assert.equal(getRoomIconSrc(icons, RoomType.Space, joinRule), 'Space');
  assert.equal(getRoomIconSrc(icons, RoomType.Call, joinRule), 'VolumeHigh');
  assert.equal(getRoomIconSrc(icons, undefined, joinRule), 'Hash');
});
