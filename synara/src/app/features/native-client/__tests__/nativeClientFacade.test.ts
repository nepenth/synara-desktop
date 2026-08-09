import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { DesktopInvokeResult } from '../../../utils/desktop';
import {
  createNativeMatrixClient,
  readinessToSyncState,
  type NativeInvoke,
} from '../nativeClientFacade';

const ok = (value: unknown): DesktopInvokeResult<unknown> => ({ available: true, value });
const unavailable: DesktopInvokeResult<unknown> = { available: false };

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
  });
});

test('getSyncState fails closed when the native command is unavailable', async () => {
  const { invoke } = invokingWith({});
  const client = createNativeMatrixClient(invoke);
  await client.refresh();
  assert.equal(client.getSyncState(), null);
  assert.equal(client.clientRunning(), false);
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
  const sent = await client.sendMessage({ roomId: '!r:example.org', body: 'hello' });
  assert.equal(sent?.eventId, '$e1');
  assert.deepEqual(callLog, ['matrix_send_text']);
});

test('F3 sendMessage fails closed when the command is unavailable', async () => {
  const client = createNativeMatrixClient(async () => unavailable);
  assert.equal(await client.sendMessage({ roomId: '!r:example.org', body: 'x' }), null);
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

test('F4 uploadContent fails closed when unavailable', async () => {
  const client = createNativeMatrixClient(async () => unavailable);
  assert.equal(await client.uploadContent({ mimeType: 'x', bytes: [1] }), null);
});

test('F4 getMediaConfig reads m.upload.size wire key', async () => {
  const { invoke } = invokingWith({
    matrix_call_media_config: { 'm.upload.size': 10485760 },
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

test('F4 getProfileInfo returns session identity', async () => {
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
  const profile = client.getProfileInfo();
  assert.equal(profile.userId, '@alice:example.org');
  assert.equal(profile.deviceId, 'DEV');
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

test('F5 matrixRTC is a GAP-safe stub (V-CALL is matrix-widget-api)', async () => {
  const client = createNativeMatrixClient(async () => unavailable);
  assert.equal(client.matrixRTC.getRoomSession({ roomId: '!r:example.org' }), null);
  assert.doesNotThrow(() => client.matrixRTC.on('session_started', () => undefined));
});

test('F5 extended GAP stubs satisfy the anchor without data', async () => {
  const client = createNativeMatrixClient(async () => unavailable);
  assert.deepEqual(await client.getCapabilities(), {});
  assert.equal(await client.getOpenIdToken(), null);
  assert.equal(await client.search({ term: 'x' }), null);
});
