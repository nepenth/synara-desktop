import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  NOTIFICATION_OBSERVED_EVENT,
  parseNativeNotificationObservation,
  subscribeNativeNotificationObservations,
  type NativeNotificationObservation,
} from '../nativeNotificationObservation';

const OBSERVATION: NativeNotificationObservation = {
  sessionGeneration: 7,
  roomId: '!room:example.org',
  eventId: '$event:example.org',
  sender: '@bob:example.org',
  eventType: 'm.room.message',
  originServerTs: 1_700_000_000_000,
  body: 'hello',
};

test('observation parser accepts identity plus bounded facts and rejects verdicts', () => {
  assert.deepEqual(parseNativeNotificationObservation(OBSERVATION), OBSERVATION);
  // Body is optional (stickers, undecryptable events).
  const bare: NativeNotificationObservation = { ...OBSERVATION };
  delete bare.body;
  assert.deepEqual(parseNativeNotificationObservation({ ...bare, body: null }), bare);
  assert.deepEqual(parseNativeNotificationObservation(bare), bare);

  // A policy verdict or any unknown key fails closed so the wire cannot grow
  // one silently; identity must be well-formed; the type vocabulary is closed.
  for (const bad of [
    { ...OBSERVATION, highlight: true },
    { ...OBSERVATION, sound: true },
    { ...OBSERVATION, notify: true },
    { ...OBSERVATION, mode: 'all_messages' },
    { ...OBSERVATION, roomId: 'room' },
    { ...OBSERVATION, eventId: 'event' },
    { ...OBSERVATION, sender: 'bob' },
    { ...OBSERVATION, eventType: 'm.reaction' },
    { ...OBSERVATION, sessionGeneration: -1 },
    { ...OBSERVATION, originServerTs: 'now' },
    { ...OBSERVATION, body: 42 },
    null,
    [],
    'observation',
  ]) {
    assert.equal(parseNativeNotificationObservation(bad), undefined);
  }
});

test('subscription delivers only well-formed observations for the current generation', async () => {
  const handlers = new Map<string, (event: { payload: unknown }) => void>();
  let unlistened = 0;
  const session: { generation: number | undefined } = { generation: undefined };
  const received: NativeNotificationObservation[] = [];

  const dispose = subscribeNativeNotificationObservations(
    () => session.generation,
    (observation) => received.push(observation),
    {
      desktopAvailable: true,
      listen: async (event, handler) => {
        handlers.set(event, handler as (event: { payload: unknown }) => void);
        return () => {
          unlistened += 1;
        };
      },
    }
  );
  await Promise.resolve();
  const emit = handlers.get(NOTIFICATION_OBSERVED_EVENT);
  assert.ok(emit, 'subscribes to the observation event');

  // Before the renderer knows its generation nothing is decided.
  emit({ payload: OBSERVATION });
  assert.deepEqual(received, []);

  session.generation = 7;
  emit({ payload: OBSERVATION });
  emit({ payload: { ...OBSERVATION, sessionGeneration: 6 } });
  emit({ payload: { ...OBSERVATION, highlight: true } });
  assert.deepEqual(received, [OBSERVATION]);

  dispose();
  emit({ payload: OBSERVATION });
  assert.deepEqual(received, [OBSERVATION]);
  assert.equal(unlistened, 1);
});

test('subscription is inert off desktop', async () => {
  let listened = 0;
  const dispose = subscribeNativeNotificationObservations(
    () => 7,
    () => assert.fail('no observation without a native shell'),
    {
      desktopAvailable: false,
      listen: async () => {
        listened += 1;
        return () => undefined;
      },
    }
  );
  await Promise.resolve();
  dispose();
  assert.equal(listened, 0);
});

test('the renderer observes through the Core stream and no longer scans timelines', () => {
  const root = process.cwd();
  const source = readFileSync(`${root}/src/app/pages/client/ClientNonUIFeatures.tsx`, 'utf8');

  // Both the message and the agent-approval pumps subscribe to the stream.
  const subscriptions = source.match(/subscribeNativeNotificationObservations\(/g) ?? [];
  assert.ok(subscriptions.length >= 2, 'messages and approvals subscribe');
  assert.match(source, /mx\.getSyncStateData\(\)\?\.sessionGeneration/);

  // The dead observation pumps are gone: no sync-state gate the native facade
  // never satisfies, no `Room.timeline` listener the facade never emits, no
  // periodic scan of live timelines the facade stubs to `[]`.
  assert.doesNotMatch(source, /getSyncState\(\) !== 'SYNCING'/);
  assert.doesNotMatch(source, /'Room\.timeline'/);
  assert.doesNotMatch(source, /getLoadedLiveTimelineEvents/);
  assert.doesNotMatch(source, /setInterval\(scanRecent/);
  assert.doesNotMatch(source, /RECENT_MESSAGE_NOTIFICATION_MS/);
});
