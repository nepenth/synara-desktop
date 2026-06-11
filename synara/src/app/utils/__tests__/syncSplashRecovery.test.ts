import assert from 'node:assert/strict';
import test from 'node:test';
import { SyncState } from 'matrix-js-sdk';
import {
  formatSyncStateTransition,
  shouldShowSyncRecoveryUI,
  SYNC_PREPARED_TIMEOUT_MS,
} from '../syncSplashRecovery';

test('sync splash recovery timeout is ninety seconds', () => {
  assert.equal(SYNC_PREPARED_TIMEOUT_MS, 90_000);
});

test('sync splash recovery UI appears only while loading after timeout', () => {
  assert.equal(shouldShowSyncRecoveryUI(true, true), true);
  assert.equal(shouldShowSyncRecoveryUI(true, false), false);
  assert.equal(shouldShowSyncRecoveryUI(false, true), false);
});

test('sync state transitions format previous and current values', () => {
  assert.equal(
    formatSyncStateTransition(SyncState.Prepared, SyncState.Catchup),
    `sync ${SyncState.Catchup} -> ${SyncState.Prepared}`
  );
});