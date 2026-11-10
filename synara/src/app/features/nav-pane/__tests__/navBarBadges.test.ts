import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  accountBarState,
  activityBarBadge,
  activityLandingPage,
  approvalsBarState,
  barShowsLabels,
  homeBarBadge,
} from '../navBarBadges';

// The rail's badge states, carried to the bottom bar (N2). Each case is one
// the former HomeTab, InboxTab, ApprovalsTab or UnverifiedTab drew.

test('Home: a dot for plain unread, a red count for mentions and unread DMs', () => {
  assert.deepEqual(homeBarBadge(false, 0), { kind: 'none' });
  // Unread with no attention never shows digits.
  assert.deepEqual(homeBarBadge(true, 0), { kind: 'dot' });
  assert.deepEqual(homeBarBadge(true, 3), { kind: 'count', count: 3, highlight: true });
  // Attention without an unread scope (cannot happen in Core) stays quiet.
  assert.deepEqual(homeBarBadge(false, 2), { kind: 'none' });
});

test('Activity: invites plus active Later items, red for an invite or an overdue item', () => {
  assert.deepEqual(activityBarBadge(0, { active: 0, overdue: 0 }), { kind: 'none' });
  assert.deepEqual(activityBarBadge(2, { active: 0, overdue: 0 }), {
    kind: 'count',
    count: 2,
    highlight: true,
  });
  assert.deepEqual(activityBarBadge(0, { active: 3, overdue: 0 }), {
    kind: 'count',
    count: 3,
    highlight: false,
  });
  assert.deepEqual(activityBarBadge(0, { active: 3, overdue: 1 }), {
    kind: 'count',
    count: 3,
    highlight: true,
  });
  assert.deepEqual(activityBarBadge(1, { active: 2, overdue: 0 }), {
    kind: 'count',
    count: 3,
    highlight: true,
  });
});

test('Activity opens invites first, then Later, then notifications', () => {
  assert.equal(activityLandingPage(1, 4), 'invites');
  assert.equal(activityLandingPage(0, 4), 'later');
  assert.equal(activityLandingPage(0, 0), 'notifications');
});

test('Approvals: count, coverage and the partial or unavailable warning', () => {
  const base = { pendingCount: 0, loading: false, incomplete: false };
  assert.deepEqual(approvalsBarState(base), {
    label: 'Approvals',
    coverageDescription: '',
    badge: { kind: 'none' },
  });
  assert.deepEqual(approvalsBarState({ ...base, pendingCount: 1 }), {
    label: 'Approvals · 1 pending',
    coverageDescription: '',
    badge: { kind: 'count', count: 1, highlight: true },
  });
  assert.equal(approvalsBarState({ ...base, loading: true }).label, 'Approvals · checking rooms');
  assert.equal(
    approvalsBarState({ ...base, coverage: 'latest_event' }).label,
    'Approvals · recent activity'
  );
  // Coverage alone is no reason for a badge.
  assert.deepEqual(approvalsBarState({ ...base, coverage: 'latest_event' }).badge, {
    kind: 'none',
  });
  assert.deepEqual(
    approvalsBarState({ ...base, pendingCount: 2, coverage: 'latest_event' }).badge,
    { kind: 'count', count: 2, highlight: true }
  );
  const partial = approvalsBarState({ ...base, pendingCount: 2, incomplete: true });
  assert.equal(partial.label, 'Approvals · 2+ pending · partial coverage');
  assert.deepEqual(partial.badge, { kind: 'status', text: '2+', tone: 'Success' });
  const partialEmpty = approvalsBarState({ ...base, incomplete: true });
  assert.equal(partialEmpty.label, 'Approvals · partial coverage');
  assert.deepEqual(partialEmpty.badge, { kind: 'status', text: '!', tone: 'Warning' });
  const failed = approvalsBarState({ ...base, error: new Error('x') });
  assert.equal(failed.label, 'Approvals · unavailable');
  assert.equal(failed.coverageDescription, 'unavailable');
  assert.deepEqual(failed.badge, { kind: 'status', text: '!', tone: 'Warning' });
});

test('Account: the unverified warning moves onto the avatar', () => {
  // No cross-signing to offer: no warning, whatever the devices say.
  assert.deepEqual(accountBarState(false, true, 3), { label: 'Account', badge: { kind: 'none' } });
  assert.deepEqual(accountBarState(true, false, 0), { label: 'Account', badge: { kind: 'none' } });
  // This device unverified: Critical, and the menu offers "Verify this device".
  assert.deepEqual(accountBarState(true, true, 2), {
    label: 'Account, this device is not verified',
    badge: { kind: 'status', text: '!', tone: 'Critical' },
    verify: 'this-device',
  });
  // Other devices unverified: a Warning count.
  assert.deepEqual(accountBarState(true, false, 1), {
    label: 'Account, 1 unverified device',
    badge: { kind: 'status', text: '1', tone: 'Warning' },
    verify: 'other-devices',
  });
  assert.equal(accountBarState(true, false, 4).label, 'Account, 4 unverified devices');
});

test('bar labels appear once the column is wider than 300px, and on Mobile', () => {
  assert.equal(barShowsLabels(196), false);
  assert.equal(barShowsLabels(300), false);
  assert.equal(barShowsLabels(301), true);
  assert.equal(barShowsLabels(undefined), true);
});

test('the bar is the list column footer on every client pane', () => {
  for (const path of [
    'pages/client/home/Home.tsx',
    'pages/client/space/Space.tsx',
    'pages/client/explore/Explore.tsx',
    'pages/client/inbox/Inbox.tsx',
  ]) {
    const source = readFileSync(`src/app/${path}`, 'utf8');
    assert.match(source, /footer=\{<NavBottomBar \/>\}/, path);
    assert.match(source, /header=\{\s*<(NavPaneHeader|HomeHeader|SpaceHeader)/, path);
  }
  const bar = readFileSync('src/app/features/nav-pane/NavBottomBar.tsx', 'utf8');
  const order = ['<HomeBarItem', '<ActivityBarItem', '<ApprovalsBarItem', '<AccountBarItem'];
  const at = order.map((tag) => bar.lastIndexOf(tag));
  assert.ok(
    at.every((index, i) => index >= 0 && (i === 0 || index > at[i - 1])),
    String(at)
  );
  // Home keeps its right-click "Mark as read"; failures show in the menu.
  assert.match(bar, /useMarkRoomsAsRead\(\)/);
  assert.match(bar, /\{markReadError\}/);
  assert.match(bar, /onContextMenu=/);
  // The rail's verification gate: the device list is read only when offered.
  assert.match(bar, /\{canOffer && <DeviceVerificationProbe/);
});
