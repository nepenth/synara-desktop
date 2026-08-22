import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';
import {
  createNativePresenceSubscription,
  Presence,
  setOwnPresenceNative,
  snapshotOwnPresenceNative,
  type NativePresenceDependencies,
  type NativePresenceInvoke,
  type NativePresenceListen,
} from '../nativePresence';

const userId = '@alice:example.org';
const snapshot = {
  userId,
  state: 'online',
  currentlyActive: true,
  lastActiveTs: 1_700_000_000_000,
  statusMsg: 'coffee',
};

const eventFor = (payload: unknown) => ({
  id: 1,
  event: 'matrix-presence-updated',
  payload,
});

function harness(
  options: {
    snapshotResult?: unknown;
    subscriptionResult?: unknown;
    invokeFailure?: string;
    listenFailure?: boolean;
  } = {}
): {
  dependencies: NativePresenceDependencies;
  calls: Array<{ command: string; args?: Record<string, unknown> }>;
  emit: (payload: unknown) => void;
  wasUnlistened: () => boolean;
} {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  let handler: ((event: ReturnType<typeof eventFor>) => void) | undefined;
  let unlistened = false;
  const invoke: NativePresenceInvoke = async (command, args) => {
    calls.push({ command, args });
    if (options.invokeFailure === command) throw new Error('native failure');
    if (command === 'matrix_presence_snapshot') {
      return {
        available: true,
        value: options.snapshotResult ?? {
          status: 'ready',
          sessionGeneration: 7,
          userId,
          snapshot,
        },
      };
    }
    if (command === 'matrix_presence_subscribe') {
      return {
        available: true,
        value: options.subscriptionResult ?? {
          subscriptionId: 'presence-7-0',
          userId,
          sessionGeneration: 7,
        },
      };
    }
    return { available: true, value: undefined };
  };
  const listen: NativePresenceListen = async (_event, nextHandler) => {
    if (options.listenFailure) throw new Error('listener failure');
    handler = nextHandler as unknown as (event: ReturnType<typeof eventFor>) => void;
    return () => {
      unlistened = true;
    };
  };
  return {
    dependencies: { desktopAvailable: true, invoke, listen },
    calls,
    emit: (payload) => handler?.(eventFor(payload)),
    wasUnlistened: () => unlistened,
  };
}

test('snapshot and subscription use the exact requested user id', async () => {
  const testHarness = harness();
  const values: unknown[] = [];
  const dispose = await createNativePresenceSubscription(
    userId,
    testHarness.dependencies,
    (value) => values.push(value)
  );

  assert.deepEqual(testHarness.calls.slice(0, 2), [
    { command: 'matrix_presence_snapshot', args: { userId } },
    { command: 'matrix_presence_subscribe', args: { userId } },
  ]);
  assert.deepEqual(values[0], {
    presence: 'online',
    status: 'coffee',
    active: true,
    lastActiveTs: 1_700_000_000_000,
  });
  dispose();
});

test('matching updates replace the initial profile presence', async () => {
  const testHarness = harness();
  const values: unknown[] = [];
  const dispose = await createNativePresenceSubscription(
    userId,
    testHarness.dependencies,
    (value) => values.push(value)
  );

  testHarness.emit({
    subscriptionId: 'presence-7-0',
    userId,
    sessionGeneration: 7,
    outcome: {
      status: 'ready',
      snapshot: { ...snapshot, state: 'unavailable', currentlyActive: false },
    },
  });
  assert.deepEqual(values.at(-1), {
    presence: 'unavailable',
    status: 'coffee',
    active: false,
    lastActiveTs: 1_700_000_000_000,
  });
  dispose();
});

test('wrong-owner and stale-generation events fail closed without merging state', async () => {
  const testHarness = harness();
  const values: unknown[] = [];
  const dispose = await createNativePresenceSubscription(
    userId,
    testHarness.dependencies,
    (value) => values.push(value)
  );

  testHarness.emit({
    subscriptionId: 'other-subscription',
    userId,
    sessionGeneration: 7,
    outcome: { status: 'ready', snapshot },
  });
  assert.equal(values.at(-1), undefined);

  testHarness.emit({
    subscriptionId: 'presence-7-0',
    userId: '@bob:example.org',
    sessionGeneration: 7,
    outcome: { status: 'ready', snapshot: { ...snapshot, userId: '@bob:example.org' } },
  });
  assert.equal(values.at(-1), undefined);

  testHarness.emit({
    subscriptionId: 'presence-7-0',
    userId,
    sessionGeneration: 8,
    outcome: { status: 'ready', snapshot },
  });
  assert.equal(values.at(-1), undefined);
  dispose();
});

test('malformed subscription ids fail closed without accepting the update', async () => {
  const testHarness = harness();
  const values: unknown[] = [];
  const dispose = await createNativePresenceSubscription(
    userId,
    testHarness.dependencies,
    (value) => values.push(value)
  );

  testHarness.emit({
    subscriptionId: 'x'.repeat(256),
    userId,
    sessionGeneration: 7,
    outcome: { status: 'ready', snapshot },
  });
  assert.equal(values.at(-1), undefined);
  dispose();
});

test('unknown, unavailable, malformed, and failed native results show no badge', async () => {
  const unknown = harness({
    snapshotResult: { status: 'unknown', sessionGeneration: 7, userId },
  });
  const unknownValues: unknown[] = [];
  await createNativePresenceSubscription(userId, unknown.dependencies, (value) =>
    unknownValues.push(value)
  );
  assert.equal(unknownValues.at(-1), undefined);

  for (const invalidSnapshot of [
    { ...snapshot, statusMsg: 'x'.repeat(257) },
    { ...snapshot, lastActiveTs: -1 },
    { ...snapshot, lastActiveTs: Number.MAX_SAFE_INTEGER + 1 },
    { ...snapshot, lastActiveTs: Number.NaN },
    { ...snapshot, accessToken: 'secret' },
  ]) {
    const malformed = harness({
      snapshotResult: {
        status: 'ready',
        sessionGeneration: 7,
        userId,
        snapshot: invalidSnapshot,
      },
    });
    const malformedValues: unknown[] = [];
    await createNativePresenceSubscription(userId, malformed.dependencies, (value) =>
      malformedValues.push(value)
    );
    assert.equal(malformedValues.at(-1), undefined);
  }

  const failedSubscription = harness({
    subscriptionResult: { subscriptionId: 'presence-7-0', userId, sessionGeneration: 8 },
  });
  const failedValues: unknown[] = [];
  await createNativePresenceSubscription(userId, failedSubscription.dependencies, (value) =>
    failedValues.push(value)
  );
  assert.equal(failedValues.at(-1), undefined);
});

test('missing desktop/native bridge, invoke failure, and listener failure stay native-only', async () => {
  const unavailable = harness();
  unavailable.dependencies.desktopAvailable = false;
  const unavailableValues: unknown[] = [];
  await createNativePresenceSubscription(userId, unavailable.dependencies, (value) =>
    unavailableValues.push(value)
  );
  assert.equal(unavailableValues.at(-1), undefined);
  assert.deepEqual(unavailable.calls, []);

  const invalidUser = harness();
  const invalidValues: unknown[] = [];
  await createNativePresenceSubscription('alice', invalidUser.dependencies, (value) =>
    invalidValues.push(value)
  );
  assert.equal(invalidValues.at(-1), undefined);
  assert.deepEqual(invalidUser.calls, []);

  for (const failed of [
    harness({ invokeFailure: 'matrix_presence_snapshot' }),
    harness({ listenFailure: true }),
  ]) {
    const values: unknown[] = [];
    await createNativePresenceSubscription(userId, failed.dependencies, (value) =>
      values.push(value)
    );
    assert.equal(values.at(-1), undefined);
  }
});

test('an unavailable native update clears an existing badge', async () => {
  const testHarness = harness();
  const values: unknown[] = [];
  const dispose = await createNativePresenceSubscription(
    userId,
    testHarness.dependencies,
    (value) => values.push(value)
  );

  testHarness.emit({
    subscriptionId: 'presence-7-0',
    userId,
    sessionGeneration: 7,
    outcome: { status: 'unavailable', diagnosticId: 'v-presence-store-read-failed' },
  });
  assert.equal(values.at(-1), undefined);
  dispose();
});

test('dispose unsubscribes and late events cannot update the profile', async () => {
  const testHarness = harness();
  const values: unknown[] = [];
  const dispose = await createNativePresenceSubscription(
    userId,
    testHarness.dependencies,
    (value) => values.push(value)
  );
  const countBeforeDispose = values.length;
  dispose();
  assert.equal(testHarness.wasUnlistened(), true);
  assert.deepEqual(testHarness.calls.at(-1), {
    command: 'matrix_presence_unsubscribe',
    args: { subscriptionId: 'presence-7-0' },
  });
  dispose();
  assert.equal(
    testHarness.calls.filter(({ command }) => command === 'matrix_presence_unsubscribe').length,
    1
  );
  testHarness.emit({
    subscriptionId: 'presence-7-0',
    userId,
    sessionGeneration: 7,
    outcome: { status: 'ready', snapshot },
  });
  assert.equal(values.length, countBeforeDispose);
});

test('setOwnPresenceNative invokes matrix_presence_set with state and no userId', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativePresenceInvoke = async (command, args) => {
    calls.push({ command, args });
    return { available: true, value: { status: 'ok' } };
  };
  await setOwnPresenceNative(Presence.Online, undefined, invoke);
  assert.deepEqual(calls, [{ command: 'matrix_presence_set', args: { state: 'online' } }]);
  assert.equal(Object.prototype.hasOwnProperty.call(calls[0]?.args ?? {}, 'userId'), false);

  calls.length = 0;
  await setOwnPresenceNative(Presence.Unavailable, undefined, invoke);
  assert.deepEqual(calls, [{ command: 'matrix_presence_set', args: { state: 'unavailable' } }]);
  await setOwnPresenceNative(Presence.Offline, undefined, invoke);
  assert.equal(calls.at(-1)?.args?.state, 'offline');
  assert.equal(Object.prototype.hasOwnProperty.call(calls.at(-1)?.args ?? {}, 'userId'), false);
});

test('snapshotOwnPresenceNative returns undefined on unknown without throwing', async () => {
  const presence = await snapshotOwnPresenceNative(userId, async () => ({
    available: true,
    value: { status: 'unknown', sessionGeneration: 7, userId },
  }));
  assert.equal(presence, undefined);
});

test('setOwnPresenceNative fails closed when invoke is unavailable', async () => {
  await assert.rejects(
    () => setOwnPresenceNative(Presence.Online, undefined, async () => ({ available: false })),
    /unavailable/
  );
  await assert.rejects(
    () =>
      setOwnPresenceNative('away' as Presence, undefined, async () => {
        throw new Error('should not invoke');
      }),
    /unavailable/
  );
});

test('profile presence path has no JavaScript Matrix presence owner', () => {
  const files = [
    'src/app/components/user-profile/UserRoomProfile.tsx',
    'src/app/components/user-profile/UserHero.tsx',
    'src/app/components/presence/Presence.tsx',
    'src/app/features/matrix-presence/nativePresence.ts',
  ];
  const source = files.map((file) => readFileSync(join(process.cwd(), file), 'utf8')).join('\n');
  assert.equal(source.includes('matrix-js-sdk'), false);
  assert.equal(source.includes('UserEvent'), false);
  assert.equal(source.includes('useUserPresence'), false);
  assert.equal(source.includes('presenceStatusMsg'), false);
  assert.equal(source.includes('UserEvent.Presence'), false);
  assert.equal(source.includes('getPresence'), false);
  assert.equal(source.includes('currentlyActive'), true);
});
