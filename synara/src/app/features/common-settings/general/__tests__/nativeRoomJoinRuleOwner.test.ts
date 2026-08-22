import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import type { DesktopEvent, DesktopInvokeResult } from '../../../../utils/desktop';
import {
  createNativeRoomJoinRuleOwner,
  parseRoomJoinRuleSnapshot,
  type NativeRoomJoinRuleDependencies,
  type NativeRoomJoinRuleListen,
  type NativeRoomJoinRuleState,
} from '../nativeRoomJoinRuleOwner';

const session = {
  status: 'logged_in',
  user_id: '@alice:example.org',
  device_id: 'DEVICE',
  homeserver_url: 'https://matrix.example.org',
  sessionGeneration: 7,
};

const readySnapshot = (joinRule: string) => ({
  status: 'ok',
  roomId: '!room:example.org',
  sessionGeneration: 7,
  joinRule,
});

const baseDependencies = (
  invoke: NativeRoomJoinRuleDependencies['invoke'],
  listen: NativeRoomJoinRuleDependencies['listen']
): NativeRoomJoinRuleDependencies => ({
  desktopAvailable: true,
  invoke,
  listen,
});

test('join-rule snapshot parser accepts only the six exact product values', () => {
  for (const joinRule of [
    'public',
    'knock',
    'invite',
    'restricted',
    'knock_restricted',
    'private',
  ]) {
    assert.equal(
      parseRoomJoinRuleSnapshot(readySnapshot(joinRule), '!room:example.org', 7).joinRule,
      joinRule
    );
  }
  for (const value of [
    readySnapshot('custom'),
    { ...readySnapshot('public'), extra: true },
    { ...readySnapshot('public'), roomId: '!other:example.org' },
    { ...readySnapshot('public'), sessionGeneration: 8 },
    { ...readySnapshot('public'), accessToken: 'secret' },
  ]) {
    assert.throws(() => parseRoomJoinRuleSnapshot(value, '!room:example.org', 7));
  }
});

test('native join-rule owner installs listener before session and snapshot reads', async () => {
  const calls: string[] = [];
  let handler: ((event: DesktopEvent<unknown>) => void) | undefined;
  const invoke = async (
    command: string,
    args?: Record<string, unknown>
  ): Promise<DesktopInvokeResult<unknown>> => {
    calls.push(command);
    if (command === 'matrix_session_snapshot') return { available: true, value: session };
    assert.equal(command, 'matrix_room_join_rule_snapshot');
    assert.deepEqual(args, { roomId: '!room:example.org', sessionGeneration: 7 });
    return { available: true, value: readySnapshot('public') };
  };
  const listen: NativeRoomJoinRuleListen = async (event, nextHandler) => {
    calls.push(event);
    handler = nextHandler as (event: DesktopEvent<unknown>) => void;
    return () => undefined;
  };
  const states: NativeRoomJoinRuleState[] = [];
  await createNativeRoomJoinRuleOwner(
    '!room:example.org',
    baseDependencies(invoke, listen),
    (state) => states.push(state)
  );

  assert.deepEqual(calls, [
    'matrix-room-join-rule-updated',
    'matrix_session_snapshot',
    'matrix_room_join_rule_snapshot',
  ]);
  assert.deepEqual(states.at(-1), {
    status: 'ready',
    userId: '@alice:example.org',
    snapshot: readySnapshot('public'),
  });
  assert.ok(handler);
});

test('stale, malformed, and unavailable updates clear a previously ready gate', async () => {
  let handler: ((event: DesktopEvent<unknown>) => void) | undefined;
  const listen: NativeRoomJoinRuleListen = async (_event, nextHandler) => {
    handler = nextHandler as (event: DesktopEvent<unknown>) => void;
    return () => undefined;
  };
  const invoke = async (command: string): Promise<DesktopInvokeResult<unknown>> =>
    command === 'matrix_session_snapshot'
      ? { available: true, value: session }
      : { available: true, value: readySnapshot('knock') };
  const states: NativeRoomJoinRuleState[] = [];
  await createNativeRoomJoinRuleOwner(
    '!room:example.org',
    baseDependencies(invoke, listen),
    (state) => states.push(state)
  );
  assert.equal(states.at(-1)?.status, 'ready');

  handler?.({
    event: 'matrix-room-join-rule-updated',
    id: 1,
    payload: {
      status: 'ready',
      roomId: '!room:example.org',
      sessionGeneration: 6,
      joinRule: 'public',
    },
  });
  assert.equal(states.at(-1)?.status, 'error');

  handler?.({
    event: 'matrix-room-join-rule-updated',
    id: 2,
    payload: { status: 'unavailable', roomId: '!room:example.org', sessionGeneration: 7 },
  });
  assert.equal(states.at(-1)?.status, 'error');

  handler?.({
    event: 'matrix-room-join-rule-updated',
    id: 3,
    payload: {
      status: 'ready',
      roomId: '!room:example.org',
      sessionGeneration: 7,
      joinRule: 'custom',
    },
  });
  assert.equal(states.at(-1)?.status, 'error');
});

test('RoomJoinRules uses native join-rule write on native sessions', () => {
  const source = readFileSync(
    join(process.cwd(), 'src/app/features/common-settings/general/RoomJoinRules.tsx'),
    'utf8'
  );
  assert.match(source, /isNativeMatrixSession\(\)/);
  assert.match(source, /matrix_room_set_join_rule/);
  assert.match(source, /allowRoomIds/);
  assert.match(source, /roomIdToParents/);
});
