import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { NotificationType } from '../../../../types/matrix/room';
import {
  decideNotificationWithNativeOwner,
  dismissNotificationWithNativeOwner,
  eventMentionsUser,
  notificationRoomModeForType,
  setNotificationFocusWithNativeOwner,
} from '../nativeNotificationDecision';

test('renderer push-rule readings map to the closed Core mode vocabulary', () => {
  assert.equal(notificationRoomModeForType(NotificationType.AllMessages), 'all');
  assert.equal(notificationRoomModeForType(NotificationType.MentionsAndKeywords), 'mentions');
  assert.equal(notificationRoomModeForType(NotificationType.Mute), 'mute');
  assert.equal(notificationRoomModeForType(NotificationType.Default), 'default');
});

test('highlight observation reads explicit user mentions only', () => {
  assert.equal(
    eventMentionsUser({ 'm.mentions': { user_ids: ['@u:example.org'] } }, '@u:example.org'),
    true
  );
  assert.equal(
    eventMentionsUser({ 'm.mentions': { user_ids: ['@other:example.org'] } }, '@u:example.org'),
    false
  );
  assert.equal(eventMentionsUser({}, '@u:example.org'), false);
  assert.equal(eventMentionsUser(null, '@u:example.org'), false);
  assert.equal(eventMentionsUser({ 'm.mentions': { user_ids: ['@u:example.org'] } }, null), false);
});

test('decide routes only through matrix_notification_decide with closed observations', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const readback = await decideNotificationWithNativeOwner(
    {
      roomId: '!room:example.org',
      eventId: '$event:example.org',
      kind: 'message',
      title: 'Room',
      body: 'New inbox notification from @u:example.org',
      route: '/home/room/!room:example.org',
      suppressIfFocusedRoom: true,
      isEncrypted: false,
      roomMode: 'all',
      highlight: false,
      isOwnEvent: false,
    },
    async (command, args) => {
      calls.push({ command, args });
      return {
        available: true,
        value: {
          decision: 'show',
          candidate: {
            candidateId: 'notif-1',
            roomId: '!room:example.org',
            eventId: '$event:example.org',
            kind: 'message',
            title: 'Room',
            body: 'New inbox notification from @u:example.org',
            route: '/home/room/!room:example.org',
            suppressIfFocusedRoom: true,
            isEncrypted: false,
          },
        },
      };
    }
  );

  assert.equal(readback.decision, 'show');
  assert.deepEqual(calls, [
    {
      command: 'matrix_notification_decide',
      args: {
        request: {
          roomId: '!room:example.org',
          eventId: '$event:example.org',
          kind: 'message',
          title: 'Room',
          body: 'New inbox notification from @u:example.org',
          route: '/home/room/!room:example.org',
          suppressIfFocusedRoom: true,
          isEncrypted: false,
          roomMode: 'all',
          highlight: false,
          isOwnEvent: false,
        },
      },
    },
  ]);
});

test('decide rejects readbacks outside the closed show/suppress vocabulary', async () => {
  await assert.rejects(
    decideNotificationWithNativeOwner(
      {
        roomId: '!room:example.org',
        kind: 'message',
        title: 'Room',
        body: 'Hello',
        roomMode: 'all',
      },
      async () => ({
        available: true,
        value: { decision: 'maybe', candidate: undefined } as never,
      })
    ),
    /closed vocabulary/
  );

  // Show without a candidate is not an acceptable readback.
  await assert.rejects(
    decideNotificationWithNativeOwner(
      {
        roomId: '!room:example.org',
        kind: 'message',
        title: 'Room',
        body: 'Hello',
        roomMode: 'all',
      },
      async () => ({ available: true, value: { decision: 'show' } })
    ),
    /closed vocabulary/
  );
});

test('focus and dismiss route through their Core commands with no fallback', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke = async (command: string, args?: Record<string, unknown>) => {
    calls.push({ command, args });
    if (command === 'matrix_notification_dismiss') {
      return { available: true as const, value: true };
    }
    return { available: true as const, value: undefined };
  };

  await setNotificationFocusWithNativeOwner('!room:example.org', invoke);
  const dismissed = await dismissNotificationWithNativeOwner('notif-1', invoke);
  assert.equal(dismissed, true);
  assert.deepEqual(calls, [
    {
      command: 'matrix_notification_focus_set',
      args: { roomId: '!room:example.org' },
    },
    {
      command: 'matrix_notification_dismiss',
      args: { candidateId: 'notif-1' },
    },
  ]);
});

test('message notifications never decide mute policy in TypeScript', () => {
  const source = readFileSync(
    `${process.cwd()}/src/app/pages/client/ClientNonUIFeatures.tsx`,
    'utf8'
  );
  assert.match(source, /decideNotificationWithNativeOwner/);
  assert.match(source, /setNotificationFocusWithNativeOwner/);
  assert.match(source, /notificationRoomModeForType/);
  // The renderer observes the room mode and passes it in; only Core branches
  // on it. No TS mute matcher, no room-list polling on this path.
  assert.doesNotMatch(source, /getNotificationType\(mx, room\.roomId\) ===/);
  assert.doesNotMatch(source, /NotificationType\.Mute/);
  assert.doesNotMatch(source, /unreadNotificationCache/);
});
