import assert from 'node:assert/strict';
import test from 'node:test';
import {
  DEFAULT_THEME_BASE_COLOR,
  deriveThemeSurfaceRamp,
  normalizeThemeBaseColor,
  resolveThemeBaseColor,
  themeContrastRatio,
  themeSurfaceLuminance,
} from '../themeBase';

test('normalizeThemeBaseColor accepts only full hex colors', () => {
  assert.equal(normalizeThemeBaseColor('#AABBCC'), '#aabbcc');
  assert.equal(normalizeThemeBaseColor(' #2b2d31 '), '#2b2d31');
  assert.equal(normalizeThemeBaseColor('#abc'), undefined);
  assert.equal(normalizeThemeBaseColor('red'), undefined);
  assert.equal(normalizeThemeBaseColor(undefined), undefined);
});

test('resolveThemeBaseColor falls back to the Discord-like charcoal default', () => {
  assert.equal(resolveThemeBaseColor(undefined), DEFAULT_THEME_BASE_COLOR);
  assert.equal(resolveThemeBaseColor('#FF0000'), '#ff0000');
});

test('default dark ramp stacks sidebar darker than chat', () => {
  const ramp = deriveThemeSurfaceRamp(DEFAULT_THEME_BASE_COLOR, 'dark');

  assert.ok(
    themeSurfaceLuminance(ramp.background.Container) < themeSurfaceLuminance(ramp.surface.Container)
  );
  assert.ok(
    themeSurfaceLuminance(ramp.surface.Container) <
      themeSurfaceLuminance(ramp.surfaceVariant.Container)
  );
  assert.ok(themeSurfaceLuminance(ramp.background.Container) < 0.03);
  assert.ok(themeContrastRatio(ramp.background.OnContainer, ramp.background.Container) >= 7);
  assert.ok(
    themeContrastRatio(ramp.surfaceVariant.OnContainer, ramp.surfaceVariant.Container) >= 7
  );
});

test('default light ramp is a real light theme with a darker sidebar', () => {
  const ramp = deriveThemeSurfaceRamp(DEFAULT_THEME_BASE_COLOR, 'light');

  assert.ok(
    themeSurfaceLuminance(ramp.background.Container) < themeSurfaceLuminance(ramp.surface.Container)
  );
  assert.ok(
    themeSurfaceLuminance(ramp.surface.Container) <
      themeSurfaceLuminance(ramp.surfaceVariant.Container)
  );
  assert.ok(themeSurfaceLuminance(ramp.surfaceVariant.Container) > 0.9);
  assert.ok(themeSurfaceLuminance(ramp.background.OnContainer) < 0.08);
  assert.ok(
    themeContrastRatio(ramp.surfaceVariant.OnContainer, ramp.surfaceVariant.Container) >= 7
  );
});

test('a saturated base color tints both ramps without inverting stacked lightness', () => {
  const dark = deriveThemeSurfaceRamp('#5865f2', 'dark');
  const light = deriveThemeSurfaceRamp('#5865f2', 'light');
  const defaultDark = deriveThemeSurfaceRamp(DEFAULT_THEME_BASE_COLOR, 'dark');

  assert.notEqual(dark.surface.Container, defaultDark.surface.Container);
  assert.ok(
    themeSurfaceLuminance(dark.background.Container) <
      themeSurfaceLuminance(dark.surfaceVariant.Container)
  );
  assert.ok(
    themeSurfaceLuminance(light.background.Container) <
      themeSurfaceLuminance(light.surfaceVariant.Container)
  );
  assert.ok(themeContrastRatio(dark.surface.OnContainer, dark.surface.Container) >= 4.5);
  assert.ok(themeContrastRatio(light.surface.OnContainer, light.surface.Container) >= 4.5);
});
