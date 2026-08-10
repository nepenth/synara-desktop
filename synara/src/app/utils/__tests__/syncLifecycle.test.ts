import assert from 'node:assert/strict';
import test from 'node:test';
// SyncState literals are the probed js-sdk enum values.
import { shouldRetrySyncOnResume } from '../syncLifecycle';

test('sync resume retry is limited to backed-off connection states', () => {
  assert.equal(shouldRetrySyncOnResume('RECONNECTING'), true);
  assert.equal(shouldRetrySyncOnResume('ERROR'), true);

  assert.equal(shouldRetrySyncOnResume(null), false);
  assert.equal(shouldRetrySyncOnResume('PREPARED'), false);
  assert.equal(shouldRetrySyncOnResume('SYNCING'), false);
  assert.equal(shouldRetrySyncOnResume('CATCHUP'), false);
  assert.equal(shouldRetrySyncOnResume('STOPPED'), false);
});
