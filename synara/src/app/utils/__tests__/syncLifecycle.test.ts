import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  hiddenDurationMs,
  shouldRecoverSyncOnWake,
  shouldRetrySyncOnResume,
  SYNC_WAKE_HIDDEN_MS,
} from '../syncLifecycle';

test('sync resume retry is limited to backed-off connection states', () => {
  assert.equal(shouldRetrySyncOnResume('RECONNECTING'), true);
  assert.equal(shouldRetrySyncOnResume('ERROR'), true);

  assert.equal(shouldRetrySyncOnResume(null), false);
  assert.equal(shouldRetrySyncOnResume('PREPARED'), false);
  assert.equal(shouldRetrySyncOnResume('SYNCING'), false);
  assert.equal(shouldRetrySyncOnResume('CATCHUP'), false);
  assert.equal(shouldRetrySyncOnResume('STOPPED'), false);
});

test('wake recovery restarts a live SDK that still reports PREPARED after sleep', () => {
  assert.equal(
    shouldRecoverSyncOnWake({
      reason: 'online',
      syncState: 'PREPARED',
      hiddenDurationMs: 0,
    }),
    true
  );
  assert.equal(
    shouldRecoverSyncOnWake({
      reason: 'pageshow',
      syncState: 'PREPARED',
      hiddenDurationMs: 0,
      persisted: true,
    }),
    true
  );
  assert.equal(
    shouldRecoverSyncOnWake({
      reason: 'visibilitychange',
      syncState: 'PREPARED',
      hiddenDurationMs: SYNC_WAKE_HIDDEN_MS,
    }),
    true
  );
  assert.equal(
    shouldRecoverSyncOnWake({
      reason: 'focus',
      syncState: 'PREPARED',
      hiddenDurationMs: SYNC_WAKE_HIDDEN_MS,
    }),
    true
  );
});

test('wake recovery does not restart a healthy live session on brief focus', () => {
  assert.equal(
    shouldRecoverSyncOnWake({
      reason: 'focus',
      syncState: 'PREPARED',
      hiddenDurationMs: 1_000,
    }),
    false
  );
  assert.equal(
    shouldRecoverSyncOnWake({
      reason: 'visibilitychange',
      syncState: 'PREPARED',
      hiddenDurationMs: 0,
    }),
    false
  );
  assert.equal(
    shouldRecoverSyncOnWake({
      reason: 'pageshow',
      syncState: 'PREPARED',
      hiddenDurationMs: 0,
      persisted: false,
    }),
    false
  );
});

test('wake recovery still retries backed-off states and ignores stopped clients', () => {
  assert.equal(
    shouldRecoverSyncOnWake({
      reason: 'focus',
      syncState: 'ERROR',
      hiddenDurationMs: 0,
    }),
    true
  );
  assert.equal(
    shouldRecoverSyncOnWake({
      reason: 'online',
      syncState: 'STOPPED',
      hiddenDurationMs: 0,
    }),
    false
  );
  assert.equal(
    shouldRecoverSyncOnWake({
      reason: 'online',
      syncState: null,
      hiddenDurationMs: 60_000,
    }),
    false
  );
});

test('hidden duration is zero until the window has been hidden', () => {
  assert.equal(hiddenDurationMs(null, 50_000), 0);
  assert.equal(hiddenDurationMs(10_000, 25_000), 15_000);
});

test('desktop wake path restarts native sync instead of only rereading status', () => {
  const clientRoot = readFileSync('src/app/pages/client/ClientRoot.tsx', 'utf8');
  const facade = readFileSync('src/app/features/native-client/nativeClientFacade.ts', 'utf8');
  const product = readFileSync('../src-tauri/src/matrix/auth/product.rs', 'utf8');
  const lib = readFileSync('../src-tauri/src/lib.rs', 'utf8');

  assert.match(clientRoot, /shouldRecoverSyncOnWake/);
  assert.match(clientRoot, /scheduleRetry\('online'\)/);
  assert.match(facade, /matrix_sync_recover/);
  assert.match(facade, /await invoke\('matrix_sync_recover'\)/);
  assert.match(product, /spawn_suspend_resume_watch/);
  assert.match(product, /suspend_detected/);
  assert.match(lib, /spawn_suspend_resume_watch/);
});
