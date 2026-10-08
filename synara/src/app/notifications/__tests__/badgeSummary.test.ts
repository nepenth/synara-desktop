import { readFileSync } from 'node:fs';
import test from 'node:test';
import assert from 'node:assert/strict';
import { getBadgeCount, summarizeNotifications } from '../badgeSummary';

// The per-room rule (mentions count instead of totals, space rollups never
// count) lives in Core: `room_list::presentation::summarize_notifications`.

test('summarizeNotifications adds Core room totals to renderer-owned counts', () => {
  assert.deepEqual(
    summarizeNotifications({
      highlightTotal: 2,
      unreadTotal: 3,
      laterActiveCount: 5,
      inviteCount: 2,
      agentApprovalCount: 1,
    }),
    {
      appBadgeCount: 10,
      inboxBadgeCount: 8,
      laterActiveCount: 5,
      inviteCount: 2,
      agentApprovalCount: 1,
      highlightCount: 2,
      unreadCount: 3,
    }
  );
});

test('summarizeNotifications clamps negative, fractional and missing counts', () => {
  assert.deepEqual(
    summarizeNotifications({
      highlightTotal: -2,
      unreadTotal: 3.9,
      laterActiveCount: 2.8,
      inviteCount: -1,
      agentApprovalCount: Number.NaN,
    }),
    {
      appBadgeCount: 5,
      inboxBadgeCount: 2,
      laterActiveCount: 2,
      inviteCount: 0,
      agentApprovalCount: 0,
      highlightCount: 0,
      unreadCount: 3,
    }
  );
  assert.equal(summarizeNotifications({}).appBadgeCount, 0);
});

test('getBadgeCount returns the app-icon number for platform adapters', () => {
  assert.equal(getBadgeCount({ highlightTotal: 4, unreadTotal: 1, laterActiveCount: 2 }), 7);
});

test('the badge reads Core totals, not the unread map that holds space rollups', () => {
  const updater = readFileSync('src/app/pages/client/ClientNonUIFeatures.tsx', 'utf8');
  const block = updater.slice(updater.indexOf('function PlatformBadgeAndTrayUpdater'));
  assert.match(block, /presentation\.highlightTotal/);
  assert.match(block, /presentation\.unreadTotal/);
  assert.doesNotMatch(block.slice(0, block.indexOf('return null')), /roomToUnread/);
});

test('desktop tray shares the approval center snapshot instead of latest-message heuristics', () => {
  const source = readFileSync('src/app/pages/client/ClientNonUIFeatures.tsx', 'utf8');
  assert.match(source, /pendingCount: agentApprovalCount.*useApprovalInboxSummary/);
  assert.doesNotMatch(source, /countJoinedRoomAgentApprovals|lastMessageIsAgentApproval/);
});

test('dock badge stays on summarizeNotifications and never uses SDK total_unread_notifications', () => {
  const updater = readFileSync('src/app/pages/client/ClientNonUIFeatures.tsx', 'utf8');
  const badge = readFileSync('src/app/platform/badge.ts', 'utf8');
  const summary = readFileSync('src/app/notifications/badgeSummary.ts', 'utf8');
  assert.match(updater, /getPlatformNotificationSummary/);
  assert.match(updater, /setPlatformBadgeCount\(summary\.appBadgeCount\)/);
  assert.match(badge, /summarizeNotifications/);
  assert.match(badge, /setDesktopBadgeCount/);
  assert.doesNotMatch(updater, /total_unread_notifications/);
  assert.doesNotMatch(badge, /total_unread_notifications/);
  assert.doesNotMatch(summary, /total_unread_notifications/);
});

test('notification pings prefer native playback over HTML audio', () => {
  const updater = readFileSync('src/app/pages/client/ClientNonUIFeatures.tsx', 'utf8');
  const desktop = readFileSync('src/app/utils/desktop.ts', 'utf8');
  const notifications = readFileSync('src/app/platform/notifications.ts', 'utf8');
  assert.match(updater, /playPlatformNotificationSound\('invite'\)/);
  assert.match(updater, /playPlatformNotificationSound\('message'\)/);
  assert.match(desktop, /desktop_play_notification_sound/);
  assert.match(notifications, /playDesktopNotificationSound/);
});

test('logged-in desktop shell always mounts PlatformBadgeAndTrayUpdater', () => {
  const updater = readFileSync('src/app/pages/client/ClientNonUIFeatures.tsx', 'utf8');
  const router = readFileSync('src/app/pages/Router.tsx', 'utf8');
  const exportStart = updater.indexOf('export function ClientNonUIFeatures');
  assert.ok(exportStart >= 0);
  const exported = updater.slice(exportStart);
  assert.match(exported, /<PlatformBadgeAndTrayUpdater \/>/);
  assert.doesNotMatch(
    exported,
    /supportsAppBadge|supportsPlatformTrayState\(\)[\s\S]*PlatformBadgeAndTrayUpdater/
  );
  assert.match(router, /<ClientNonUIFeatures>/);
  assert.doesNotMatch(router, /supportsAppBadge[\s\S]*ClientNonUIFeatures/);
});
