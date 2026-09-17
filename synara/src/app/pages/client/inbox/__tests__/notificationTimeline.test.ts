import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import type { NotificationReading } from '../notificationResponse';
import {
  groupNotifications,
  notificationGroupsEquivalent,
  shouldResetNotificationTimeline,
} from '../notificationTimeline';

const reading = (roomId: string, eventId: string): NotificationReading => ({
  room_id: roomId,
  event: {
    event_id: eventId,
    type: 'm.room.message',
    sender: '@alice:example.org',
    origin_server_ts: 1,
    content: { msgtype: 'm.text', body: 'hi' },
  },
});

test('shouldResetNotificationTimeline only wipes on first load or highlight changes', () => {
  assert.equal(
    shouldResetNotificationTimeline({
      from: 'page-2',
      hasGroups: true,
      highlightFilterChanged: false,
    }),
    false
  );
  assert.equal(
    shouldResetNotificationTimeline({
      from: undefined,
      hasGroups: true,
      highlightFilterChanged: false,
    }),
    false
  );
  assert.equal(
    shouldResetNotificationTimeline({
      from: undefined,
      hasGroups: false,
      highlightFilterChanged: false,
    }),
    true
  );
  assert.equal(
    shouldResetNotificationTimeline({
      from: undefined,
      hasGroups: true,
      highlightFilterChanged: true,
    }),
    true
  );
});

test('notificationGroupsEquivalent compares room ids and event ids in order', () => {
  const left = groupNotifications(
    [reading('!a:ex', '$1'), reading('!a:ex', '$2'), reading('!b:ex', '$3')],
    new Set(['!a:ex', '!b:ex'])
  );
  const same = groupNotifications(
    [reading('!a:ex', '$1'), reading('!a:ex', '$2'), reading('!b:ex', '$3')],
    new Set(['!a:ex', '!b:ex'])
  );
  const reordered = groupNotifications(
    [reading('!b:ex', '$3'), reading('!a:ex', '$1'), reading('!a:ex', '$2')],
    new Set(['!a:ex', '!b:ex'])
  );
  const differentEvent = groupNotifications(
    [reading('!a:ex', '$1'), reading('!a:ex', '$9'), reading('!b:ex', '$3')],
    new Set(['!a:ex', '!b:ex'])
  );

  assert.equal(notificationGroupsEquivalent(left, same), true);
  assert.equal(notificationGroupsEquivalent(left, reordered), false);
  assert.equal(notificationGroupsEquivalent(left, differentEvent), false);
});

test('groupNotifications drops rooms that are not currently joined', () => {
  const groups = groupNotifications(
    [reading('!a:ex', '$1'), reading('!left:ex', '$2')],
    new Set(['!a:ex'])
  );
  assert.deepEqual(
    groups.map((group) => group.roomId),
    ['!a:ex']
  );
});

test('inbox first-page load does not depend on joined-room list identity', () => {
  const source = readFileSync('src/app/pages/client/inbox/Notifications.tsx', 'utf8');
  const hook = source.slice(
    source.indexOf('const useNotificationTimeline'),
    source.indexOf('type RoomNotificationsGroupProps')
  );

  assert.match(hook, /allRoomsRef\.current = allRooms/);
  assert.match(hook, /new Set\(allRoomsRef\.current\)/);
  assert.doesNotMatch(hook, /allJoinedRooms/);
  assert.match(hook, /\[paginationLimit, onlyHighlight, fetchNotifications\]/);
  assert.doesNotMatch(hook, /if \(!from\) \{\s*setNotificationTimeline\(\{ groups: \[\] \}\);/);
  assert.match(source, /useEffect\(\(\) => \{\s*loadTimeline\(\);\s*\}, \[loadTimeline\]\)/);
});
