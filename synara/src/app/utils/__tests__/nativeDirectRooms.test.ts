import assert from 'node:assert/strict';
import test from 'node:test';
import type { DesktopInvokeResult } from '../desktop';
import {
  createNativeSession,
  getNativeRoom,
  nativeSession,
  setNativeSessionForTests,
} from '../../native/nativeSession';
import type { NativeInvoke } from '../../native/nativeWire';
import { getDMRoomFor, guessDmRoomUserId } from '../matrix';

const ME = '@me:example.org';

const summary = (roomId: string, overrides: Record<string, unknown> = {}) => ({
  roomId,
  name: roomId,
  membership: 'join',
  isDirect: false,
  isSpace: false,
  isCall: false,
  hasActiveCall: false,
  activeCallParticipantCount: 0,
  isFavorite: false,
  isEncrypted: false,
  encryptionStatus: 'not_encrypted',
  unreadCount: 0,
  highlightCount: 0,
  markedUnread: false,
  lastMessageIsAgentApproval: false,
  ...overrides,
});

const withRooms = async (rooms: Record<string, unknown>[], run: () => void) => {
  const routes: Record<string, unknown> = {
    matrix_session_snapshot: {
      status: 'logged_in',
      user_id: ME,
      device_id: 'DEVICE',
      homeserver_url: 'https://example.org',
      sessionGeneration: 3,
    },
    matrix_room_list_snapshot: { sessionGeneration: 3, rooms },
  };
  const invoke: NativeInvoke = async (command) =>
    (Object.prototype.hasOwnProperty.call(routes, command)
      ? { available: true, value: routes[command] }
      : { available: false }) as DesktopInvokeResult<unknown>;
  const session = createNativeSession(invoke);
  await session.refresh();
  const original = nativeSession();
  setNativeSessionForTests(session);
  try {
    run();
  } finally {
    setNativeSessionForTests(original);
  }
};

test('D15: an existing DM is found from Core summaries, most recent first', async () => {
  await withRooms(
    [
      summary('!old:example.org', {
        isDirect: true,
        directUserId: '@bob:example.org',
        lastActivityTs: 10,
      }),
      summary('!new:example.org', {
        isDirect: true,
        directUserId: '@bob:example.org',
        lastActivityTs: 20,
      }),
      summary('!left:example.org', {
        isDirect: true,
        directUserId: '@bob:example.org',
        membership: 'leave',
        lastActivityTs: 30,
      }),
      summary('!group:example.org', { directUserId: undefined, lastActivityTs: 40 }),
    ],
    () => {
      assert.equal(getDMRoomFor('@bob:example.org')?.roomId, '!new:example.org');
      assert.equal(getDMRoomFor('@carol:example.org'), undefined);
    }
  );
});

test('D15: the DM peer never falls back to the signed-in user', async () => {
  await withRooms(
    [
      summary('!dm:example.org', { isDirect: true, directUserId: '@bob:example.org' }),
      summary('!hero:example.org', {
        heroes: [{ userId: ME }, { userId: '@carol:example.org' }],
      }),
      summary('!group:example.org', {
        heroes: [{ userId: '@carol:example.org' }, { userId: '@dave:example.org' }],
      }),
      summary('!alone:example.org'),
    ],
    () => {
      const peer = (roomId: string) => {
        const room = getNativeRoom(roomId);
        assert.ok(room);
        return guessDmRoomUserId(room, ME);
      };
      assert.equal(peer('!dm:example.org'), '@bob:example.org');
      assert.equal(peer('!hero:example.org'), '@carol:example.org');
      assert.equal(peer('!group:example.org'), undefined);
      assert.equal(peer('!alone:example.org'), undefined);
    }
  );
});

test('D15: the room facade reports encryption from Core, so logout can warn', async () => {
  await withRooms(
    [
      summary('!enc:example.org', { isEncrypted: true, encryptionStatus: 'encrypted' }),
      summary('!plain:example.org'),
    ],
    () => {
      assert.equal(getNativeRoom('!enc:example.org')?.hasEncryptionStateEvent(), true);
      assert.equal(getNativeRoom('!plain:example.org')?.hasEncryptionStateEvent(), false);
    }
  );
});
