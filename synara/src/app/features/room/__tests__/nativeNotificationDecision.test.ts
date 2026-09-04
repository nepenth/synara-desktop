import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { NotificationType } from '../../../../types/matrix/room';
import {
  decideNotificationWithNativeOwner,
  dismissNotificationWithNativeOwner,
  effectiveNotificationRoomMode,
  eventIsHighlightObservation,
  eventMentionsUser,
  notificationBodyContainsToken,
  notificationRoomModeForType,
  resolveObservedNotificationRoomMode,
  roomOverrideMapFromSnapshots,
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

const ALL_ON_FLAGS = {
  userMention: true,
  displayName: true,
  userName: true,
  roomMention: true,
  atRoom: true,
};

test('muted native override resolves to mute, not inherited default', () => {
  const rooms = roomOverrideMapFromSnapshots([
    { roomId: '!muted:example.org', mode: 'mute' },
    { roomId: '!mentions:example.org', mode: 'mentions' },
    { roomId: '!ignored:example.org', mode: 'default' },
  ]);
  assert.equal(rooms.get('!muted:example.org'), 'mute');
  assert.equal(
    resolveObservedNotificationRoomMode({
      userDefined: rooms.get('!muted:example.org'),
      isEncrypted: false,
      isDirect: false,
      defaults: { dm: 'all', dmEncrypted: 'all', group: 'all', groupEncrypted: 'all' },
    }),
    'mute'
  );
  assert.equal(
    resolveObservedNotificationRoomMode({
      userDefined: rooms.get('!mentions:example.org'),
      isEncrypted: false,
      isDirect: false,
      defaults: { dm: 'all', dmEncrypted: 'all', group: 'all', groupEncrypted: 'all' },
    }),
    'mentions'
  );
});

test('rooms without an override inherit account defaults and fail closed', () => {
  const defaults = {
    dm: 'mute',
    dmEncrypted: 'mentions',
    group: 'all',
    groupEncrypted: 'mute',
  } as const;
  assert.equal(
    effectiveNotificationRoomMode({
      userDefined: 'default',
      isEncrypted: false,
      isDirect: true,
      defaults,
    }),
    'mute'
  );
  assert.equal(
    resolveObservedNotificationRoomMode({
      userDefined: undefined,
      isEncrypted: true,
      isDirect: false,
      defaults,
    }),
    'mute'
  );
  assert.equal(
    resolveObservedNotificationRoomMode({
      userDefined: 'default',
      listMode: 'all',
      isEncrypted: false,
      isDirect: false,
      defaults: null,
    }),
    'all'
  );
  assert.equal(
    resolveObservedNotificationRoomMode({
      userDefined: 'default',
      isEncrypted: false,
      isDirect: false,
    }),
    'mentions'
  );
});

test('highlight observation covers mentions, @room, keywords, and skips ciphertext', () => {
  assert.equal(
    eventIsHighlightObservation({
      content: { 'm.mentions': { user_ids: ['@u:example.org'] } },
      userId: '@u:example.org',
      isEncrypted: false,
      flags: ALL_ON_FLAGS,
    }),
    true
  );
  assert.equal(
    eventIsHighlightObservation({
      content: { 'm.mentions': { room: true } },
      userId: '@u:example.org',
      isEncrypted: true,
      body: 'ciphertext-must-not-match keyword',
      keywords: ['keyword'],
      flags: ALL_ON_FLAGS,
    }),
    true
  );
  assert.equal(
    eventIsHighlightObservation({
      content: { body: 'please see @room later' },
      userId: '@u:example.org',
      isEncrypted: false,
      body: 'please see @room later',
      flags: ALL_ON_FLAGS,
    }),
    true
  );
  assert.equal(
    eventIsHighlightObservation({
      content: { body: 'ship the launch keyword today' },
      userId: '@u:example.org',
      isEncrypted: false,
      body: 'ship the launch keyword today',
      keywords: ['keyword'],
      flags: ALL_ON_FLAGS,
    }),
    true
  );
  assert.equal(
    eventIsHighlightObservation({
      content: { body: 'hey Alice Smith' },
      userId: '@u:example.org',
      isEncrypted: false,
      body: 'hey Alice Smith',
      displayName: 'Alice Smith',
      flags: ALL_ON_FLAGS,
    }),
    true
  );
  assert.equal(
    eventIsHighlightObservation({
      content: { body: 'ping @u later' },
      userId: '@u:example.org',
      isEncrypted: false,
      body: 'ping @u later',
      localpart: 'u',
      flags: ALL_ON_FLAGS,
    }),
    true
  );
  assert.equal(
    eventIsHighlightObservation({
      content: { ciphertext: 'AAAA', body: 'keyword' },
      userId: '@u:example.org',
      isEncrypted: true,
      body: 'keyword',
      keywords: ['keyword'],
      flags: ALL_ON_FLAGS,
    }),
    false
  );
  assert.equal(notificationBodyContainsToken('this has keyword inside', 'keyword'), true);
  assert.equal(notificationBodyContainsToken('keywords', 'keyword'), false);
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
  assert.match(source, /dismissNotificationWithNativeOwner/);
  assert.match(source, /resolveObservedNotificationRoomMode/);
  assert.match(source, /nativeRoomNotificationsSnapshot/);
  assert.match(source, /nativePushRulesSnapshot/);
  assert.match(source, /eventIsHighlightObservation/);
  // The renderer observes Core snapshots and passes a resolved mode; only
  // Core branches on mute/mentions. No JS push-rule stub, no TS mute matcher.
  assert.doesNotMatch(source, /getNotificationType\(/);
  assert.doesNotMatch(source, /notificationRoomModeForType/);
  assert.doesNotMatch(source, /NotificationType\.Mute/);
  assert.doesNotMatch(source, /unreadNotificationCache/);
});
