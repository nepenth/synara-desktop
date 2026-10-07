import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
// SyncState literals are the probed js-sdk enum values.
import {
  CONNECTED_STATUS_BANNER_DURATION_MS,
  RECONNECTING_BANNER_HOLD_MS,
  getSlidingSyncCapabilityBannerCopy,
  getSyncStatusBannerCopy,
  getTransientSyncStatusBannerCopy,
  isSignedInSessionForBanner,
  shouldShowConnectedTransition,
} from '../syncStatusCopy';

test('sync status copy distinguishes catchup from prepared', () => {
  const catchupCopy = getSyncStatusBannerCopy('CATCHUP');
  const preparedCopy = getSyncStatusBannerCopy('PREPARED');

  assert.match(catchupCopy ?? '', /history|syncing/i);
  assert.notEqual(catchupCopy, preparedCopy);
  assert.equal(preparedCopy, 'Connected');
});

test('sliding-sync capability copy warns about homeserver support', () => {
  assert.match(getSlidingSyncCapabilityBannerCopy(), /sliding-sync|MSC4186/i);
  assert.match(getSlidingSyncCapabilityBannerCopy(), /sync/i);
});

test('connected is transient while steady prepared sync is bannerless', () => {
  assert.equal(getTransientSyncStatusBannerCopy('PREPARED', true), 'Connected');
  assert.equal(getTransientSyncStatusBannerCopy('PREPARED', false), null);
  assert.equal(
    getTransientSyncStatusBannerCopy('RECONNECTING', false, true),
    'Connection Lost! Reconnecting...'
  );
  assert.equal(getTransientSyncStatusBannerCopy('RECONNECTING', false, false), null);
  assert.equal(getTransientSyncStatusBannerCopy('ERROR', false, false), 'Connection Lost!');
  assert.equal(getTransientSyncStatusBannerCopy('STOPPED', false, false, true), 'Connection Lost!');
  assert.equal(getTransientSyncStatusBannerCopy('STOPPED', false, false, false), null);
  assert.ok(CONNECTED_STATUS_BANNER_DURATION_MS > 0);
  assert.ok(RECONNECTING_BANNER_HOLD_MS >= 4_000);
});

test('connected flash only follows a Lost banner the user actually saw', () => {
  assert.equal(shouldShowConnectedTransition('PREPARED', false), false);
  assert.equal(shouldShowConnectedTransition('PREPARED', true), true);
  assert.equal(shouldShowConnectedTransition('RECONNECTING', true), false);
  assert.equal(shouldShowConnectedTransition('ERROR', true), false);
});

test('STOPPED is Connection Lost only for a signed-in session that was connected', () => {
  const signedIn = { hasSignedInSession: () => true };
  const signedOut = { hasSignedInSession: () => false };
  assert.equal(isSignedInSessionForBanner(signedIn, true), true);
  // Cold start before the first PREPARED stays blank.
  assert.equal(isSignedInSessionForBanner(signedIn, false), false);
  // Logout, stopClient, or a logged-out snapshot.
  assert.equal(isSignedInSessionForBanner(signedOut, true), false);
  // A client without the probe (non-native) never claims a signed-in session.
  assert.equal(isSignedInSessionForBanner({}, true), false);
  assert.equal(isSignedInSessionForBanner(null, true), false);

  assert.equal(
    getTransientSyncStatusBannerCopy(
      'STOPPED',
      false,
      false,
      isSignedInSessionForBanner(signedIn, true)
    ),
    'Connection Lost!'
  );
  assert.equal(
    getTransientSyncStatusBannerCopy(
      'STOPPED',
      false,
      false,
      isSignedInSessionForBanner(signedOut, true)
    ),
    null
  );
});

test('SyncStatus derives the signed-in flag from the client, not a constant', () => {
  const source = readFileSync('src/app/pages/client/SyncStatus.tsx', 'utf8');
  assert.match(source, /isSignedInSessionForBanner\(mx, connectedDuringMount\)/);
  assert.doesNotMatch(source, /signedInSession = true/);
});
