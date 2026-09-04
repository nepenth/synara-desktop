import assert from 'node:assert/strict';
import test from 'node:test';

import {
  requestNativeTimelineFollowLive,
  type NativeTimelineViewSnapshot,
} from '../nativeTimelineView';

const snapshot = {
  schemaVersion: 1,
  sessionGeneration: 7,
  roomId: '!room:example.org',
  revision: 3,
  position: { kind: 'live_bottom' },
} as unknown as NativeTimelineViewSnapshot;

test('follow-live routes only through matrix_timeline_follow_live with no fallback', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const result = await requestNativeTimelineFollowLive(
    { streamId: 'view-1', observedLiveTailEventId: '$tail:example.org' },
    async (command, args) => {
      calls.push({ command, args });
      return { available: true, value: snapshot };
    }
  );

  assert.equal(result, snapshot);
  assert.deepEqual(calls, [
    {
      command: 'matrix_timeline_follow_live',
      args: { streamId: 'view-1', observedLiveTailEventId: '$tail:example.org' },
    },
  ]);
});

test('follow-live is unavailable without a native readback', async () => {
  await assert.rejects(
    requestNativeTimelineFollowLive(
      { streamId: 'view-1', observedLiveTailEventId: '$tail:example.org' },
      async () => ({ available: false as const })
    ),
    /follow-live is unavailable/
  );

  await assert.rejects(
    requestNativeTimelineFollowLive(
      { streamId: 'view-1', observedLiveTailEventId: '$tail:example.org' },
      async () => ({ available: true as const, value: undefined })
    ),
    /follow-live is unavailable/
  );
});
