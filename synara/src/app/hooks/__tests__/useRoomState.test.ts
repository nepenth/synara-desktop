import assert from 'node:assert/strict';
import test from 'node:test';

import { StateEvent } from '../../../types/matrix/room';
import type { MatrixEventReading } from '../../utils/room';
import { collectRoomStateEvents } from '../useRoomState';
import { collectRoomAccountData } from '../useRoomAccountData';

const makeEvent = (type: string, stateKey: string): MatrixEventReading =>
  ({
    getContent: () => ({}),
    getPrevContent: () => ({}),
    getSender: () => '@alice:example.org',
    getType: () => type,
    getStateKey: () => stateKey,
    getTs: () => 0,
    getId: () => `$${type}:${stateKey}`,
    getRoomId: () => '!room:example.org',
    isRedacted: () => false,
    isSending: () => false,
    getRelation: () => null,
    event: {},
  } as MatrixEventReading);

test('collectRoomStateEvents returns empty when currentState has no events map', () => {
  assert.equal(collectRoomStateEvents(undefined).size, 0);
  assert.equal(collectRoomStateEvents(null).size, 0);
  assert.equal(collectRoomStateEvents({}).size, 0);
  assert.doesNotThrow(() => collectRoomStateEvents({ events: undefined }));
});

test('collectRoomStateEvents copies indexed state and skips room members', () => {
  const topic = makeEvent(StateEvent.RoomTopic, '');
  const member = makeEvent(StateEvent.RoomMember, '@alice:example.org');
  const events = new Map<string, Map<string, MatrixEventReading>>([
    [StateEvent.RoomTopic, new Map([['', topic]])],
    [StateEvent.RoomMember, new Map([[member.getStateKey() ?? '', member]])],
  ]);

  const state = collectRoomStateEvents({ events });
  assert.equal(state.size, 1);
  assert.equal(state.get(StateEvent.RoomTopic)?.get('')?.getType(), StateEvent.RoomTopic);
  assert.equal(state.has(StateEvent.RoomMember), false);
});

test('collectRoomAccountData returns empty for native get-only stubs', () => {
  assert.equal(collectRoomAccountData(undefined).size, 0);
  assert.equal(collectRoomAccountData({ get: () => undefined }).size, 0);
  assert.doesNotThrow(() => collectRoomAccountData({ get: () => undefined }));
});

test('collectRoomAccountData copies Map entries with event content', () => {
  const source = new Map([['org.example.pref', { getContent: () => ({ color: 'blue' }) }]]);

  const accountData = collectRoomAccountData(source);
  assert.deepEqual(accountData.get('org.example.pref'), { color: 'blue' });
});
