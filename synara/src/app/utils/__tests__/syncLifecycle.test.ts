import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  consumeHiddenDurationMs,
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

test('reopening a live session from the tray does not restart sync', () => {
  assert.ok(SYNC_WAKE_HIDDEN_MS >= 5 * 60_000);
  for (const reason of ['visibilitychange', 'focus'] as const) {
    assert.equal(
      shouldRecoverSyncOnWake({
        reason,
        syncState: 'PREPARED',
        hiddenDurationMs: 60_000,
      }),
      false
    );
    assert.equal(
      shouldRecoverSyncOnWake({
        reason,
        syncState: 'ERROR',
        hiddenDurationMs: 60_000,
      }),
      true
    );
  }
});

test('hidden duration is zero until the window has been hidden', () => {
  assert.equal(hiddenDurationMs(null, 50_000), 0);
  assert.equal(hiddenDurationMs(10_000, 25_000), 15_000);
});

test('a short hide is consumed so a later focus cannot reuse a stale duration', () => {
  const firstVisible = consumeHiddenDurationMs(10_000, 20_000);
  assert.equal(firstVisible.hiddenDurationMs, 10_000);
  assert.equal(firstVisible.hiddenAtMs, null);
  assert.equal(
    shouldRecoverSyncOnWake({
      reason: 'visibilitychange',
      syncState: 'PREPARED',
      hiddenDurationMs: firstVisible.hiddenDurationMs,
    }),
    false
  );

  const laterFocus = consumeHiddenDurationMs(firstVisible.hiddenAtMs, 40_000);
  assert.equal(laterFocus.hiddenDurationMs, 0);
  assert.equal(
    shouldRecoverSyncOnWake({
      reason: 'focus',
      syncState: 'PREPARED',
      hiddenDurationMs: laterFocus.hiddenDurationMs,
    }),
    false
  );

  const longHide = consumeHiddenDurationMs(10_000, 10_000 + SYNC_WAKE_HIDDEN_MS);
  assert.equal(longHide.hiddenDurationMs, SYNC_WAKE_HIDDEN_MS);
  assert.equal(
    shouldRecoverSyncOnWake({
      reason: 'visibilitychange',
      syncState: 'PREPARED',
      hiddenDurationMs: longHide.hiddenDurationMs,
    }),
    true
  );
});

test('desktop wake path restarts native sync instead of only rereading status', () => {
  const clientRoot = readFileSync('src/app/pages/client/ClientRoot.tsx', 'utf8');
  const session = readFileSync('src/app/native/nativeSession.ts', 'utf8');
  const product = readFileSync('../src-tauri/src/matrix/auth/product.rs', 'utf8');
  const lib = readFileSync('../src-tauri/src/lib.rs', 'utf8');
  // The recover gate and its wall-clock cooldown live in the shared Core owner.
  const recoverGate = readFileSync(
    '../crates/synara-core/src/app/lifecycle/session/recover.rs',
    'utf8'
  );

  assert.match(clientRoot, /shouldRecoverSyncOnWake/);
  assert.match(clientRoot, /consumeHiddenDurationMs/);
  assert.match(clientRoot, /scheduleRetry\('online'\)/);
  assert.match(session, /matrix_sync_recover/);
  assert.match(session, /await invoke\('matrix_sync_recover'\)/);
  assert.match(product, /spawn_suspend_resume_watch/);
  assert.match(product, /suspend_detected/);
  assert.match(product, /self\.recover_live\(/);
  assert.match(recoverGate, /recover_cooldown_active/);
  assert.match(recoverGate, /last_success_wall/);
  assert.match(product, /recover_sync_after_detected_suspend/);
  assert.doesNotMatch(product, /last\.elapsed\(\)/);
  assert.doesNotMatch(recoverGate, /last\.elapsed\(\)/);
  assert.match(lib, /spawn_suspend_resume_watch/);
});
