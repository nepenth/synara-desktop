import assert from 'node:assert/strict';
import test from 'node:test';

import {
  HIDDEN_POLL_INTERVAL_MS,
  startVisibilityAwarePoll,
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
