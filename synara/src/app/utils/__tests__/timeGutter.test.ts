import assert from 'node:assert/strict';
import test from 'node:test';
import { GUTTER_TIME_MAX_CHARS, timeFull, timeGutter, timeHourMinute } from '../time';

const at = (hours: number, minutes: number) => new Date(2026, 10, 18, hours, minutes).getTime();

test('the gutter time is the short clock, never wider than the gutter is sized for', () => {
  assert.equal(timeGutter(at(20, 22), false), '8:22');
  assert.equal(timeGutter(at(8, 5), false), '8:05');
  assert.equal(timeGutter(at(0, 0), false), '12:00');
  assert.equal(timeGutter(at(20, 22), true), '20:22');
  assert.equal(timeGutter(at(8, 5), true), '08:05');

  for (let hours = 0; hours < 24; hours += 1) {
    for (const hour24Clock of [false, true]) {
      const time = timeGutter(at(hours, 59), hour24Clock);
      assert.ok(time.length <= GUTTER_TIME_MAX_CHARS, `${time} is wider than the gutter`);
      assert.doesNotMatch(time, /AM|PM/);
    }
  }
});

test('the full time names the weekday, the chosen date format and the clock', () => {
  assert.equal(timeFull(at(12, 59), false, 'D MMM YYYY'), 'Wednesday, 18 Nov 2026 12:59 PM');
  assert.equal(timeFull(at(20, 22), true, 'YYYY-MM-DD'), 'Wednesday, 2026-11-18 20:22');
});

test('the 12-hour clock time is not zero-padded', () => {
  assert.equal(timeHourMinute(at(14, 3), false), '2:03 PM');
  assert.equal(timeHourMinute(at(8, 22), false), '8:22 AM');
  assert.equal(timeHourMinute(at(0, 5), false), '12:05 AM');
  assert.equal(timeHourMinute(at(14, 3), true), '14:03');
  assert.equal(timeHourMinute(at(8, 22), true), '08:22');
});
