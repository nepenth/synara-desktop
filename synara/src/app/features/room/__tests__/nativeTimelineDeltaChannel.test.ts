import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';

import {
  subscribeNativeTimelineDeltas,
  type NativeTimelineDeltaChannelDeps,
  type NativeTimelineViewDeltaBatch,
} from '../nativeTimelineView';
import type { DesktopInvokeResult } from '../../../utils/desktop';

const ROOM = '!room:example.org';

const batch = (revision: number): NativeTimelineViewDeltaBatch => ({
  schemaVersion: 1,
  sessionGeneration: 1,
  streamId: `live:${ROOM}:1`,
  roomId: ROOM,
  revision,
  ops: [],
});

type Call = { command: string; args?: Record<string, unknown> };
type Reply = (call: Call) => Promise<DesktopInvokeResult<unknown>>;

const fakeDeps = (
  reply: Reply
): {
  deps: NativeTimelineDeltaChannelDeps;
  calls: Call[];
  send: (value: NativeTimelineViewDeltaBatch) => void;
} => {
  const calls: Call[] = [];
  let onBatch: ((value: NativeTimelineViewDeltaBatch) => void) | undefined;
  return {
    calls,
    send: (value) => onBatch?.(value),
    deps: {
      createChannel: (handler) => {
        onBatch = handler;
        return { id: 7 };
      },
      invoke: (command, args) => {
        const call = { command, args };
        calls.push(call);
        return reply(call);
      },
    },
  };
};

test('L20: a view subscribes its room with the channel and receives batches in order', async () => {
  const { deps, calls, send } = fakeDeps(async () => ({ available: true, value: 42 }));
  const received: number[] = [];
  const subscription = await subscribeNativeTimelineDeltas(
    ROOM,
    (value) => received.push(value.revision),
    deps
  );

  assert.equal(subscription?.id, 42);
  assert.deepEqual(calls, [
    { command: 'desktop_timeline_subscribe', args: { roomId: ROOM, channel: { id: 7 } } },
  ]);
  send(batch(1));
  send(batch(2));
  assert.deepEqual(received, [1, 2]);
});

test('L20: closing a subscription unsubscribes once and stops delivery', async () => {
  const { deps, calls, send } = fakeDeps(async ({ command }) =>
    command === 'desktop_timeline_subscribe'
      ? { available: true, value: 3 }
      : { available: true, value: true }
  );
  const received: number[] = [];
  const subscription = await subscribeNativeTimelineDeltas(
    ROOM,
    (value) => received.push(value.revision),
    deps
  );
  assert.ok(subscription);

  subscription.close();
  subscription.close();
  // A batch already in flight when the view unmounted is ignored.
  send(batch(1));

  assert.deepEqual(received, []);
  assert.deepEqual(
    calls.map((call) => call.command),
    ['desktop_timeline_subscribe', 'desktop_timeline_unsubscribe']
  );
  assert.deepEqual(calls[1]?.args, { subscriptionId: 3 });
});

test('L20: a failed unsubscribe does not reject', async () => {
  const { deps } = fakeDeps(async ({ command }) => {
    if (command === 'desktop_timeline_unsubscribe') throw new Error('window gone');
    return { available: true, value: 1 };
  });
  const subscription = await subscribeNativeTimelineDeltas(ROOM, () => undefined, deps);
  assert.ok(subscription);
  subscription.close();
  await new Promise((resolve) => {
    setTimeout(resolve, 0);
  });
});

test('L20: without a channel the view falls back to polling', async () => {
  const replies: Reply[] = [
    async () => ({ available: false }),
    async () => ({ available: true, value: undefined }),
    async () => ({ available: true, value: 'not-an-id' }),
    async () => {
      throw new Error('No native Matrix session is active.');
    },
  ];
  for (const reply of replies) {
    const { deps, send } = fakeDeps(reply);
    const received: number[] = [];
    const subscription = await subscribeNativeTimelineDeltas(
      ROOM,
      (value) => received.push(value.revision),
      deps
    );
    assert.equal(subscription, undefined);
    send(batch(1));
    assert.deepEqual(received, []);
  }

  const calls: Call[] = [];
  const subscription = await subscribeNativeTimelineDeltas(ROOM, () => undefined, {
    createChannel: () => {
      throw new Error('Tauri IPC is unavailable');
    },
    invoke: async (command, args) => {
      calls.push({ command, args });
      return { available: true, value: 1 };
    },
  });
  assert.equal(subscription, undefined);
  assert.deepEqual(calls, []);
});

test('L20: the view passes its subscription to every open and jump', () => {
  const source = readFileSync('src/app/features/room/nativeTimelineView.ts', 'utf8');
  assert.match(source, /\{ request: nativeRequest, subscriptionId: deltas\?\.id \}/);
  assert.match(
    source,
    /\{ request: \{ streamId \}, subscriptionId: deltaSubscriptionIdRef\.current \}/
  );
  assert.match(source, /subscriptionId: deltaSubscriptionIdRef\.current,\n {10}\}/);
  assert.match(source, /deltaSubscriptionIdRef\.current = deltas\?\.id;/);
  // Unmount closes the channel with the stream.
  assert.match(source, /stopPoll\?\.\(\);\n {6}deltas\?\.close\(\);/);
});
