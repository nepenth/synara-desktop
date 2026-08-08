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
  const state = await client.getSyncState();
  assert.equal(state, 'PREPARED');
  assert.equal(await client.clientRunning(), true);
  assert.deepEqual(await client.getSyncStateData(), {
    readiness: 'running',
    sessionGeneration: 7,
    failureDiagnosticId: null,
  });
});

test('getSyncState fails closed when the native command is unavailable', async () => {
  const { invoke } = invokingWith({});
  const client = createNativeMatrixClient(invoke);
  assert.equal(await client.getSyncState(), null);
  assert.equal(await client.clientRunning(), false);
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
  await client.getSyncState();
  assert.deepEqual(seen, ['ERROR']);
  client.removeListener('sync', listener);
  await client.getSyncState();
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
  assert.equal(await client.getUserId(), '@alice:example.org');
  assert.equal(await client.getSafeUserId(), '@alice:example.org');
  assert.equal(await client.getDeviceId(), 'DEVICE');
});

test('identity is empty when the session is logged out', async () => {
  const { invoke } = invokingWith({ matrix_session_snapshot: { status: 'logged_out' } });
  const client = createNativeMatrixClient(invoke);
  assert.equal(await client.getUserId(), undefined);
  assert.equal(await client.getSafeUserId(), '');
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
