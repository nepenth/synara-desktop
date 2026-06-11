import assert from 'node:assert/strict';
import test from 'node:test';
import { SyncState } from 'matrix-js-sdk';
import { getSyncStatusBannerCopy } from '../syncStatusCopy';

test('sync status copy distinguishes catchup from prepared', () => {
  const catchupCopy = getSyncStatusBannerCopy(SyncState.Catchup);
  const preparedCopy = getSyncStatusBannerCopy(SyncState.Prepared);

  assert.match(catchupCopy ?? '', /history|syncing/i);
  assert.notEqual(catchupCopy, preparedCopy);
  assert.equal(preparedCopy, 'Connected');
});