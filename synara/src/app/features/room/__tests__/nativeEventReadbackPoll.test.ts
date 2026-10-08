import assert from 'node:assert/strict';
import test from 'node:test';

import {
  EVENT_READBACK_INITIAL_DELAY_MS,
  EVENT_READBACK_MAX_DELAY_MS,
  nextEventReadbackDelayMs,
} from '../message/nativeEventReadbackPoll';

test('pending readbacks back off exponentially to a cap', () => {
  const delays: number[] = [];
  let delay: number | undefined;
  for (let attempt = 0; attempt < 8; attempt += 1) {
    delay = nextEventReadbackDelayMs('pending', delay);
    assert.notEqual(delay, undefined);
    delays.push(delay as number);
  }
  assert.equal(delays[0], EVENT_READBACK_INITIAL_DELAY_MS);
  assert.deepEqual(delays.slice(0, 6), [1_000, 2_000, 4_000, 8_000, 16_000, 30_000]);
  assert.equal(delays[7], EVENT_READBACK_MAX_DELAY_MS);
});

test('unavailable and decrypted readbacks stop polling', () => {
  assert.equal(nextEventReadbackDelayMs('unavailable', 4_000), undefined);
  assert.equal(nextEventReadbackDelayMs('decrypted', undefined), undefined);
});
