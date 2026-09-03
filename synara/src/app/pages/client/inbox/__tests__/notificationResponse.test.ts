import assert from 'node:assert/strict';
import test from 'node:test';

import {
  HomeserverNotificationsError,
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
  for (const value of [undefined, {}, { notifications: 'none' }, { notifications: 7 }]) {
    assert.throws(() => normalizeNotificationsResponse(value), InvalidNotificationsResponseError);
  }
  assert.deepEqual(normalizeNotificationsResponse({ notifications: [] }), {
    notifications: [],
    next_token: undefined,
  });
});

test('notifications boundary surfaces homeserver error envelopes with their own diagnostic', () => {
  for (const value of [
    { errcode: 'M_UNKNOWN' },
    { errcode: 'M_UNKNOWN', error: 'Stale pagination token.' },
    { errcode: 'M_UNKNOWN', notifications: null },
  ]) {
    assert.throws(
      () => normalizeNotificationsResponse(value),
      (error: unknown) =>
        error instanceof HomeserverNotificationsError &&
        (error as HomeserverNotificationsError).name === 'M_UNKNOWN'
    );
  }
  const err = (() => {
    try {
      normalizeNotificationsResponse({ errcode: 'M_UNKNOWN', error: 'Stale pagination token.' });
    } catch (error) {
      return error as HomeserverNotificationsError;
    }
    throw new Error('expected a homeserver error');
  })();
  assert.equal(err.message, 'Stale pagination token.');
});

test('notifications boundary reads an explicit null list as an empty timeline', () => {
  assert.deepEqual(normalizeNotificationsResponse({ notifications: null }), {
    notifications: [],
    next_token: undefined,
  });
  assert.deepEqual(normalizeNotificationsResponse({ notifications: null, next_token: 'next' }), {
    notifications: [],
    next_token: 'next',
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
