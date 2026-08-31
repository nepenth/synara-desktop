import assert from 'node:assert/strict';
import test from 'node:test';

import {
  InvalidNotificationsResponseError,
  normalizeNotificationsResponse,
} from '../notificationResponse';

const validNotification = {
  room_id: '!room:example.org',
  event: {
    event_id: '$event:example.org',
    type: 'm.room.message',
    sender: '@alice:example.org',
    origin_server_ts: 123,
    content: { msgtype: 'm.text', body: 'hello' },
  },
};

test('notifications boundary rejects malformed envelopes instead of showing a false empty Inbox', () => {
  for (const value of [undefined, {}, { notifications: null }, { errcode: 'M_UNKNOWN' }]) {
    assert.throws(() => normalizeNotificationsResponse(value), InvalidNotificationsResponseError);
  }
  assert.deepEqual(normalizeNotificationsResponse({ notifications: [] }), {
    notifications: [],
    next_token: undefined,
  });
});

test('notifications boundary retains only render-safe entries and string pagination tokens', () => {
  const normalized = normalizeNotificationsResponse({
    notifications: [validNotification, null, { room_id: '!room:example.org' }],
    next_token: 'next',
  });
  assert.deepEqual(normalized, { notifications: [validNotification], next_token: 'next' });
  assert.equal(
    normalizeNotificationsResponse({ notifications: [validNotification], next_token: 7 })
      .next_token,
    undefined
  );
});
