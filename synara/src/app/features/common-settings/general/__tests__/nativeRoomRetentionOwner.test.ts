import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { DesktopInvokeResult } from '../../../../utils/desktop';
import {
  getRoomRetentionWithNativeOwner,
  parseRoomRetentionSnapshot,
  type NativeInvoke,
} from '../nativeRoomRetentionOwner';

const session = {
  status: 'logged_in',
  user_id: '@alice:example.org',
  device_id: 'DEVICE',
  homeserver_url: 'https://matrix.example.org',
  sessionGeneration: 7,
};

const snapshot = {
  status: 'ok',
  roomId: '!room:example.org',
  sessionGeneration: 7,
  advertised: false,
  summary:
    'The homeserver does not advertise a retention policy. History may still be deleted by the server without telling this client.',
  distinction:
    'This is separate from history visibility, which only controls who may see past messages, not how long the server keeps them.',
  mediaCacheSummary:
    'Media cache on this device expires after 30 days unless a joined room has a shorter retention.',
};

test('retention parser rejects forever copy and history-visibility reuse', () => {
  assert.equal(
    parseRoomRetentionSnapshot(snapshot, '!room:example.org', 7).advertised,
    false
  );
  assert.throws(() =>
    parseRoomRetentionSnapshot(
      { ...snapshot, summary: 'Messages are kept forever.' },
      '!room:example.org',
      7
    )
  );
  assert.throws(() =>
    parseRoomRetentionSnapshot(
      { ...snapshot, summary: 'Same as history visibility.' },
      '!room:example.org',
      7
    )
  );
});

test('retention owner invokes matrix_room_retention with session generation', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativeInvoke = async (command, args) => {
    calls.push({ command, args });
    if (command === 'matrix_session_snapshot') {
      return { available: true, value: session } satisfies DesktopInvokeResult<unknown>;
    }
    if (command === 'matrix_room_retention') {
      return { available: true, value: snapshot } satisfies DesktopInvokeResult<unknown>;
    }
    throw new Error(`unexpected ${command}`);
  };
  const result = await getRoomRetentionWithNativeOwner('!room:example.org', true, invoke);
  assert.equal(result.summary, snapshot.summary);
  assert.deepEqual(calls[1], {
    command: 'matrix_room_retention',
    args: { roomId: '!room:example.org', sessionGeneration: 7 },
  });
});
