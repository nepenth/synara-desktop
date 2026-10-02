import assert from 'node:assert/strict';
import { test } from 'node:test';
import colorMXID, { cssColorMXID } from '../../../util/colorMXID';

test('Matrix ID presentation colors keep the established palette assignments', () => {
  for (const [userId, variable] of [
    ['', '--mx-uc-1'],
    ['@alice:example.org', '--mx-uc-7'],
    ['@bob:example.org', '--mx-uc-4'],
    ['@机器人:example.org', '--mx-uc-7'],
  ]) {
    assert.equal(cssColorMXID(userId), variable, userId);
    assert.equal(colorMXID(userId), `var(${variable})`, userId);
  }
});
