import test from 'node:test';
import assert from 'node:assert/strict';
import { normalizeSystemNotificationRequest } from '../systemNotification';

test('normalizeSystemNotificationRequest creates a bounded default request', () => {
  assert.deepEqual(
    normalizeSystemNotificationRequest({
      title: '  Reminder  ',
      body: '  A saved reminder is due.  ',
      route: '/home/!room/$event/',
    }),
    {
      title: 'Reminder',
      body: 'A saved reminder is due.',
      route: '/home/!room/$event/',
      privacy: 'standard',
      sound: 'default',
    }
  );
});

test('normalizeSystemNotificationRequest rejects empty titles and external routes', () => {
  assert.equal(normalizeSystemNotificationRequest({ title: '   ' }), undefined);
  assert.deepEqual(
    normalizeSystemNotificationRequest({
      title: 'Open',
      route: 'https://example.org',
    }),
    {
      title: 'Open',
      body: undefined,
      route: undefined,
      privacy: 'standard',
      sound: 'default',
    }
  );
});

test('normalizeSystemNotificationRequest rejects unsupported internal routes', () => {
  assert.deepEqual(
    normalizeSystemNotificationRequest({
      title: 'Auth',
      route: '/login/matrix.org/',
    }),
    {
      title: 'Auth',
      body: undefined,
      route: undefined,
      privacy: 'standard',
      sound: 'default',
    }
  );
});

test('normalizeSystemNotificationRequest rejects oversized routes', () => {
  assert.deepEqual(
    normalizeSystemNotificationRequest({
      title: 'Long',
      route: `/home/${'x'.repeat(2_048)}/`,
    }),
    {
      title: 'Long',
      body: undefined,
      route: undefined,
      privacy: 'standard',
      sound: 'default',
    }
  );
});

test('normalizeSystemNotificationRequest preserves explicit privacy and sound policy', () => {
  assert.deepEqual(
    normalizeSystemNotificationRequest({
      title: 'Hidden',
      privacy: 'private',
      sound: 'silent',
    }),
    {
      title: 'Hidden',
      body: undefined,
      route: undefined,
      privacy: 'private',
      sound: 'silent',
    }
  );
});

test('normalizeSystemNotificationRequest preserves safe notification actions', () => {
  assert.deepEqual(
    normalizeSystemNotificationRequest({
      title: 'Approval',
      actions: [
        { id: ' agent-approval.approve-once ', label: ' Approve once ' },
        { id: 'agent-approval.deny', label: 'Deny' },
        { id: 'bad action id', label: 'Bad' },
      ],
      actionContext: {
        kind: ' agent-approval ',
        roomId: ' !room:matrix.org ',
        eventId: ' $event:matrix.org ',
      },
    }),
    {
      title: 'Approval',
      body: undefined,
      route: undefined,
      actions: [
        { id: 'agent-approval.approve-once', label: 'Approve once' },
        { id: 'agent-approval.deny', label: 'Deny' },
      ],
      actionContext: {
        kind: 'agent-approval',
        roomId: '!room:matrix.org',
        eventId: '$event:matrix.org',
      },
      privacy: 'standard',
      sound: 'default',
    }
  );
});
