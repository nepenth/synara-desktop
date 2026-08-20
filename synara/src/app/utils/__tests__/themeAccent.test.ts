import assert from 'node:assert/strict';
import test from 'node:test';
import {
  DARK_THEME_ACCENT_COLOR,
  DEFAULT_ACCENT_COLOR,
  normalizeAccentColor,
  themeDefaultAccentColor,
} from '../themeAccent';

test('normalizeAccentColor accepts only full hex colors', () => {
  assert.equal(normalizeAccentColor('#AABBCC'), '#aabbcc');
  assert.equal(normalizeAccentColor(' #123456 '), '#123456');
  assert.equal(normalizeAccentColor('#abc'), undefined);
  assert.equal(normalizeAccentColor('red'), undefined);
  assert.equal(normalizeAccentColor('var(--x)'), undefined);
});

test('unset accent falls back to the theme primary actually in use, not mint', () => {
  assert.equal(DEFAULT_ACCENT_COLOR, '#1245a8');
  assert.equal(DARK_THEME_ACCENT_COLOR, '#bdb6ec');
  assert.equal(themeDefaultAccentColor('light'), DEFAULT_ACCENT_COLOR);
  assert.equal(themeDefaultAccentColor('dark'), DARK_THEME_ACCENT_COLOR);
  assert.notEqual(DEFAULT_ACCENT_COLOR, '#6bdbb8');
});
