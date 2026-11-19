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

test('Settings avatar: the unverified warning sits on it and decides where it opens', () => {
  // No cross-signing to offer: no warning, whatever the devices say.
  assert.deepEqual(accountBarState(false, true, 3), { label: 'Settings', badge: { kind: 'none' } });
  assert.deepEqual(accountBarState(true, false, 0), { label: 'Settings', badge: { kind: 'none' } });
  // This session unverified: Critical, and the avatar opens Security & recovery.
  assert.deepEqual(accountBarState(true, true, 2), {
    label: 'Settings, this session is not verified',
    badge: { kind: 'status', text: '!', tone: 'Critical' },
    verify: 'this-device',
  });
  // Other sessions unverified: a Warning count, and the avatar opens Sessions.
  assert.deepEqual(accountBarState(true, false, 1), {
    label: 'Settings, 1 unverified session',
    badge: { kind: 'status', text: '1', tone: 'Warning' },
    verify: 'other-devices',
  });
  assert.equal(accountBarState(true, false, 4).label, 'Settings, 4 unverified sessions');
});

test('the avatar toggles Settings with no menu, and Log out lives in Settings', () => {
  const bar = readFileSync('src/app/features/nav-pane/NavBottomBar.tsx', 'utf8');
  assert.doesNotMatch(bar, /AccountMenu/);
  assert.doesNotMatch(bar, /LogoutDialog/);
  assert.match(bar, /useToggleAppSettings\(\)/);
  assert.match(bar, /aria-expanded=\{settingsOpen\}/);
  assert.match(bar, /verificationSettingsLocation\(verify\)/);
  assert.match(bar, /returnFocus: \(\) => triggerRef\.current\?\.focus\(\)/);
  const settings = readFileSync('src/app/features/settings/Settings.tsx', 'utf8');
  assert.match(settings, /<Text size="B400">Log out<\/Text>/);
  // The shortcut toggles the same way.
  const shortcuts = readFileSync(
    'src/app/features/desktop-keyboard/DesktopAppShortcuts.tsx',
    'utf8'
  );
  assert.match(shortcuts, /toggleSettings\(\)/);
});

test('a profile view that mounts fresh starts from the last loaded profile', () => {
  // The Settings header and Account › Profile mount each time Settings opens;
  // they must not fall back to the initial while the profile is fetched.
  const hook = readFileSync('src/app/hooks/useUserProfile.ts', 'utf8');
  assert.match(hook, /useState<UserProfile>\(\(\) => knownProfiles\.get\(userId\) \?\? \{\}\)/);
  assert.match(hook, /knownProfiles\.set\(userId, next\)/);
});

test('bar labels appear once the column is wider than 300px, and on Mobile', () => {
  assert.equal(barShowsLabels(196), false);
  assert.equal(barShowsLabels(300), false);
  assert.equal(barShowsLabels(301), true);
  assert.equal(barShowsLabels(undefined), true);
});

test('the bar is mounted once by the client shell and placed in every pane column', () => {
  for (const path of [
    'pages/client/home/Home.tsx',
    'pages/client/space/Space.tsx',
    'pages/client/explore/Explore.tsx',
    'pages/client/inbox/Inbox.tsx',
  ]) {
    const source = readFileSync(`src/app/${path}`, 'utf8');
    assert.match(source, /footer=\{<NavBottomBarSlot \/>\}/, path);
    assert.doesNotMatch(source, /<NavBottomBar \/>/, path);
    assert.match(source, /header=\{\s*<(NavPaneHeader|HomeHeader|SpaceHeader)/, path);
  }
  // One host for the signed-in shell, so navigation never remounts the bar.
  const router = readFileSync('src/app/pages/Router.tsx', 'utf8');
  assert.equal(router.match(/<NavBottomBarHost \/>/g)?.length, 1);
  const slot = readFileSync('src/app/features/nav-pane/NavBottomBarSlot.tsx', 'utf8');
  assert.match(slot, /createPortal\(<NavBottomBar navWidth=/);
  assert.match(slot, /appendChild\(getBarElement\(\)\)/);
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
