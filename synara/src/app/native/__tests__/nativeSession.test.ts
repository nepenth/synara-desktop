import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import type { DesktopInvokeResult } from '../../utils/desktop';
import { isRoom, isSpace } from '../../utils/room';
import { createNativeSession } from '../nativeSession';
import {
  readinessToSyncState,
  type NativeInvoke,
  type NativeRoomListSnapshot,
} from '../nativeWire';

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

test('readinessToSyncState maps Rust readiness to connection states', () => {
  assert.equal(readinessToSyncState('running'), 'PREPARED');
  assert.equal(readinessToSyncState('running', 'open'), 'PREPARED');
  assert.equal(readinessToSyncState('running', 'closed'), 'ERROR');
  assert.equal(readinessToSyncState('offline', 'closed'), 'ERROR');
  assert.equal(readinessToSyncState('offline'), 'RECONNECTING');
  assert.equal(readinessToSyncState('failed'), 'ERROR');
  assert.equal(readinessToSyncState('idle'), 'STOPPED');
  assert.equal(readinessToSyncState('unconfigured'), 'STOPPED');
  assert.equal(readinessToSyncState('terminated'), 'ERROR');
});

/**
 * Shared connection-status table. The identical rows live in
 * synara-ios/SynaraTests/ConnectionStatusCopyTests.swift
 * (`sharedConnectionStatusCases`); the last test below keeps them in step.
 * Columns: readiness, command gate, connected earlier in this session, banner.
 */
const SHARED_CONNECTION_STATUS_CASES: ReadonlyArray<
  readonly [string, string, boolean, 'connected' | 'reconnecting' | 'lost' | 'cold']
> = [
  ['running', 'open', false, 'connected'],
  ['running', 'open', true, 'connected'],
  ['running', 'closed', false, 'lost'],
  ['running', 'closed', true, 'lost'],
  ['running', 'unexpected', true, 'lost'],
  ['offline', 'open', true, 'reconnecting'],
  ['failed', 'open', false, 'lost'],
  ['failed', 'open', true, 'lost'],
  ['terminated', 'open', false, 'lost'],
  ['terminated', 'open', true, 'lost'],
  ['idle', 'open', false, 'cold'],
  ['idle', 'open', true, 'lost'],
  ['unconfigured', 'open', false, 'cold'],
  ['unconfigured', 'open', true, 'lost'],
];

const desktopBannerClass = (
  readiness: string,
  gate: string,
  connectedEarlier: boolean
): 'connected' | 'reconnecting' | 'lost' | 'cold' => {
  const state = readinessToSyncState(
    readiness as Parameters<typeof readinessToSyncState>[0],
    gate as Parameters<typeof readinessToSyncState>[1]
  );
  if (state === 'PREPARED') return 'connected';
  if (state === 'RECONNECTING') return 'reconnecting';
  if (state === 'ERROR') return 'lost';
  return connectedEarlier ? 'lost' : 'cold';
};

test('desktop connection status follows the shared desktop/iOS table', () => {
  for (const [readiness, gate, connectedEarlier, expected] of SHARED_CONNECTION_STATUS_CASES) {
    assert.equal(
      desktopBannerClass(readiness, gate, connectedEarlier),
      expected,
      `${readiness}/${gate}/${connectedEarlier}`
    );
  }
});

test('the iOS connection status tests carry the same shared table', () => {
  const swift = readFileSync('../synara-ios/SynaraTests/ConnectionStatusCopyTests.swift', 'utf8');
  const table =
    swift.split('sharedConnectionStatusCases')[1]?.split('= [')[1]?.split('\n    ]')[0] ?? '';
  const rows = [...table.matchAll(/\("(\w+)", "(\w+)", (true|false), \.(\w+)\)/g)].map(
    ([, readiness, gate, connectedEarlier, expected]) =>
      [readiness, gate, connectedEarlier === 'true', expected] as const
  );
  assert.deepEqual(rows, SHARED_CONNECTION_STATUS_CASES);
});

test('getSyncState proxies matrix_sync_status and caches PREPARED when running', async () => {
  const { invoke } = invokingWith({
    matrix_sync_status: {
      readiness: 'running',
      sessionGeneration: 7,
      offlineModeEnabled: false,
    },
  });
  const client = createNativeSession(invoke);
  await client.refresh();
  assert.equal(client.getSyncState(), 'PREPARED');
  assert.equal(client.isSyncRunning(), true);
  assert.deepEqual(client.getSyncStateData(), {
    readiness: 'running',
    sessionGeneration: 7,
    failureDiagnosticId: null,
    slidingSyncCapable: null,
  });
});

const LOGGED_IN_SESSION = {
  status: 'logged_in',
  userId: '@alice:example.org',
  deviceId: 'DEVICE',
  homeserverUrl: 'https://example.org',
  sessionGeneration: 7,
};

test('a closed command gate is ERROR even while readiness is running', async () => {
  const { invoke } = invokingWith({
    matrix_session_snapshot: LOGGED_IN_SESSION,
    matrix_sync_status: {
      readiness: 'running',
      sessionGeneration: 7,
      offlineModeEnabled: false,
      commandGate: 'closed',
    },
  });
  const client = createNativeSession(invoke);
  await client.refresh();
  assert.equal(client.hasSignedInSession(), true);
  assert.equal(client.getSyncState(), 'ERROR');
});

test('an unknown command gate fails closed and a missing gate stays open', async () => {
  const closed = invokingWith({
    matrix_session_snapshot: LOGGED_IN_SESSION,
    matrix_sync_status: {
      readiness: 'running',
      sessionGeneration: 7,
      offlineModeEnabled: false,
      commandGate: 'https://private.example/token',
    },
  });
  const closedClient = createNativeSession(closed.invoke);
  await closedClient.refresh();
  assert.equal(closedClient.getSyncState(), 'ERROR');

  const open = invokingWith({
    matrix_session_snapshot: LOGGED_IN_SESSION,
    matrix_sync_status: {
      readiness: 'running',
      sessionGeneration: 7,
      offlineModeEnabled: false,
    },
  });
  const openClient = createNativeSession(open.invoke);
  await openClient.refresh();
  assert.equal(openClient.getSyncState(), 'PREPARED');
});

test('a closed gate without a signed-in session is not connection loss', async () => {
  const { invoke } = invokingWith({
    matrix_session_snapshot: { status: 'logged_out' },
    matrix_sync_status: {
      readiness: 'unconfigured',
      sessionGeneration: 7,
      offlineModeEnabled: false,
      commandGate: 'closed',
    },
  });
  const client = createNativeSession(invoke);
  await client.refresh();
  assert.equal(client.hasSignedInSession(), false);
  assert.notEqual(client.getSyncState(), 'ERROR');
});

test('voluntary logout does not paint Connection Lost while native tears down', async () => {
  let releaseLogout: (value: unknown) => void = () => undefined;
  const syncStatus = {
    readiness: 'running',
    sessionGeneration: 7,
    offlineModeEnabled: false,
    commandGate: 'open',
  };
  const invoke: NativeInvoke = async (command) => {
    if (command === 'matrix_session_snapshot') return ok(LOGGED_IN_SESSION);
    if (command === 'matrix_sync_status') return ok(syncStatus);
    if (command === 'matrix_logout') {
      return ok(
        await new Promise((resolve) => {
          releaseLogout = resolve;
        })
      );
    }
    return unavailable;
  };
  const client = createNativeSession(invoke);
  await client.refresh();
  assert.equal(client.getSyncState(), 'PREPARED');
  const emitted: unknown[] = [];
  client.subscribe('sync', (state: unknown) => emitted.push(state));

  const logout = client.logout();
  assert.equal(client.hasSignedInSession(), false, 'logging out is not a signed-in session');
  // Native has taken the session out and closed Core: unconfigured + closed.
  syncStatus.readiness = 'unconfigured';
  syncStatus.commandGate = 'closed';
  const stopWatching = client.watchSync(60_000);
  await new Promise((resolve) => setTimeout(resolve, 0));
  await client.refresh();
  assert.notEqual(client.getSyncState(), 'ERROR');
  assert.ok(!emitted.includes('ERROR'));

  releaseLogout({ status: 'logged_out' });
  await logout;
  stopWatching();
  assert.ok(!emitted.includes('ERROR'));
});

test('a failed logout restores the signed-in banner mapping', async () => {
  const { invoke } = invokingWith({
    matrix_session_snapshot: LOGGED_IN_SESSION,
    matrix_sync_status: {
      readiness: 'running',
      sessionGeneration: 7,
      offlineModeEnabled: false,
      commandGate: 'closed',
    },
    matrix_logout: { status: 'logged_in' },
  });
  const client = createNativeSession(invoke);
  await client.refresh();
  await assert.rejects(client.logout(), /did not complete/);
  assert.equal(client.hasSignedInSession(), true);
  await client.refresh();
  assert.equal(client.getSyncState(), 'ERROR');
});

test('stop before reload is not a signed-in session', async () => {
  const { invoke } = invokingWith({
    matrix_session_snapshot: LOGGED_IN_SESSION,
    matrix_sync_status: { readiness: 'running', sessionGeneration: 7, offlineModeEnabled: false },
  });
  const client = createNativeSession(invoke);
  await client.start();
  assert.equal(client.hasSignedInSession(), true);
  await client.stop();
  assert.equal(client.hasSignedInSession(), false);
  await client.start();
  assert.equal(client.hasSignedInSession(), true);
});

test('getSyncState fails closed when the native command is unavailable', async () => {
  const { invoke } = invokingWith({});
  const client = createNativeSession(invoke);
  await client.refresh();
  assert.equal(client.getSyncState(), null);
  assert.equal(client.isSyncRunning(), false);
});

test('retrySyncNow recovers native sync then refreshes status', async () => {
  const { invoke, callLog } = invokingWith({
    matrix_sync_recover: {
      readiness: 'running',
      sessionGeneration: 7,
      offlineModeEnabled: false,
    },
    matrix_sync_status: {
      readiness: 'running',
      sessionGeneration: 7,
      offlineModeEnabled: false,
    },
    matrix_session_snapshot: { status: 'logged_out' },
  });
  const client = createNativeSession(invoke);
  await client.retrySyncNow();
  assert.equal(callLog[0], 'matrix_sync_recover');
  assert.ok(callLog.includes('matrix_sync_status'));
});

test('retrySyncNow still refreshes status when recover is unavailable', async () => {
  const { invoke, callLog } = invokingWith({
    matrix_sync_status: {
      readiness: 'running',
      sessionGeneration: 7,
      offlineModeEnabled: false,
    },
    matrix_session_snapshot: { status: 'logged_out' },
  });
  const client = createNativeSession(invoke);
  await client.retrySyncNow();
  assert.equal(callLog[0], 'matrix_sync_recover');
  assert.ok(callLog.includes('matrix_sync_status'));
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
    const client = createNativeSession(invoke);
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
  const client = createNativeSession(invoke);
  await client.refresh();
  assert.equal(client.getSyncState(), 'STOPPED');
  assert.equal(client.getSyncStateData()?.slidingSyncCapable, null);
});

test('sync subscribers receive each state and unsubscribe detaches', async () => {
  const { invoke } = invokingWith({
    matrix_sync_status: { readiness: 'failed', sessionGeneration: 3, offlineModeEnabled: false },
  });
  const client = createNativeSession(invoke);
  const seen: unknown[] = [];
  const listener = (payload: unknown): void => {
    seen.push(payload);
  };
  const unsubscribe = client.subscribe('sync', listener);
  await client.refresh();
  assert.deepEqual(seen, ['ERROR']);
  unsubscribe();
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
  const client = createNativeSession(invoke);
  await client.refresh();
  assert.equal(client.getIdentity().userId ?? null, '@alice:example.org');
  assert.equal(client.getIdentity().userId ?? '', '@alice:example.org');
  assert.equal(client.getIdentity().deviceId, 'DEVICE');
});

test('identity is empty when the session is logged out', async () => {
  const { invoke } = invokingWith({ matrix_session_snapshot: { status: 'logged_out' } });
  const client = createNativeSession(invoke);
  await client.refresh();
  assert.equal(client.getIdentity().userId ?? null, null);
  assert.equal(client.getIdentity().userId ?? '', '');
});

test('D1C: the native session exposes no token surface (renderer cedes custody)', async () => {
  const client = createNativeSession(async () => unavailable);
  const surface = client as unknown as Record<string, unknown>;
  assert.equal('getAccessToken' in surface, false);
  assert.equal('setAccessToken' in surface, false);
  assert.equal('refreshToken' in surface, false);
});

test('watchSync emits on readiness change into PREPARED', async () => {
  let readiness: 'idle' | 'running' = 'idle';
  const invokeDyn: NativeInvoke = async (command) => {
    if (command === 'matrix_sync_status') {
      return ok({ readiness, sessionGeneration: 4, offlineModeEnabled: false });
    }
    return unavailable;
  };
  const client = createNativeSession(invokeDyn);
  const seen: unknown[] = [];
  client.subscribe('sync', (payload) => seen.push(payload));
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
  const client = createNativeSession(invoke);
  const seen: unknown[] = [];
  client.subscribe('sync', (state) => seen.push(state));
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
  const client = createNativeSession(invoke);
  const unwatch = client.watchSync(5);
  await new Promise((resolve) => setTimeout(resolve, 25));
  assert.equal(calls, 1);

  unwatch();
  resolveStatus(ok({ readiness: 'running', sessionGeneration: 4, offlineModeEnabled: false }));
});

test('room-list bridge hydrates the session before atom writes and ClientRoot owns one watcher', () => {
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

test('getRooms reads matrix_room_list_snapshot and maps summaries', async () => {
  const summary = {
    roomId: '!r:example.org',
    name: 'Engineering',
    canonicalAlias: '#eng:example.org',
    avatarUrl: 'mxc://example.org/av',
    membership: 'join',
    isDirect: false,
    isSpace: false,
    isEncrypted: true,
    encryptionStatus: 'encrypted',
    joinRule: 'invite',
    unreadCount: 2,
    highlightCount: 1,
    markedUnread: false,
    lastMessageIsAgentApproval: false,
    lastActivityTs: 1234,
  };
  const { invoke } = invokingWith({
    matrix_room_list_snapshot: { sessionGeneration: 8, rooms: [summary] },
  });
  const client = createNativeSession(invoke);
  await client.refresh();
  const rooms = client.getRooms();
  assert.equal(rooms.length, 1);
  assert.equal(rooms[0].roomId, '!r:example.org');
  assert.equal(rooms[0].name, 'Engineering');
  assert.equal(rooms[0].getMyMembership(), 'join');
  assert.equal(rooms[0].isSpaceRoom(), false);
  assert.equal(rooms[0].getCanonicalAlias(), '#eng:example.org');
});

test('getRoom finds a single room by id or null', async () => {
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
          encryptionStatus: 'not_encrypted',
          unreadCount: 0,
          highlightCount: 0,
          markedUnread: false,
          lastMessageIsAgentApproval: false,
        },
        {
          roomId: '!b:example.org',
          name: 'B',
          membership: 'join',
          isDirect: true,
          isSpace: false,
          isEncrypted: false,
          encryptionStatus: 'not_encrypted',
          unreadCount: 0,
          highlightCount: 0,
          markedUnread: false,
          lastMessageIsAgentApproval: false,
        },
      ],
    },
  });
  const client = createNativeSession(invoke);
  await client.refresh();
  assert.equal(client.getRoom('!b:example.org')?.name, 'B');
  assert.equal(client.getRoom('!missing:example.org'), null);
});

test('getRooms fails closed when the command is unavailable', async () => {
  const client = createNativeSession(async () => unavailable);
  await client.refresh();
  assert.deepEqual(client.getRooms(), []);
  assert.equal(client.getRoom('!r:example.org'), null);
});

test('room-list snapshots update held rooms and clear a valid empty list', async () => {
  let snapshot = roomSnapshot({ name: 'Before', unreadCount: 1 });
  const invoke: NativeInvoke = async (command) => {
    if (command === 'matrix_room_list_snapshot') return ok(snapshot);
    return unavailable;
  };
  const client = createNativeSession(invoke);
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
  const client = createNativeSession(async () => unavailable);
  client.applyRoomListSnapshot(roomSnapshot({ isSpace: true }));
  const space = client.getRoom('!r:example.org');

  assert.ok(space);
  assert.equal(isSpace(space), true);
  assert.equal(isRoom(space), false);
});

test('a native logged-out transition clears identity once and notifies loggedOut subscribers', async () => {
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
  const client = createNativeSession(invoke);
  await client.refresh();
  assert.equal(client.getIdentity().userId ?? null, '@alice:example.org');
  assert.ok(client.getRoom('!r:example.org'));

  let loggedOutEvents = 0;
  client.subscribe('loggedOut', () => {
    loggedOutEvents += 1;
  });
  session = { status: 'logged_out' };
  await client.refresh();

  assert.equal(client.getIdentity().userId ?? null, null);
  assert.equal(client.getIdentity().userId ?? '', '');
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
  const client = createNativeSession(invoke);
  await client.refresh();
  session = { status: 'unexpected' };
  await client.refresh();

  assert.equal(client.getIdentity().userId ?? null, '@alice:example.org');
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
  const client = createNativeSession(invoke);
  await client.refresh();
  assert.ok(client.getRoom('!r:example.org'));

  const sessionEvents: unknown[] = [];
  const loggedOutEvents: unknown[] = [];
  client.subscribe('session', (event) => sessionEvents.push(event));
  client.subscribe('loggedOut', (event) => loggedOutEvents.push(event));
  const replacement = {
    status: 'logged_in' as const,
    userId: '@bob:example.org',
    deviceId: 'BOB',
    homeserverUrl: 'https://matrix.example.org',
    sessionGeneration: 9,
  };
  client.applySessionSnapshot(replacement);

  assert.equal(client.getIdentity().userId ?? null, '@bob:example.org');
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

test('logout clears the identity after the native command succeeds', async () => {
  const { invoke } = invokingWith({
    matrix_session_snapshot: {
      status: 'logged_in',
      user_id: '@alice:example.org',
      device_id: 'DEVICE',
      homeserver_url: 'https://matrix.example.org',
      sessionGeneration: 8,
    },
    matrix_logout: { status: 'logged_out' },
  });
  const client = createNativeSession(invoke);
  await client.refresh();
  await client.logout();

  assert.equal(client.getIdentity().userId ?? null, null);
  assert.equal(client.getIdentity().userId ?? '', '');
});

test('native logout rejects unavailable, malformed, and failed completion without clearing identity', async () => {
  for (const outcome of [
    unavailable,
    ok({ status: 'logged_in' }),
    ok(undefined),
    new Error('local deletion failed'),
  ]) {
    const client = createNativeSession(async (command) => {
      if (command === 'matrix_logout') {
        if (outcome instanceof Error) throw outcome;
        return outcome;
      }
      if (command === 'matrix_session_snapshot')
        return ok({
          status: 'logged_in',
          user_id: '@alice:example.org',
          device_id: 'DEVICE',
          homeserver_url: 'https://matrix.example.org',
          sessionGeneration: 8,
        });
      return unavailable;
    });
    await client.refresh();
    await assert.rejects(client.logout());
    assert.equal(client.getIdentity().userId ?? null, '@alice:example.org');
    assert.equal(client.getIdentity().userId ?? '', '@alice:example.org');
  }
});

test('getIdentity preserves userId and deviceId', async () => {
  const { invoke } = invokingWith({
    matrix_session_snapshot: {
      status: 'logged_in',
      user_id: '@alice:example.org',
      device_id: 'DEV',
      homeserver_url: 'https://matrix.example.org',
      sessionGeneration: 5,
    },
  });
  const client = createNativeSession(invoke);
  await client.refresh();
  assert.equal(client.getIdentity().userId, '@alice:example.org');
  assert.equal(client.getIdentity().deviceId, 'DEV');
});

test('native rooms satisfy the EventedRoomReading contract', async () => {
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
          encryptionStatus: 'not_encrypted',
          unreadCount: 0,
          highlightCount: 0,
          markedUnread: false,
          lastMessageIsAgentApproval: false,
        },
      ],
    },
    matrix_sync_status: { readiness: 'Prepared', session_generation: 1, failure: null },
    matrix_session_snapshot: {},
  });
  const client = createNativeSession(invoke);
  await client.refresh();
  const room = client.getRoom('!r:example.org');
  assert.ok(room);
  assert.ok(typeof room?.on === 'function');
  assert.ok(typeof room?.removeListener === 'function');
  assert.deepEqual(room?.getUsersReadUpTo({} as never), []);
  assert.equal(room?.findEventById('$e'), undefined);
  assert.equal(room?.hasEncryptionStateEvent(), false);
  assert.equal(room?.isDirect, false);
  assert.equal(room?.isEncrypted, false);
  assert.equal(typeof room?.currentState.events?.forEach, 'function');
  assert.equal(typeof (room?.accountData as { entries?: unknown }).entries, 'function');
});

test('confirmed native logout clears identity even when a stopped-sync listener throws', async () => {
  const { invoke } = invokingWith({
    matrix_session_snapshot: {
      status: 'logged_in',
      userId: '@alice:example.org',
      deviceId: 'DEVICE',
      homeserverUrl: 'https://matrix.example.org',
      sessionGeneration: 7,
    },
    matrix_sync_status: { readiness: 'running', sessionGeneration: 7 },
    matrix_logout: { status: 'logged_out' },
  });
  const client = createNativeSession(invoke);
  await client.refresh();
  assert.equal(client.getSessionGeneration(), 7);
  client.subscribe('sync', () => {
    throw new Error('renderer listener failed');
  });
  await client.logout();
  assert.equal(client.getIdentity().userId ?? '', '');
  assert.equal(client.getSessionGeneration(), undefined);
});
