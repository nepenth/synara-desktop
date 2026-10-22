import { readFileSync } from 'node:fs';
import test from 'node:test';
import assert from 'node:assert/strict';
import {
  createApprovalAlertWithdrawal,
  shouldDeliverNotificationKind,
  shouldSubmitApprovalObservation,
} from '../notificationDelivery';
import { notifiedEventIdsCache } from '../notificationCaches';
import {
  buildNativeObservedNotificationPresentation,
  deliverNativeObservedNotificationCandidate,
} from '../../features/room/nativeNotificationPresentation';
import type { NativeNotificationCandidate } from '../../features/room/nativeNotificationDecision';

const ROOM = '!agent:example.org';
const NOW = 1_700_000_000_000;

const features = () => readFileSync('src/app/pages/client/ClientNonUIFeatures.tsx', 'utf8');
const section = (source: string, start: string) => {
  const from = source.indexOf(start);
  assert.ok(from >= 0, start);
  const to = source.indexOf('\nfunction ', from + 1);
  return source.slice(from, to < 0 ? undefined : to);
};

const candidateFor = (
  eventId: string,
  kind: NativeNotificationCandidate['kind']
): NativeNotificationCandidate => ({
  candidateId: `candidate-${eventId}`,
  roomId: ROOM,
  eventId,
  kind,
  title: kind === 'agent_approval' ? 'Approval Required: Dangerous Command' : 'Agent room',
  body: kind === 'agent_approval' ? 'Review a request in Agent room.' : 'New message',
  suppressIfFocusedRoom: kind !== 'agent_approval',
  isEncrypted: false,
});

/**
 * The approval pump's steps around Core: skip expired or already-notified
 * prompts, deliver what Core shows through the kind gate, then remember the
 * event and arm its expiry withdrawal.
 */
const createPump = (showNotifications: boolean) => {
  const posted: string[] = [];
  const withdrawn: string[] = [];
  const timers = new Map<number, () => void>();
  let nextTimer = 0;
  let now = NOW;
  const alerts = createApprovalAlertWithdrawal({
    withdraw: (eventId) => withdrawn.push(eventId),
    now: () => now,
    schedule: (callback) => {
      nextTimer += 1;
      timers.set(nextTimer, callback);
      return nextTimer as unknown as ReturnType<typeof setTimeout>;
    },
    cancel: (handle) => {
      timers.delete(handle as unknown as number);
    },
  });
  const observe = async ({
    eventId,
    kind = 'agent_approval',
    expired = false,
    expiresAt = NOW + 300_000,
  }: {
    eventId: string;
    /** What Core decided: an approval over its room's budget comes back as a message. */
    kind?: NativeNotificationCandidate['kind'];
    expired?: boolean;
    expiresAt?: number;
  }) => {
    if (
      !shouldSubmitApprovalObservation({
        expired,
        alreadyNotified: notifiedEventIdsCache.has(eventId),
      })
    )
      return;
    await deliverNativeObservedNotificationCandidate({
      candidate: candidateFor(eventId, kind),
      observedGeneration: 1,
      currentGeneration: () => 1,
      deliver: async (candidate) => {
        if (!shouldDeliverNotificationKind({ kind: candidate.kind, showNotifications })) {
          return undefined;
        }
        posted.push(eventId);
        return 'delivered';
      },
      commit: (outcome) => {
        notifiedEventIdsCache.add(eventId);
        if (outcome === 'delivered') alerts.track(eventId, expiresAt);
      },
      acknowledge: async () => undefined,
    });
  };
  const expire = (at: number) => {
    now = at;
    [...timers].forEach(([handle, fire]) => {
      timers.delete(handle);
      fire();
    });
  };
  return { observe, expire, posted, withdrawn, alerts };
};

test('approval alerts get through Desktop Notifications off and tray Do Not Disturb', async () => {
  notifiedEventIdsCache.clear();
  assert.equal(
    shouldDeliverNotificationKind({ kind: 'agent_approval', showNotifications: false }),
    true
  );
  assert.equal(shouldDeliverNotificationKind({ kind: 'message', showNotifications: false }), false);
  assert.equal(shouldDeliverNotificationKind({ kind: 'message', showNotifications: true }), true);
  assert.equal(shouldDeliverNotificationKind({ kind: 'invite', showNotifications: false }), false);

  const pump = createPump(false);
  await pump.observe({ eventId: '$approval' });
  assert.deepEqual(pump.posted, ['$approval']);
  assert.equal(pump.alerts.has('$approval'), true);

  // A message, or an approval Core downgraded past the room's budget, stays held back.
  await pump.observe({ eventId: '$message', kind: 'message' });
  await pump.observe({ eventId: '$flood', kind: 'message' });
  assert.deepEqual(pump.posted, ['$approval']);
  assert.equal(pump.alerts.has('$flood'), false);
  notifiedEventIdsCache.clear();
});

test('under Do Not Disturb an approval alert is still deduped and withdrawn at expiry', async () => {
  notifiedEventIdsCache.clear();
  const pump = createPump(false);
  await pump.observe({ eventId: '$approval', expiresAt: NOW + 300_000 });
  await pump.observe({ eventId: '$approval', expiresAt: NOW + 300_000 });
  assert.deepEqual(pump.posted, ['$approval'], 'one alert per event');

  await pump.observe({ eventId: '$expired', expired: true });
  assert.deepEqual(pump.posted, ['$approval'], 'an expired prompt never alerts');
  assert.equal(notifiedEventIdsCache.has('$expired'), false);

  pump.expire(NOW + 300_000);
  assert.deepEqual(pump.withdrawn, ['$approval']);
  assert.equal(pump.alerts.has('$approval'), false);

  assert.equal(shouldSubmitApprovalObservation({ expired: false, alreadyNotified: false }), true);
  assert.equal(shouldSubmitApprovalObservation({ expired: true, alreadyNotified: false }), false);
  assert.equal(shouldSubmitApprovalObservation({ expired: false, alreadyNotified: true }), false);
  notifiedEventIdsCache.clear();
});

test('an approval alert keeps its own sound under Do Not Disturb', () => {
  const source = { roomId: ROOM, eventId: '$approval', sessionGeneration: 1 };
  // The approval pump passes no message sound: the native banner asks the OS
  // for the default (time-sensitive) sound itself.
  assert.equal(
    buildNativeObservedNotificationPresentation(candidateFor('$approval', 'agent_approval'), source)
      .sound,
    'default'
  );

  const approvals = section(features(), 'function AgentApprovalNotifications()');
  assert.match(
    approvals,
    /if \(outcome === 'delivered' && !supportsPlatformSystemNotifications\(\)\) playSound\(\);/
  );
});

test('both notification routes deliver through the kind gate', () => {
  const source = features();
  const messages = section(source, 'function MessageNotifications()');
  const approvals = section(source, 'function AgentApprovalNotifications()');
  for (const route of [messages, approvals]) {
    assert.match(
      route,
      /shouldDeliverNotificationKind\(\{ kind: candidate\.kind, showNotifications \}\) &&\s*\(supportsPlatformSystemNotifications\(\) \|\| notificationPermission\('granted'\)\)/
    );
    assert.doesNotMatch(route, /if \(\s*showNotifications &&/);
  }
  assert.match(
    approvals,
    /shouldSubmitApprovalObservation\(\{\s*expired: agentApproval\.expired,\s*alreadyNotified: notifiedEventIdsCache\.has\(eventId\),\s*\}\)/
  );
  assert.match(approvals, /approvalAlerts\.track\(eventId, agentApproval\.expiresAt\)/);
  // An approval action's result notice is posted under Do Not Disturb too.
  assert.match(approvals, /if \(!notice \|\| !supportsPlatformSystemNotifications\(\)\) return;/);
  assert.doesNotMatch(approvals, /showNotificationsRef/);

  // Invites and reminders keep following the switch.
  assert.match(
    section(source, 'function InviteNotifications()'),
    /!delivery\.showNotifications \|\|/
  );
  assert.match(section(source, 'function LaterReminderNotifications()'), /showNotifications &&/);
});
