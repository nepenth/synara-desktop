import assert from 'node:assert/strict';
import test from 'node:test';
// SyncState literals are the probed js-sdk enum values.
import { getSyncStatusBannerCopy } from '../syncStatusCopy';

test('sync status copy distinguishes catchup from prepared', () => {
  const catchupCopy = getSyncStatusBannerCopy('CATCHUP');
  const preparedCopy = getSyncStatusBannerCopy('PREPARED');

  assert.match(catchupCopy ?? '', /history|syncing/i);
  assert.notEqual(catchupCopy, preparedCopy);
  assert.equal(preparedCopy, 'Connected');
});
