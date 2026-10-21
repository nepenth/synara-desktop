import { readFileSync } from 'node:fs';
import test from 'node:test';
import assert from 'node:assert/strict';
import {
  createApprovalAlertWithdrawal,
  createInviteArrivalTracker,
  createRoomReadTracker,
  inviteNotificationText,
  isDocumentActive,
  shouldPlayNotificationSound,
  shouldPresentOrdinaryMessage,
} from '../notificationDelivery';

const features = () => readFileSync('src/app/pages/client/ClientNonUIFeatures.tsx', 'utf8');
const section = (source: string, start: string) => {
  const from = source.indexOf(start);
  assert.ok(from >= 0, start);
  const to = source.indexOf('\nfunction ', from + 1);
  return source.slice(from, to < 0 ? undefined : to);
};

test('the Notifications list holds messages back only while the window is in front', () => {
  // Tray "Notifications" and the global shortcut open that list, then the
  // window is usually closed to the tray: messages must still notify.
  assert.equal(
    shouldPresentOrdinaryMessage({
      notificationListSelected: true,
      windowActive: false,
      isSpaceRoom: false,
    }),
    true
  );
  assert.equal(
    shouldPresentOrdinaryMessage({
      notificationListSelected: true,
      windowActive: true,
      isSpaceRoom: false,
    }),
    false
  );
  assert.equal(
    shouldPresentOrdinaryMessage({
      notificationListSelected: false,
      windowActive: true,
      isSpaceRoom: false,
    }),
    true
  );
  assert.equal(
    shouldPresentOrdinaryMessage({
      notificationListSelected: false,
      windowActive: false,
      isSpaceRoom: true,
    }),
    false
  );
});

test('a window counts as active only when focused and visible', () => {
  assert.equal(isDocumentActive({ hasFocus: () => true, visibilityState: 'visible' }), true);
  assert.equal(isDocumentActive({ hasFocus: () => false, visibilityState: 'visible' }), false);
  assert.equal(isDocumentActive({ hasFocus: () => true, visibilityState: 'hidden' }), false);
});

test('the message route decides suppression at delivery and keeps rooms missing from the snapshot', () => {
  const route = section(features(), 'function MessageNotifications()');
  assert.match(route, /presentOrdinaryMessages: shouldPresentOrdinaryMessage\(\{/);
  assert.match(route, /notificationListSelected: notificationSelectedRef\.current/);
  assert.match(route, /windowActive: isDocumentActive\(document\)/);
  assert.match(route, /isSpaceRoom: room\?\.isSpaceRoom\(\) \?\? false/);
  assert.doesNotMatch(route, /!notificationSelected && !!room/);
});

test('sounds play only with an accepted banner', () => {
  // Notifications off and tray Do Not Disturb leave the outcome undefined.
  assert.equal(shouldPlayNotificationSound({ soundsEnabled: true, outcome: undefined }), false);
  assert.equal(shouldPlayNotificationSound({ soundsEnabled: true, outcome: 'failed' }), false);
  assert.equal(shouldPlayNotificationSound({ soundsEnabled: true, outcome: 'delivered' }), true);
  assert.equal(shouldPlayNotificationSound({ soundsEnabled: false, outcome: 'delivered' }), false);
  assert.equal(
    shouldPlayNotificationSound({ soundsEnabled: true, requested: false, outcome: 'delivered' }),
    false
  );
});

test('the invite sound follows the invite banner instead of ignoring the notification switch', () => {
  const route = section(features(), 'function InviteNotifications()');
  assert.match(route, /if \(\s*!delivery\.showNotifications \|\|/);
  assert.match(
    route,
    /shouldPlayNotificationSound\(\{ soundsEnabled: delivery\.notificationSound, outcome \}\)/
  );
  assert.doesNotMatch(route, /if \(notificationSound\) \{\s*playSound\(\);/);
});

test('invite notifications open the Invites page and read naturally', () => {
  assert.deepEqual(inviteNotificationText(1), {
    title: 'New invitation',
    body: 'You have a new invitation.',
  });
  assert.deepEqual(inviteNotificationText(3), {
    title: 'New invitations',
    body: 'You have 3 new invitations.',
  });
  const route = section(features(), 'function InviteNotifications()');
  assert.match(route, /route: getInboxInvitesPath\(\)/);
  assert.doesNotMatch(route, /new invitation request/);
});

test('invites that exist at launch or arrive while sync is not running never notify', () => {
  const arrivals = createInviteArrivalTracker();
  assert.equal(arrivals({ sessionGeneration: 0, roomIds: [] }, true), 0);
  // The first loaded snapshot is the baseline, even with invites in it.
  assert.equal(arrivals({ sessionGeneration: 4, roomIds: ['!a', '!b'] }, true), 0);
  assert.equal(arrivals({ sessionGeneration: 4, roomIds: ['!a', '!b', '!c'] }, true), 1);
  // A declined invite and a new one in the same snapshot still count one.
  assert.equal(arrivals({ sessionGeneration: 4, roomIds: ['!b', '!c', '!d'] }, true), 1);
  assert.equal(arrivals({ sessionGeneration: 4, roomIds: ['!b', '!c', '!d', '!e'] }, false), 0);
  assert.equal(arrivals({ sessionGeneration: 4, roomIds: ['!b', '!c', '!d', '!e'] }, true), 0);
  // A new session starts a new baseline.
  assert.equal(arrivals({ sessionGeneration: 5, roomIds: ['!x', '!y'] }, true), 0);
  assert.equal(arrivals({ sessionGeneration: 5, roomIds: ['!x', '!y', '!z'] }, true), 1);
});

test('rooms read here, read elsewhere or left withdraw their banners once', () => {
  const read = createRoomReadTracker();
  const room = (roomId: string, unreadCount: number, highlightCount = 0, markedUnread = false) => ({
    roomId,
    unreadCount,
    highlightCount,
    markedUnread,
  });
  assert.deepEqual(read([room('!a', 2), room('!b', 0, 1), room('!c', 0)]), []);
  assert.deepEqual(read([room('!a', 0), room('!b', 0, 1), room('!c', 0)]), ['!a']);
  assert.deepEqual(read([room('!a', 0), room('!b', 0, 1), room('!c', 0)]), []);
  // Leaving the room also makes its banner stale.
  assert.deepEqual(read([room('!a', 0, 0, true)]), ['!b']);
  assert.deepEqual(read([]), ['!a']);

  const source = features();
  assert.match(section(source, 'function ReadRoomNotificationCleanup()'), /`room:\$\{roomId\}`/);
  assert.match(
    source.slice(source.indexOf('export function ClientNonUIFeatures')),
    /<ReadRoomNotificationCleanup \/>/
  );
});

test('approval alerts are withdrawn on expiry or a decision made elsewhere', () => {
  const withdrawn: string[] = [];
  const timers = new Map<number, () => void>();
  let nextTimer = 0;
  let now = 1_000;
  const alerts = createApprovalAlertWithdrawal({
    withdraw: (eventId) => withdrawn.push(eventId),
    now: () => now,
    schedule: (callback, delayMs) => {
      nextTimer += 1;
      assert.ok(delayMs >= 0);
      timers.set(nextTimer, callback);
      return nextTimer as unknown as ReturnType<typeof setTimeout>;
    },
    cancel: (handle) => {
      timers.delete(handle as unknown as number);
    },
  });

  alerts.track('$expiring', 301_000);
  alerts.track('$decided', 301_000);
  alerts.settle('$decided');
  assert.deepEqual(withdrawn, ['$decided']);
  // Settling twice, or an alert this window never delivered, does nothing.
  alerts.settle('$decided');
  alerts.settle('$unknown');
  assert.deepEqual(withdrawn, ['$decided']);

  now = 301_000;
  [...timers].forEach(([handle, fire]) => {
    timers.delete(handle);
    fire();
  });
  assert.deepEqual(withdrawn, ['$decided', '$expiring']);
  assert.equal(alerts.has('$expiring'), false);

  alerts.track('$late', 0);
  alerts.clear();
  assert.equal(timers.size, 0);

  const route = section(features(), 'function AgentApprovalNotifications()');
  assert.match(route, /approvalAlerts\.track\(eventId, agentApproval\.expiresAt\)/);
  assert.match(route, /if \(item\.status !== 'pending'\) approvalAlerts\.settle\(item\.eventId\)/);
  assert.match(route, /dismissDesktopNotifications\(\[`event:\$\{eventId\}`\]\)/);
});

test('notification listeners register once and read handlers from refs', () => {
  const source = features();
  const messages = section(source, 'function MessageNotifications()');
  assert.match(messages, /const decideAndNotifyRef = useLatestRef\(decideAndNotify\)/);
  assert.match(messages, /void decideAndNotifyRef\.current\(observation\)/);
  assert.match(messages, /\}, \[decideAndNotifyRef\]\);/);
  assert.doesNotMatch(messages, /\}, \[decideAndNotify\]\);/);

  const approvals = section(source, 'function AgentApprovalNotifications()');
  assert.match(approvals, /notifyApprovalEventRef\.current\(observation\)/);
  assert.match(approvals, /\}, \[notifyApprovalEventRef\]\);/);
  assert.match(approvals, /void handleNativeNotificationActionRef\.current\(payload\)/);
  assert.match(approvals, /\}, \[handleNativeNotificationActionRef\]\);/);
  assert.doesNotMatch(approvals, /\}, \[handleNativeNotificationAction\]\);/);
  assert.doesNotMatch(approvals, /\}, \[notifyApprovalEvent\]\);/);

  // The navigation callbacks stay stable across room-list snapshots.
  const navigate = readFileSync('src/app/hooks/useRoomNavigate.ts', 'utf8');
  assert.match(navigate, /resolveRoomRoutePath\(roomId, target, scopeRef\.current\),\s*\[\]/);
  assert.match(navigate, /\[navigate, startTransition, roomPath\]/);
  assert.doesNotMatch(navigate, /directRoomIds, developerTools\]/);
});

test('notification routes use the same direct and space resolution as in-app navigation', () => {
  const source = features();
  assert.doesNotMatch(source, /buildDesktopNotificationRoomRoute/);
  assert.equal((source.match(/route: roomPath\(/g) ?? []).length, 3);
});
