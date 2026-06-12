import assert from 'node:assert/strict';
import test from 'node:test';
import { SyncState } from 'matrix-js-sdk';
import {
  formatSyncSplashStatus,
  formatSyncStateTransition,
  selectSyncSplashView,
  shouldShowSyncRecoveryUI,
  SYNC_PREPARED_TIMEOUT_MS,
} from '../syncSplashRecovery';

test('sync splash recovery timeout is thirty seconds', () => {
  assert.equal(SYNC_PREPARED_TIMEOUT_MS, 30_000);
});

test('sync splash recovery UI appears only while loading after timeout', () => {
  assert.equal(shouldShowSyncRecoveryUI(true, true), true);
  assert.equal(shouldShowSyncRecoveryUI(true, false), false);
  assert.equal(shouldShowSyncRecoveryUI(false, true), false);
});

test('sync splash view renders only one startup surface at a time', () => {
  assert.equal(
    selectSyncSplashView({
      hasError: true,
      hasClient: true,
      loading: true,
      syncTimedOut: true,
    }),
    'error'
  );
  assert.equal(
    selectSyncSplashView({
      hasError: false,
      hasClient: true,
      loading: true,
      syncTimedOut: true,
    }),
    'recovery'
  );
  assert.equal(
    selectSyncSplashView({
      hasError: false,
      hasClient: true,
      loading: true,
      syncTimedOut: false,
    }),
    'loading'
  );
  assert.equal(
    selectSyncSplashView({
      hasError: false,
      hasClient: false,
      loading: false,
      syncTimedOut: false,
    }),
    'loading'
  );
  assert.equal(
    selectSyncSplashView({
      hasError: false,
      hasClient: true,
      loading: false,
      syncTimedOut: true,
    }),
    'client'
  );
});

test('sync splash status names the current startup phase', () => {
  assert.equal(formatSyncSplashStatus(undefined, false), 'Restoring session');
  assert.equal(formatSyncSplashStatus(null, true), 'Starting Matrix sync');
  assert.equal(formatSyncSplashStatus(SyncState.Catchup, true), 'Catching up');
  assert.equal(formatSyncSplashStatus(SyncState.Syncing, true), 'Syncing messages');
  assert.equal(formatSyncSplashStatus(SyncState.Reconnecting, true), 'Reconnecting');
});

test('sync state transitions format previous and current values', () => {
  assert.equal(
    formatSyncStateTransition(SyncState.Prepared, SyncState.Catchup),
    `sync ${SyncState.Catchup} -> ${SyncState.Prepared}`
  );
});
