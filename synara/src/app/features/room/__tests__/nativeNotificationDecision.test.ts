import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  decideNotificationWithNativeOwner,
  dismissNotificationWithNativeOwner,
  setNotificationFocusWithNativeOwner,
} from '../nativeNotificationDecision';

const SHOWN_CANDIDATE = {
  candidateId: 'notif-1',
  roomId: '!room:example.org',
  eventId: '$event:example.org',
  kind: 'message' as const,
  title: 'Room',
  body: 'New inbox notification from @u:example.org',
  route: '/home/room/!room:example.org',
  suppressIfFocusedRoom: true,
  isEncrypted: false,
};

test('decide submits identity and presentation only through matrix_notification_decide', async () => {
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
    },
    async (command, args) => {
      calls.push({ command, args });
      return {
        available: true,
        value: { decision: 'show', candidate: SHOWN_CANDIDATE, highlight: true, sound: true },
      };
    }
  );

  assert.equal(readback.decision, 'show');
  assert.equal(readback.highlight, true);
  assert.equal(readback.sound, true);
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
        },
      },
    },
  ]);
  // The wire never carries a renderer verdict: Core resolves these from the SDK.
  const request = calls[0]?.args?.request as Record<string, unknown>;
  for (const retired of ['roomMode', 'highlight', 'isOwnEvent', 'isEncrypted']) {
    assert.equal(retired in request, false, `${retired} must not be sent to Core`);
  }
});

test('decide accepts SDK push echoes only as optional booleans', async () => {
  const suppressed = await decideNotificationWithNativeOwner(
    { roomId: '!room:example.org', eventId: '$e', kind: 'message', title: 'Room', body: 'Hello' },
    async () => ({
      available: true,
      value: { decision: 'suppress', reason: 'push-rules-no-notify' },
    })
  );
  assert.equal(suppressed.decision, 'suppress');
  assert.equal(suppressed.reason, 'push-rules-no-notify');
  assert.equal(suppressed.highlight, undefined);

  await assert.rejects(
    decideNotificationWithNativeOwner(
      { roomId: '!room:example.org', kind: 'message', title: 'Room', body: 'Hello' },
      async () => ({
        available: true,
        value: { decision: 'show', candidate: SHOWN_CANDIDATE, highlight: 'yes' } as never,
      })
    ),
    /closed vocabulary/
  );
});

test('decide rejects readbacks outside the closed show/suppress vocabulary', async () => {
  await assert.rejects(
    decideNotificationWithNativeOwner(
      { roomId: '!room:example.org', kind: 'message', title: 'Room', body: 'Hello' },
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
      { roomId: '!room:example.org', kind: 'message', title: 'Room', body: 'Hello' },
      async () => ({ available: true, value: { decision: 'show' } })
    ),
    /closed vocabulary/
  );

  // Suppress carrying a candidate is contradictory.
  await assert.rejects(
    decideNotificationWithNativeOwner(
      { roomId: '!room:example.org', kind: 'message', title: 'Room', body: 'Hello' },
      async () => ({
        available: true,
        value: { decision: 'suppress', candidate: SHOWN_CANDIDATE },
      })
    ),
    /closed vocabulary/
  );

  // Core unavailable is an error, never a TS decision.
  await assert.rejects(
    decideNotificationWithNativeOwner(
      { roomId: '!room:example.org', kind: 'message', title: 'Room', body: 'Hello' },
      async () => ({ available: false as const, value: undefined })
    ),
    /unavailable/
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
  const dismissed = await dismissNotificationWithNativeOwner('notif-1', undefined, invoke);
  assert.equal(dismissed, true);
  // The delivery receipt travels with the acknowledgement using the closed
  // vocabulary; a plain acknowledgement sends no outcome key at all.
  await dismissNotificationWithNativeOwner('notif-2', 'delivered', invoke);
  await dismissNotificationWithNativeOwner('notif-3', 'failed', invoke);
  assert.deepEqual(calls, [
    {
      command: 'matrix_notification_focus_set',
      args: { roomId: '!room:example.org' },
    },
    {
      command: 'matrix_notification_dismiss',
      args: { candidateId: 'notif-1' },
    },
    {
      command: 'matrix_notification_dismiss',
      args: { candidateId: 'notif-2', outcome: 'delivered' },
    },
    {
      command: 'matrix_notification_dismiss',
      args: { candidateId: 'notif-3', outcome: 'failed' },
    },
  ]);

  await assert.rejects(
    dismissNotificationWithNativeOwner('notif-4', 'delivered', async () => ({
      available: false as const,
      value: undefined,
    })),
    /unavailable/
  );
});

test('the renderer acknowledges with the OS receipt after delivery and follows the SDK sound', () => {
  const root = process.cwd();
  const source = readFileSync(`${root}/src/app/pages/client/ClientNonUIFeatures.tsx`, 'utf8');
  const start = source.indexOf('function MessageNotifications()');
  assert.ok(start > 0);
  const end = source.indexOf('\nfunction ', start + 1);
  assert.ok(end > start);
  const feature = source.slice(start, end);

  // Delivery is awaited and its outcome reaches Core; the old fire-and-forget
  // `.catch(() => undefined)` around the OS call is gone from this path.
  assert.match(feature, /outcome = await notify\(/);
  assert.match(feature, /dismissNotificationWithNativeOwner\(shownCandidateId, outcome\)/);
  assert.match(feature, /const shown = await showPlatformNotification\(\{/);
  assert.match(feature, /return shown \? 'delivered' : 'failed'/);
  assert.doesNotMatch(feature, /\}\)\.catch\(\(\) => undefined\);\s*return;/);

  // Sound is the SDK push tweak Core echoed, gated by the preference, and
  // never plays for a delivery the OS refused.
  assert.match(feature, /notificationSound && readback\.sound === true && outcome !== 'failed'/);
  // No renderer retry loop: a failed receipt is recorded by Core only.
  assert.doesNotMatch(feature, /retryNotification|setTimeout\([^)]*notify/);
});

test('the renderer never reconstructs push-rule policy for message notifications', () => {
  const root = process.cwd();
  const feature = readFileSync(`${root}/src/app/pages/client/ClientNonUIFeatures.tsx`, 'utf8');
  const facade = readFileSync(
    `${root}/src/app/features/room/nativeNotificationDecision.ts`,
    'utf8'
  );

  assert.match(feature, /decideNotificationWithNativeOwner/);
  assert.match(feature, /setNotificationFocusWithNativeOwner/);
  assert.match(feature, /dismissNotificationWithNativeOwner/);

  // No JS push-rule stub, mode resolution, or mention/keyword matcher: Core
  // reads the SDK-evaluated push actions for the exact observed event.
  for (const source of [feature, facade]) {
    assert.doesNotMatch(source, /getNotificationType\(/);
    assert.doesNotMatch(source, /NotificationType\.Mute/);
    assert.doesNotMatch(source, /unreadNotificationCache/);
    assert.doesNotMatch(source, /nativePushRulesSnapshot/);
    assert.doesNotMatch(source, /nativeRoomNotificationsSnapshot/);
    assert.doesNotMatch(source, /resolveObservedNotificationRoomMode/);
    assert.doesNotMatch(source, /eventIsHighlightObservation/);
    assert.doesNotMatch(source, /notificationBodyContainsToken/);
    assert.doesNotMatch(source, /'m\.mentions'/);
    assert.doesNotMatch(source, /@room/);
    assert.doesNotMatch(source, /roomMode\s*[:=]/);
    assert.doesNotMatch(source, /highlight\s*:/);
    assert.doesNotMatch(source, /isOwnEvent/);
  }
});
