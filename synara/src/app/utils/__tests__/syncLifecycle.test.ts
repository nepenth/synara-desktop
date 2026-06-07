import assert from 'node:assert/strict';
import test from 'node:test';
import { SyncState } from 'matrix-js-sdk';
import { shouldRetrySyncOnResume } from '../syncLifecycle';

test('sync resume retry is limited to backed-off connection states', () => {
  assert.equal(shouldRetrySyncOnResume(SyncState.Reconnecting), true);
  assert.equal(shouldRetrySyncOnResume(SyncState.Error), true);

  assert.equal(shouldRetrySyncOnResume(null), false);
  assert.equal(shouldRetrySyncOnResume(SyncState.Prepared), false);
  assert.equal(shouldRetrySyncOnResume(SyncState.Syncing), false);
  assert.equal(shouldRetrySyncOnResume(SyncState.Catchup), false);
  assert.equal(shouldRetrySyncOnResume(SyncState.Stopped), false);
});
