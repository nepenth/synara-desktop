import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { fetchNativeInboxNotifications } from '../nativeInboxNotifications';
import { InvalidNotificationsResponseError } from '../notificationResponse';

test('Inbox fetch uses the native owner and preserves highlight/pagination', async () => {
  const query = { from: 'exact/token+1', limit: 30, only: 'highlight' as const };
  const notifications = [
    {
      room_id: '!room:example.org',
      ts: 123,
      read: false,
      event: {
        event_id: '$event',
        sender: '@alice:example.org',
        type: 'm.room.message',
        origin_server_ts: 123,
        content: { msgtype: 'm.text', body: 'hello' },
      },
    },
  ];
  const page = await fetchNativeInboxNotifications(
    query,
    async <T>(command: string, args?: Record<string, unknown>) => {
      assert.equal(command, 'matrix_inbox_notifications');
      assert.deepEqual(args, query);
      return { available: true, value: { notifications, next_token: 'next' } as T };
    }
  );
  assert.deepEqual(page, { notifications, next_token: 'next' });
});

test('native empty Inbox is valid while unavailable, malformed, and server errors remain failures', async () => {
  const page = await fetchNativeInboxNotifications({}, async <T>() => ({
    available: true,
    value: { notifications: [] } as T,
  }));
  assert.deepEqual(page.notifications, []);
  await assert.rejects(
    fetchNativeInboxNotifications({}, async () => ({ available: false })),
    /unavailable/
  );
  await assert.rejects(
    fetchNativeInboxNotifications({}, async <T>() => ({ available: true, value: {} as T })),
    InvalidNotificationsResponseError
  );
  const failure = new Error('Notifications could not be loaded.');
  await assert.rejects(
    fetchNativeInboxNotifications({}, async () => {
      throw failure;
    }),
    (error) => error === failure
  );
});

test('inbox timeline reloads keep current groups until the native page returns', () => {
  const source = readFileSync('src/app/pages/client/inbox/Notifications.tsx', 'utf8');
  assert.doesNotMatch(source, /if \(!from\) \{\s*setNotificationTimeline\(\{ groups: \[\] \}\)/);
  assert.match(source, /sameNotificationTimeline/);
  assert.match(source, /allRoomsKey/);
  assert.match(source, /notificationTimeline\.groups\.length === 0/);
});
