import assert from 'node:assert/strict';
import test from 'node:test';
import {
  clearActiveNativeMatrixSession,
  getActiveNativeMatrixSession,
  hasActiveNativeMatrixSession,
  restoreNativeMatrixSessionWith,
  setActiveNativeMatrixSession,
} from '../nativeMatrixSession';

const identity = {
  userId: '@alice:example.org',
  deviceId: 'DEVICE',
  homeserverUrl: 'https://example.org',
};

test.afterEach(() => clearActiveNativeMatrixSession());

test('native session identity is token-free authenticated routing state', () => {
  setActiveNativeMatrixSession(identity);

  assert.equal(hasActiveNativeMatrixSession(), true);
  assert.deepEqual(getActiveNativeMatrixSession(), identity);
  assert.equal('accessToken' in getActiveNativeMatrixSession()!, false);
});

test('desktop startup restores the Rust session without a JS client session', async () => {
  const commands: string[] = [];
  const restored = await restoreNativeMatrixSessionWith(true, async (command) => {
    commands.push(command);
    return { available: true, value: identity };
  });

  assert.deepEqual(commands, ['matrix_restore_session']);
  assert.deepEqual(restored, identity);
  assert.equal(hasActiveNativeMatrixSession(), true);
});

test('browser startup does not invoke native restore', async () => {
  const restored = await restoreNativeMatrixSessionWith(false, async () => {
    throw new Error('native invoke should not run');
  });

  assert.equal(restored, undefined);
  assert.equal(hasActiveNativeMatrixSession(), false);
});

test('native restore fails closed and clears stale routing state', async () => {
  setActiveNativeMatrixSession(identity);
  const restored = await restoreNativeMatrixSessionWith(true, async () => {
    throw new Error('restore failed');
  });

  assert.equal(restored, undefined);
  assert.equal(hasActiveNativeMatrixSession(), false);
});
