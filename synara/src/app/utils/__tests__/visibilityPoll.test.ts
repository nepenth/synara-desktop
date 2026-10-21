import assert from 'node:assert/strict';
import test from 'node:test';

import {
  HIDDEN_POLL_INTERVAL_MS,
  startVisibilityAwarePoll,
  startVisibleOnlyPoll,
  VISIBLE_POLL_INTERVAL_MS,
  type VisibilityPollEnvironment,
} from '../visibilityPoll';

const fakeEnvironment = () => {
  let hidden = false;
  let nextId = 1;
  const timers = new Map<number, { callback: () => void; ms: number }>();
  const listeners = new Set<() => void>();
  const environment: VisibilityPollEnvironment = {
    isHidden: () => hidden,
    setTimeout: (callback, ms) => {
      const id = nextId;
      nextId += 1;
      timers.set(id, { callback, ms });
      return id;
    },
    clearTimeout: (id) => {
      timers.delete(id);
    },
    onVisibilityChange: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
  return {
    environment,
    timers,
    listeners,
    setHidden: (value: boolean) => {
      hidden = value;
      listeners.forEach((listener) => listener());
    },
    fire: () => {
      const [[id, timer]] = [...timers.entries()];
      timers.delete(id);
      timer.callback();
    },
  };
};

test('polls at the visible cadence and slows down while hidden', () => {
  const fake = fakeEnvironment();
  let ticks = 0;
  const stop = startVisibilityAwarePoll(
    () => {
      ticks += 1;
    },
    VISIBLE_POLL_INTERVAL_MS,
    HIDDEN_POLL_INTERVAL_MS,
    fake.environment
  );
  assert.equal(ticks, 0, 'callers refresh on mount themselves');
  assert.deepEqual(
    [...fake.timers.values()].map((timer) => timer.ms),
    [VISIBLE_POLL_INTERVAL_MS]
  );
  fake.fire();
  assert.equal(ticks, 1);

  fake.setHidden(true);
  assert.equal(ticks, 1, 'hiding does not tick');
  assert.deepEqual(
    [...fake.timers.values()].map((timer) => timer.ms),
    [HIDDEN_POLL_INTERVAL_MS]
  );

  fake.setHidden(false);
  assert.equal(ticks, 2, 'becoming visible refreshes immediately');
  assert.deepEqual(
    [...fake.timers.values()].map((timer) => timer.ms),
    [VISIBLE_POLL_INTERVAL_MS]
  );

  stop();
  assert.equal(fake.timers.size, 0);
  assert.equal(fake.listeners.size, 0);
});

test('D11: a visible-only poll stops while hidden and catches up when shown', () => {
  const fake = fakeEnvironment();
  let ticks = 0;
  const delays: boolean[] = [];
  const stop = startVisibleOnlyPoll(
    () => {
      ticks += 1;
    },
    (firstPoll) => {
      delays.push(firstPoll);
      return firstPoll ? 1_500 : 10_000;
    },
    fake.environment
  );
  assert.deepEqual(
    [...fake.timers.values()].map((timer) => timer.ms),
    [1_500],
    'the first poll uses the early delay'
  );
  fake.fire();
  assert.equal(ticks, 1);
  assert.deepEqual(
    [...fake.timers.values()].map((timer) => timer.ms),
    [10_000]
  );

  fake.setHidden(true);
  assert.equal(fake.timers.size, 0, 'no timer, so no IPC, while hidden');
  assert.equal(ticks, 1);

  fake.setHidden(false);
  assert.equal(ticks, 2, 'showing the window polls at once');
  assert.deepEqual(
    [...fake.timers.values()].map((timer) => timer.ms),
    [10_000]
  );
  assert.deepEqual(delays, [true, false, false]);

  stop();
  assert.equal(fake.timers.size, 0);
  assert.equal(fake.listeners.size, 0);
});

test('D11: a visible-only poll started while hidden waits for the window', () => {
  const fake = fakeEnvironment();
  fake.setHidden(true);
  let ticks = 0;
  const stop = startVisibleOnlyPoll(
    () => {
      ticks += 1;
    },
    () => 1_000,
    fake.environment
  );
  assert.equal(fake.timers.size, 0);
  fake.setHidden(false);
  assert.equal(ticks, 1);
  assert.equal(fake.timers.size, 1);
  stop();
});
