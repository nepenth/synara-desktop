import assert from 'node:assert/strict';
import test from 'node:test';
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
