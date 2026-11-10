import assert from 'node:assert/strict';
import test from 'node:test';

import {
  MESSAGE_NOTIFICATION_ACTIONS,
  messageReplyActionRoom,
} from '../messageNotificationActions';
import { normalizeSystemNotificationRequest } from '../systemNotification';

test('message notification actions keep the ids the shell handles', () => {
  assert.deepEqual(
    MESSAGE_NOTIFICATION_ACTIONS.map((action) => action.id),
    ['message.mark-read', 'message.reply']
  );
});

test('only a message Reply action names a room to open', () => {
  const context = { kind: 'message', roomId: ' !room:example.org ', eventId: '$event' };
  assert.equal(messageReplyActionRoom({ actionId: 'message.reply', context }), '!room:example.org');
  assert.equal(messageReplyActionRoom({ actionId: 'message.mark-read', context }), undefined);
  assert.equal(messageReplyActionRoom({ actionId: 'agent-approval.deny', context }), undefined);
  assert.equal(
    messageReplyActionRoom({
      actionId: 'message.reply',
      context: { ...context, kind: 'agent-approval' },
    }),
    undefined
  );
  assert.equal(
    messageReplyActionRoom({ actionId: 'message.reply', context: { kind: 'message' } }),
    undefined
  );
  assert.equal(
    messageReplyActionRoom({
      actionId: 'message.reply',
      context: { kind: 'message', roomId: '#alias:example.org' },
    }),
    undefined
  );
  assert.equal(messageReplyActionRoom({ actionId: 'message.reply' }), undefined);
});

test('the read-receipt setting travels only with a banner that has actions', () => {
  const request = {
    title: 'Room',
    actions: MESSAGE_NOTIFICATION_ACTIONS,
    actionContext: { kind: 'message', roomId: '!room:example.org', eventId: '$event' },
    publicReadReceipt: true,
  };
  assert.equal(normalizeSystemNotificationRequest(request)?.publicReadReceipt, true);
  assert.equal(
    normalizeSystemNotificationRequest({ ...request, publicReadReceipt: false })?.publicReadReceipt,
    false
  );
  assert.equal(
    normalizeSystemNotificationRequest({ title: 'Room', publicReadReceipt: true })
      ?.publicReadReceipt,
    undefined
  );
});
