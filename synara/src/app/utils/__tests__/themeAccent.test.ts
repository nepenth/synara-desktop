import assert from 'node:assert/strict';
import test from 'node:test';
import { normalizeAccentColor } from '../themeAccent';

test('normalizeAccentColor accepts only full hex colors', () => {
  assert.equal(normalizeAccentColor('#AABBCC'), '#aabbcc');
  assert.equal(normalizeAccentColor(' #123456 '), '#123456');
  assert.equal(normalizeAccentColor('#abc'), undefined);
  assert.equal(normalizeAccentColor('red'), undefined);
  assert.equal(normalizeAccentColor('var(--x)'), undefined);
});
