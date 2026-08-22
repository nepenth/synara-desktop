import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import type { DesktopInvokeResult } from '../../../utils/desktop';
import { isRoom, isSpace } from '../../../utils/room';
import {
  createNativeMatrixClient,
  readinessToSyncState,
  type NativeInvoke,
  type NativeRoomListSnapshot,
} from '../nativeClientFacade';

const ok = (value: unknown): DesktopInvokeResult<unknown> => ({ available: true, value });
const unavailable: DesktopInvokeResult<unknown> = { available: false };

const roomSnapshot = (
  overrides: Partial<NativeRoomListSnapshot['rooms'][number]> = {}
): NativeRoomListSnapshot => ({
  sessionGeneration: 8,
  rooms: [
    {
      roomId: '!r:example.org',
      name: 'Engineering',
      membership: 'join',
      isDirect: false,
      isSpace: false,
      isCall: false,
      isFavorite: false,
      isEncrypted: false,
      unreadCount: 0,
      highlightCount: 0,
      markedUnread: false,
      ...overrides,
    },
  ],
});

const invokingWith = (routes: Record<string, unknown>) => {
  const callLog: string[] = [];
  const invoke: NativeInvoke = async (command) => {
    callLog.push(command);
    if (Object.prototype.hasOwnProperty.call(routes, command)) return ok(routes[command]);
    return unavailable;
  };
  return { invoke, callLog };
};

test('readinessToSyncState maps Rust readiness to js-sdk literals', () => {
  assert.equal(readinessToSyncState('running'), 'PREPARED');
  assert.equal(readinessToSyncState('offline'), 'RECONNECTING');
  assert.equal(readinessToSyncState('failed'), 'ERROR');
  assert.equal(readinessToSyncState('idle'), 'STOPPED');
  assert.equal(readinessToSyncState('unconfigured'), 'STOPPED');
});

test('getSyncState proxies matrix_sync_status and caches PREPARED when running', async () => {
  const { invoke } = invokingWith({
    matrix_sync_status: {
      readiness: 'running',
      sessionGeneration: 7,
      offlineModeEnabled: false,
    },
  });
  const client = createNativeMatrixClient(invoke);
  await client.refresh();
  assert.equal(client.getSyncState(), 'PREPARED');
  assert.equal(client.clientRunning(), true);
  assert.deepEqual(client.getSyncStateData(), {
    readiness: 'running',
    sessionGeneration: 7,
    failureDiagnosticId: null,
    slidingSyncCapable: null,
  });
});

test('getSyncState fails closed when the native command is unavailable', async () => {
  const { invoke } = invokingWith({});
  const client = createNativeMatrixClient(invoke);
  await client.refresh();
  assert.equal(client.getSyncState(), null);
  assert.equal(client.clientRunning(), false);
});
test('slidingSyncCapable tri-state: true/false/null propagate through getSyncStateData', async () => {
  for (const capable of [true, false, null]) {
    const { invoke } = invokingWith({
      matrix_sync_status: {
        readiness: 'running',
        sessionGeneration: 1,
        offlineModeEnabled: false,
        ...(capable === null ? {} : { slidingSyncCapable: capable }),
      },
    });
    const client = createNativeMatrixClient(invoke);
    await client.refresh();
    assert.equal(client.getSyncStateData()?.slidingSyncCapable, capable);
  }
});

test('slidingSyncCapable absent on the wire yields null (unknown)', async () => {
  const { invoke } = invokingWith({
    matrix_sync_status: {
      readiness: 'idle',
      sessionGeneration: 2,
      offlineModeEnabled: false,
    },
  });
  const client = createNativeMatrixClient(invoke);
  await client.refresh();
  assert.equal(client.getSyncState(), 'STOPPED');
  assert.equal(client.getSyncStateData()?.slidingSyncCapable, null);
});

test('sync emitter delivers the sync payload to listeners and removeListener detaches', async () => {
  const { invoke } = invokingWith({
    matrix_sync_status: { readiness: 'failed', sessionGeneration: 3, offlineModeEnabled: false },
  });
  const client = createNativeMatrixClient(invoke);
  const seen: unknown[] = [];
  const listener = (payload: unknown): void => {
    seen.push(payload);
  };
  client.on('sync', listener);
  await client.refresh();
  assert.deepEqual(seen, ['ERROR']);
  client.removeListener('sync', listener);
  await client.refresh();
  assert.deepEqual(seen, ['ERROR']);
});

test('identity comes from matrix_session_snapshot (logged_in)', async () => {
  const { invoke } = invokingWith({
    matrix_session_snapshot: {
      status: 'logged_in',
      user_id: '@alice:example.org',
      device_id: 'DEVICE',
      homeserver_url: 'https://matrix.example.org',
      sessionGeneration: 9,
    },
  });
  const client = createNativeMatrixClient(invoke);
  await client.refresh();
  assert.equal(client.getUserId(), '@alice:example.org');
  assert.equal(client.getSafeUserId(), '@alice:example.org');
  assert.equal(client.getDeviceId(), 'DEVICE');
});

test('identity is empty when the session is logged out', async () => {
  const { invoke } = invokingWith({ matrix_session_snapshot: { status: 'logged_out' } });
  const client = createNativeMatrixClient(invoke);
  await client.refresh();
  assert.equal(client.getUserId(), null);
  assert.equal(client.getSafeUserId(), '');
});

test('write commands call the native profile commands', async () => {
  const { invoke } = invokingWith({
    matrix_set_own_display_name: { status: 'ok' },
    matrix_set_own_avatar: { status: 'ok' },
  });
  const client = createNativeMatrixClient(invoke);
  assert.deepEqual(await client.setDisplayName('Alice'), { status: 'ok' });
  assert.deepEqual(await client.setAvatarUrl('mxc://example.org/a'), { status: 'ok' });
});

test('D1C: the facade exposes NO token surface (renderer cedes custody)', async () => {
  const client = createNativeMatrixClient(async () => unavailable);
  const facade = client as unknown as Record<string, unknown>;
  assert.equal('getAccessToken' in facade, false);
  assert.equal('setAccessToken' in facade, false);
  assert.equal('refreshToken' in facade, false);
});

test('watchSync emits on readiness change into PREPARED', async () => {
  let readiness: 'idle' | 'running' = 'idle';
  const invokeDyn: NativeInvoke = async (command) => {
    if (command === 'matrix_sync_status') {
      return ok({ readiness, sessionGeneration: 4, offlineModeEnabled: false });
    }
    return unavailable;
  };
  const client = createNativeMatrixClient(invokeDyn);
  const seen: unknown[] = [];
  client.on('sync', (payload) => seen.push(payload));
  await new Promise<void>((resolve) => {
    const unwatch = client.watchSync(5);
    setTimeout(() => {
      readiness = 'running';
      setTimeout(() => {
        unwatch();
        resolve();
      }, 25);
    }, 8);
  });
  assert.ok(seen.includes('PREPARED'), `expected PREPARED emission, saw: ${JSON.stringify(seen)}`);
});

test('watchSync ignores an in-flight result after disposal', async () => {
  let resolveStatus!: (result: DesktopInvokeResult<unknown>) => void;
  let markRequested!: () => void;
  const requested = new Promise<void>((resolve) => {
    markRequested = resolve;
  });
  const invoke: NativeInvoke = async (command) => {
    if (command !== 'matrix_sync_status') return unavailable;
    markRequested();
    return new Promise<DesktopInvokeResult<unknown>>((resolve) => {
      resolveStatus = resolve;
    });
  };
  const client = createNativeMatrixClient(invoke);
  const seen: unknown[] = [];
  client.on('sync', (state) => seen.push(state));
  const unwatch = client.watchSync(10_000);
  await requested;
  unwatch();
  resolveStatus(ok({ readiness: 'running', sessionGeneration: 4, offlineModeEnabled: false }));
  await new Promise((resolve) => setTimeout(resolve, 0));

  assert.deepEqual(seen, []);
  assert.equal(client.getSyncState(), null);
});

test('watchSync never overlaps a slow native status read', async () => {
  let calls = 0;
  let resolveStatus!: (result: DesktopInvokeResult<unknown>) => void;
  const invoke: NativeInvoke = async (command) => {
    if (command !== 'matrix_sync_status') return unavailable;
    calls += 1;
    return new Promise<DesktopInvokeResult<unknown>>((resolve) => {
      resolveStatus = resolve;
    });
  };
  const client = createNativeMatrixClient(invoke);
  const unwatch = client.watchSync(5);
  await new Promise((resolve) => setTimeout(resolve, 25));
  assert.equal(calls, 1);

  unwatch();
  resolveStatus(ok({ readiness: 'running', sessionGeneration: 4, offlineModeEnabled: false }));
});

test('room-list bridge hydrates the facade before atom writes and ClientRoot owns one watcher', () => {
  const roomListSource = readFileSync('src/app/state/room-list/roomList.ts', 'utf8');
  const successfulSnapshotStart = roomListSource.indexOf(
    'latestNativeRoomListSnapshot = snapshot;'
  );
  const successfulSnapshotBranch = roomListSource.slice(successfulSnapshotStart);
  const applyIndex = successfulSnapshotBranch.indexOf('onSnapshot?.(snapshot);');
  const snapshotAtomIndex = successfulSnapshotBranch.indexOf('setSnapshot(snapshot);');
  const roomsAtomIndex = successfulSnapshotBranch.indexOf(
    "setRooms({ type: 'INITIALIZE', rooms: snapshot.orderedRoomIds });"
  );
  assert.ok(
    applyIndex >= 0 && applyIndex < snapshotAtomIndex && snapshotAtomIndex < roomsAtomIndex
  );
  assert.ok(
    roomListSource.indexOf('onSessionSnapshot?.(session);') < successfulSnapshotStart + applyIndex
  );

  const roomSelectorsSource = readFileSync('src/app/state/hooks/roomList.ts', 'utf8');
  assert.ok(roomSelectorsSource.includes('useNativeRoomListSnapshot'));
  assert.ok(roomSelectorsSource.includes('nativeSnapshot'));

  const clientRootSource = readFileSync('src/app/pages/client/ClientRoot.tsx', 'utf8');
  assert.equal(clientRootSource.match(/\.watchSync\(/g)?.length, 1);
  const syncHookSource = readFileSync('src/app/hooks/useSyncState.ts', 'utf8');
  assert.equal(syncHookSource.includes('watchSync'), false);
});

test('F2 getRooms proxies matrix_room_list_snapshot and maps summaries', async () => {
  const summary = {
    roomId: '!r:example.org',
    name: 'Engineering',
    canonicalAlias: '#eng:example.org',
    avatarUrl: 'mxc://example.org/av',
    membership: 'join',
    isDirect: false,
    isSpace: false,
    isEncrypted: true,
    joinRule: 'invite',
    unreadCount: 2,
    highlightCount: 1,
    markedUnread: false,
    lastActivityTs: 1234,
  };
  const { invoke } = invokingWith({
    matrix_room_list_snapshot: { sessionGeneration: 8, rooms: [summary] },
  });
  const client = createNativeMatrixClient(invoke);
  await client.refresh();
  const rooms = client.getRooms();
  assert.equal(rooms.length, 1);
  assert.equal(rooms[0].roomId, '!r:example.org');
  assert.equal(rooms[0].name, 'Engineering');
  assert.equal(rooms[0].getMyMembership(), 'join');
  assert.equal(rooms[0].isSpaceRoom(), false);
  assert.equal(rooms[0].getCanonicalAlias(), '#eng:example.org');
});

test('F2 getRoom finds a single room by id or null', async () => {
  const { invoke } = invokingWith({
    matrix_room_list_snapshot: {
      sessionGeneration: 8,
      rooms: [
        {
          roomId: '!a:example.org',
          name: 'A',
          membership: 'join',
          isDirect: false,
          isSpace: false,
          isEncrypted: false,
          unreadCount: 0,
          highlightCount: 0,
          markedUnread: false,
        },
        {
          roomId: '!b:example.org',
          name: 'B',
          membership: 'join',
          isDirect: true,
          isSpace: false,
          isEncrypted: false,
          unreadCount: 0,
          highlightCount: 0,
          markedUnread: false,
        },
      ],
    },
  });
  const client = createNativeMatrixClient(invoke);
  await client.refresh();
  assert.equal(client.getRoom('!b:example.org')?.name, 'B');
  assert.equal(client.getRoom('!missing:example.org'), null);
});

test('F2 getRooms fails closed when the command is unavailable', async () => {
  const client = createNativeMatrixClient(async () => unavailable);
  await client.refresh();
  assert.deepEqual(client.getRooms(), []);
  assert.equal(client.getRoom('!r:example.org'), null);
});

test('room-list snapshots update held facade wrappers and clear a valid empty list', async () => {
  let snapshot = roomSnapshot({ name: 'Before', unreadCount: 1 });
  const invoke: NativeInvoke = async (command) => {
    if (command === 'matrix_room_list_snapshot') return ok(snapshot);
    return unavailable;
  };
  const client = createNativeMatrixClient(invoke);
  await client.refresh();
  const heldRoom = client.getRoom('!r:example.org');
  assert.ok(heldRoom);

  const updated = roomSnapshot({ name: 'After', unreadCount: 4, isSpace: true });
  client.applyRoomListSnapshot(updated);

  assert.equal(client.getRoom('!r:example.org'), heldRoom);
  assert.equal(heldRoom.name, 'After');
  assert.equal(heldRoom.getUnreadNotificationCount(), 4);
  assert.equal(heldRoom.isSpaceRoom(), true);

  snapshot = { sessionGeneration: 8, rooms: [] };
  await client.refresh();
  assert.deepEqual(client.getRooms(), []);
  assert.equal(client.getRoom('!r:example.org'), null);
});

test('a native Space summary is classified without a fabricated create event', () => {
  const client = createNativeMatrixClient(async () => unavailable);
  client.applyRoomListSnapshot(roomSnapshot({ isSpace: true }));
  const space = client.getRoom('!r:example.org');

  assert.ok(space);
  assert.equal(isSpace(space), true);
  assert.equal(isRoom(space), false);
});

test('a native logged-out transition clears facade identity once and notifies the session listener', async () => {
  let session: unknown = {
    status: 'logged_in',
    user_id: '@alice:example.org',
    device_id: 'DEVICE',
    homeserver_url: 'https://matrix.example.org',
    sessionGeneration: 8,
  };
  const invoke: NativeInvoke = async (command) => {
    if (command === 'matrix_session_snapshot') return ok(session);
    if (command === 'matrix_room_list_snapshot') return ok(roomSnapshot());
    return unavailable;
  };
  const client = createNativeMatrixClient(invoke);
  await client.refresh();
  assert.equal(client.getUserId(), '@alice:example.org');
  assert.ok(client.getRoom('!r:example.org'));

  let loggedOutEvents = 0;
  client.on('Session.logged_out', () => {
    loggedOutEvents += 1;
  });
  session = { status: 'logged_out' };
  await client.refresh();

  assert.equal(client.getUserId(), null);
  assert.equal(client.getSafeUserId(), '');
  assert.deepEqual(client.getRooms(), []);
  assert.equal(loggedOutEvents, 1);

  await client.refresh();
  assert.equal(loggedOutEvents, 1);
});

test('an invalid session snapshot preserves a previously hydrated identity', async () => {
  let session: unknown = {
    status: 'logged_in',
    user_id: '@alice:example.org',
    device_id: 'DEVICE',
    homeserver_url: 'https://matrix.example.org',
    sessionGeneration: 8,
  };
  const invoke: NativeInvoke = async (command) =>
    command === 'matrix_session_snapshot' ? ok(session) : unavailable;
  const client = createNativeMatrixClient(invoke);
  await client.refresh();
  session = { status: 'unexpected' };
  await client.refresh();

  assert.equal(client.getUserId(), '@alice:example.org');
});

test('a replacement session clears old rooms and rejects a stale-generation snapshot', async () => {
  let snapshot = roomSnapshot({ name: 'Alice room' });
  const session: unknown = {
    status: 'logged_in',
    user_id: '@alice:example.org',
    device_id: 'ALICE',
    homeserver_url: 'https://matrix.example.org',
    sessionGeneration: 8,
  };
  const invoke: NativeInvoke = async (command) => {
    if (command === 'matrix_session_snapshot') return ok(session);
    if (command === 'matrix_room_list_snapshot') return ok(snapshot);
    return unavailable;
  };
  const client = createNativeMatrixClient(invoke);
  await client.refresh();
  assert.ok(client.getRoom('!r:example.org'));

  const sessionEvents: unknown[] = [];
  const loggedOutEvents: unknown[] = [];
  client.on('session', (event) => sessionEvents.push(event));
  client.on('Session.logged_out', (event) => loggedOutEvents.push(event));
  const replacement = {
    status: 'logged_in' as const,
    userId: '@bob:example.org',
    deviceId: 'BOB',
    homeserverUrl: 'https://matrix.example.org',
    sessionGeneration: 9,
  };
  client.applyNativeSessionSnapshot(replacement);

  assert.equal(client.getUserId(), '@bob:example.org');
  assert.equal(client.getRoom('!r:example.org'), null);
  assert.deepEqual(sessionEvents, [replacement]);
  assert.deepEqual(loggedOutEvents, []);

  // The preceding A-generation snapshot must not repopulate B's cache.
  client.applyRoomListSnapshot(snapshot);
  assert.equal(client.getRoom('!r:example.org'), null);
  snapshot = {
    ...roomSnapshot({ name: 'Bob room' }),
    sessionGeneration: 9,
  };
  client.applyRoomListSnapshot(snapshot);
  assert.equal(client.getRoom('!r:example.org')?.name, 'Bob room');
});

test('logout clears the facade identity after the native command succeeds', async () => {
  const { invoke } = invokingWith({
    matrix_session_snapshot: {
      status: 'logged_in',
      user_id: '@alice:example.org',
      device_id: 'DEVICE',
      homeserver_url: 'https://matrix.example.org',
      sessionGeneration: 8,
    },
    matrix_logout: { status: 'ok' },
  });
  const client = createNativeMatrixClient(invoke);
  await client.refresh();
  await client.logout();

  assert.equal(client.getUserId(), null);
  assert.equal(client.getSafeUserId(), '');
});

test('F2 fetchRoomEvent proxies matrix_timeline_event_readback', async () => {
  const { invoke } = invokingWith({
    matrix_timeline_event_readback: {
      sessionGeneration: 8,
      roomId: '!r:example.org',
      eventId: '$evt1',
      item: {
        itemId: 'i1',
        eventId: '$evt1',
        sender: '@alice:example.org',
        type: 'm.room.message',
        body: 'hello',
        originServerTs: 999,
      },
    },
  });
  const client = createNativeMatrixClient(invoke);
  const evt = await client.fetchRoomEvent('!r:example.org', '$evt1');
  assert.equal(evt?.eventId, '$evt1');
  assert.equal(evt?.sender, '@alice:example.org');
  assert.equal(evt?.type, 'm.room.message');
  assert.equal(evt?.body, 'hello');
});

test('F2 fetchRoomEvent returns null on unavailable command', async () => {
  const client = createNativeMatrixClient(async () => unavailable);
  assert.equal(await client.fetchRoomEvent('!r:example.org', '$evt1'), null);
});

test('F3 sendMessage proxies matrix_send_text', async () => {
  const { invoke, callLog } = invokingWith({
    matrix_send_text: {
      roomId: '!r:example.org',
      eventId: '$e1',
      localTxnId: 't1',
      status: 'sent',
    },
  });
  const client = createNativeMatrixClient(invoke);
  const sent = await client.sendMessage('!r:example.org', { body: 'hello' });
  assert.equal(sent?.eventId, '$e1');
  assert.deepEqual(callLog, ['matrix_send_text']);
});

test('F3 sendMessage fails closed when the command is unavailable', async () => {
  const client = createNativeMatrixClient(async () => unavailable);
  assert.equal(await client.sendMessage('!r:example.org', { body: 'x' }), null);
});

test('F3 sendEvent tunnels m.room.message to send_text and GAPs other types', async () => {
  const { invoke } = invokingWith({
    matrix_send_text: {
      roomId: '!r:example.org',
      eventId: '$msg',
      localTxnId: 't2',
      status: 'sent',
    },
  });
  const client = createNativeMatrixClient(invoke);
  const msg = await client.sendEvent('!r:example.org', 'm.room.message', { body: 'hi' });
  assert.equal(msg?.eventId, '$msg');
  const gap = await client.sendEvent('!r:example.org', 'm.custom.type', {});
  assert.equal(gap, null);
});

test('F3 sendStateEvent maps covered room-state types, GAPs others', async () => {
  const { invoke, callLog } = invokingWith({
    matrix_set_room_name: { status: 'ok', roomId: '!r:example.org', sessionGeneration: 8 },
  });
  const client = createNativeMatrixClient(invoke);
  const name = await client.sendStateEvent('!r:example.org', 'm.room.name', { name: 'New' });
  assert.equal(name?.status, 'ok');
  const gap = await client.sendStateEvent('!r:example.org', 'm.custom', {});
  assert.equal(gap, null);
  assert.deepEqual(callLog, ['matrix_set_room_name']);
});

test('F3 account-data methods are documented GAP (fail-closed)', async () => {
  const client = createNativeMatrixClient(async () => unavailable);
  assert.equal(client.getAccountData('m.tag'), undefined);
  assert.equal(await client.setAccountData('m.tag', {}), null);
  assert.equal(await client.setRoomAccountData('!r:example.org', 'm.tag', {}), null);
});

test('F3 D1C still holds: no token surface after send/state additions', async () => {
  const client = createNativeMatrixClient(async () => unavailable) as unknown as Record<
    string,
    unknown
  >;
  assert.equal('getAccessToken' in client, false);
  assert.equal('setAccessToken' in client, false);
  assert.equal('refreshToken' in client, false);
});

test('F4 uploadContent proxies matrix_upload_media', async () => {
  const { invoke, callLog } = invokingWith({
    matrix_upload_media: { mxc: 'mxc://example.org/up1' },
  });
  const client = createNativeMatrixClient(invoke);
  const uploaded = await client.uploadContent({ mimeType: 'image/png', bytes: [1, 2, 3] });
  assert.equal(uploaded?.mxc, 'mxc://example.org/up1');
  assert.deepEqual(callLog, ['matrix_upload_media']);
});

test('F4 uploadContent throws when unavailable (js-sdk non-null contract)', async () => {
  const client = createNativeMatrixClient(async () => unavailable);
  await assert.rejects(() => client.uploadContent({ mimeType: 'x', bytes: [1] }));
});

test('F4 getMediaConfig reads m.upload.size wire key', async () => {
  const { invoke } = invokingWith({
    matrix_media_config: { 'm.upload.size': 10485760 },
  });
  const client = createNativeMatrixClient(invoke);
  assert.deepEqual(await client.getMediaConfig(), { maxUploadSizeBytes: 10485760 });
});

test('F4 getMediaConfig fails closed (empty) when unavailable', async () => {
  const client = createNativeMatrixClient(async () => unavailable);
  assert.deepEqual(await client.getMediaConfig(), {});
});

test('F4 downloadMedia proxies matrix_media_download', async () => {
  const { invoke } = invokingWith({
    matrix_media_download: { bytes: [10, 20, 30] },
  });
  const client = createNativeMatrixClient(invoke);
  const dl = await client.downloadMedia('mxc://example.org/f1');
  assert.deepEqual(dl?.bytes, [10, 20, 30]);
});

test('P4-S36 downloadMedia prefers timeline handles over leftover mxc', async () => {
  const seen: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const { invoke } = invokingWith({
    matrix_media_download: { bytes: [7, 8, 9] },
  });
  const client = createNativeMatrixClient(async (command, args) => {
    seen.push({ command, args });
    return invoke(command, args);
  });
  const handle = `timeline-media-${'ab'.repeat(32)}`;
  const dl = await client.downloadMedia(`synara-media://localhost/${handle}`);
  assert.deepEqual(dl?.bytes, [7, 8, 9]);
  assert.equal(seen[0]?.command, 'matrix_media_download');
  assert.equal(seen[0]?.args?.contentUri, handle);
  assert.notEqual(seen[0]?.args?.contentUri, 'mxc://example.org/f1');
});

test('F4 getProfileInfo loads own profile from the native owner', async () => {
  const { invoke, callLog } = invokingWith({
    matrix_session_snapshot: {
      status: 'logged_in',
      user_id: '@alice:example.org',
      device_id: 'DEV',
      homeserver_url: 'https://matrix.example.org',
      sessionGeneration: 5,
    },
    matrix_get_own_profile: {
      userId: '@alice:example.org',
      displayName: 'Alice',
      avatarUrl: 'mxc://example.org/avatar',
    },
  });
  const client = createNativeMatrixClient(invoke);
  await client.refresh();
  const profile = await client.getProfileInfo('@alice:example.org');
  assert.equal(profile.avatar_url, 'mxc://example.org/avatar');
  assert.equal(profile.displayname, 'Alice');
  assert.equal(callLog.includes('matrix_get_own_profile'), true);
});

test('F4 getProfileInfo rejects non-mxc avatars', async () => {
  const { invoke } = invokingWith({
    matrix_session_snapshot: {
      status: 'logged_in',
      user_id: '@alice:example.org',
      device_id: 'DEV',
      homeserver_url: 'https://matrix.example.org',
      sessionGeneration: 5,
    },
    matrix_get_own_profile: {
      userId: '@alice:example.org',
      displayName: 'Alice',
      avatarUrl: 'data:image/png;base64,AAAA',
    },
  });
  const client = createNativeMatrixClient(invoke);
  await client.refresh();
  const profile = await client.getProfileInfo('@alice:example.org');
  assert.equal(profile.avatar_url, undefined);
  assert.equal(profile.displayname, 'Alice');
});
test('F5 getIdentity snapshot preserves userId/deviceId', async () => {
  const { invoke } = invokingWith({
    matrix_session_snapshot: {
      status: 'logged_in',
      user_id: '@alice:example.org',
      device_id: 'DEV',
      homeserver_url: 'https://matrix.example.org',
      sessionGeneration: 5,
    },
  });
  const client = createNativeMatrixClient(invoke);
  await client.refresh();
  assert.equal(client.getIdentity().userId, '@alice:example.org');
  assert.equal(client.getIdentity().deviceId, 'DEV');
});

test('F5 getCryptoStatus proxies matrix_crypto_status (no keys, D1C)', async () => {
  const { invoke, callLog } = invokingWith({
    matrix_crypto_status: {
      sessionGeneration: 7,
      encryptionEnabled: true,
      crossSigningState: 'Ready',
    },
  });
  const client = createNativeMatrixClient(invoke);
  const crypto = await client.getCryptoStatus();
  assert.equal(crypto?.encryptionEnabled, true);
  assert.equal(crypto?.crossSigningState, 'Ready');
  assert.equal(callLog[0], 'matrix_crypto_status');
});

test('F5 getCrypto returns status-backed, key-free reading', async () => {
  const { invoke } = invokingWith({
    matrix_crypto_status: {
      sessionGeneration: 3,
      encryptionEnabled: true,
      crossSigningState: 'Ready',
    },
  });
  const client = createNativeMatrixClient(invoke);
  const crypto = client.getCrypto();
  assert.equal(await crypto.isCrossSigningReady(), true);
  assert.equal(await crypto.isEncryptionEnabled(), true);
  assert.equal(await crypto.getCrossSigningState(), 'Ready');
});

test('F5 getCryptoStatus fails closed when unavailable', async () => {
  const client = createNativeMatrixClient(async () => unavailable);
  assert.equal(await client.getCryptoStatus(), null);
  const crypto = client.getCrypto();
  assert.equal(await crypto.isCrossSigningReady(), false);
});

test('F5 decryptEventIfNeeded is a documented no-op (native events pre-decrypted)', async () => {
  const client = createNativeMatrixClient(async () => unavailable);
  await client.decryptEventIfNeeded({ eventId: '$e' });
  assert.ok(true);
});

test('F5 downloadKeysForUsers is a D1C-key-free stub', async () => {
  const client = createNativeMatrixClient(async () => unavailable);
  assert.deepEqual(await client.downloadKeysForUsers(['@alice:example.org']), {});
});

test('F5 extended GAP stubs satisfy the anchor without data', async () => {
  const client = createNativeMatrixClient(async () => unavailable);
  assert.deepEqual(await client.getCapabilities(), {});
  assert.equal(await client.getOpenIdToken(), null);
  assert.equal(await client.search({ body: {}, next_batch: undefined }), null);
});

test('F6c redactEvent proxies matrix_timeline_redact', async () => {
  const { invoke, callLog } = invokingWith({
    matrix_timeline_redact: { event_id: '$evt1', room_id: '!r:example.org' },
  });
  const client = createNativeMatrixClient(invoke);
  const redacted = await client.redactEvent('!r:example.org', '$evt1', 'spam');
  assert.equal(redacted?.event_id, '$evt1');
  assert.equal(callLog[0], 'matrix_timeline_redact');
});

test('F6c redactEvent fails closed when unavailable', async () => {
  const client = createNativeMatrixClient(async () => unavailable);
  assert.equal(await client.redactEvent('!r:example.org', '$evt1'), null);
});

test('F6c GAP-safe stubs (searchUserDirectory, queueToDevice, delayed events)', async () => {
  const client = createNativeMatrixClient(async () => unavailable);
  assert.deepEqual(await client.searchUserDirectory({ term: 'term' }), {
    limited: false,
    results: [],
  });
  assert.deepEqual(await client.searchUserDirectoryFn('term'), {
    limited: false,
    results: [],
  });
  await client.queueToDevice({ eventType: 'm.test', batch: [] });
  assert.equal(await client._unstable_sendDelayedEvent('!r', {}, null, 'm.test', {}), null);
  assert.equal(await client._unstable_sendDelayedStateEvent('!r', {}, 'm.test', {}, '$k'), null);
  assert.equal(await client._unstable_updateDelayedEvent('$evt', '!r', {}, {}), null);
  assert.equal(await client.getOpenIdTokenData(), null);
});

test('searchUserDirectory maps native user-directory hits and omits non-mxc avatars', async () => {
  const { invoke, callLog } = invokingWith({
    matrix_user_directory_search: {
      limited: true,
      results: [
        {
          userId: '@bob:example.org',
          displayName: 'Bob',
          avatarUrl: 'mxc://example.org/abc',
        },
        {
          userId: '@eve:example.org',
          displayName: 'Eve',
          avatarUrl: 'data:image/png;base64,AAAA',
        },
      ],
    },
  });
  const client = createNativeMatrixClient(invoke);
  const listed = {
    limited: true,
    results: [
      {
        user_id: '@bob:example.org',
        display_name: 'Bob',
        avatar_url: 'mxc://example.org/abc',
      },
      {
        user_id: '@eve:example.org',
        display_name: 'Eve',
        avatar_url: undefined,
      },
    ],
  };
  assert.deepEqual(await client.searchUserDirectory({ term: 'bo', limit: 10 }), listed);
  assert.equal(callLog[0], 'matrix_user_directory_search');
  assert.deepEqual(await client.searchUserDirectoryFn('bo'), listed);
  assert.equal(callLog[1], 'matrix_user_directory_search');
});

test('F6c crypto reading includes encryptToDeviceMessages no-op', async () => {
  const client = createNativeMatrixClient(async () => unavailable);
  const crypto = client.getCrypto();
  const batch = await crypto.encryptToDeviceMessages(
    'm.test',
    [{ userId: '@u:example.org', deviceId: 'D' }],
    {}
  );
  assert.deepEqual(batch, []);
  assert.ok(true);
});

test('F6c-2a evented room cache satisfies EventedRoomReading contract', async () => {
  const { invoke } = invokingWith({
    matrix_room_list_snapshot: {
      sessionGeneration: 8,
      rooms: [
        {
          roomId: '!r:example.org',
          name: 'Test Room',
          membership: 'join',
          isDirect: false,
          isSpace: false,
          isEncrypted: false,
          unreadCount: 0,
          highlightCount: 0,
          markedUnread: false,
        },
      ],
    },
    matrix_sync_status: { readiness: 'Prepared', session_generation: 1, failure: null },
    matrix_session_snapshot: {},
  });
  const client = createNativeMatrixClient(invoke);
  await client.refresh();
  const room = client.getRoom('!r:example.org');
  assert.ok(room);
  assert.ok(typeof room?.on === 'function');
  assert.ok(typeof room?.removeListener === 'function');
  assert.deepEqual(room?.getUsersReadUpTo({} as never), []);
  assert.equal(room?.findEventById('$e'), undefined);
  assert.equal(room?.hasEncryptionStateEvent(), false);
});

test('F6c-2a GAP stub batch (user/pusher/alias/upload/verification)', async () => {
  const client = createNativeMatrixClient(async () => unavailable);
  assert.equal(await client.getUser('@u:example.org'), null);
  assert.equal(await client.getThreePids(), null);
  assert.equal(await client.getPushers(), null);
  await client.setPusher({});
  assert.deepEqual(await client.getLocalAliases('!r'), { aliases: [] });
  assert.equal(await client.createAlias('#a', '!r'), null);
  assert.equal(await client.deleteAlias('#a'), null);
  assert.equal(await client.cancelUpload('tok'), null);
  assert.equal(await client.getBaseUrl(), null);
  assert.equal(await client.setRoomReadMarkers('!r', '$e'), null);
  assert.equal(await client.sendReadReceipt({}), null);
  assert.equal(await client.getLatestTimeline(undefined), null);
});

test('F6c-2a crypto reading exposes getOwnDeviceKeys continuity surfaceless stub', async () => {
  const client = createNativeMatrixClient(async () => unavailable);
  const crypto = client.getCrypto() as {
    getOwnDeviceKeys?(): Promise<{ ed25519: string; curve25519: string }>;
  };
  assert.ok(!crypto.getOwnDeviceKeys, 'D1C: renderer crypto must not expose own-device keys');
});
