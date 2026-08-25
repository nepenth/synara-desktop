import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';
import { roomAvatarTone } from '../common';
import {
  chromeColorsForRamp,
  DEFAULT_THEME_BASE_COLOR,
  deriveThemeSurfaceRamp,
  normalizeThemeBaseColor,
  resolveThemeBaseColor,
  shouldApplyDerivedThemeRamp,
  THEME_BASE_PRESETS,
  THEME_CHROME_ROLES,
  themeChromeAssignments,
  themeContrastRatio,
  themeSurfaceLuminance,
} from '../themeBase';

const srcRoot = join(process.cwd(), 'src');

test('normalizeThemeBaseColor accepts only full hash-prefixed hex colors', () => {
  assert.equal(normalizeThemeBaseColor('#AABBCC'), '#aabbcc');
  assert.equal(normalizeThemeBaseColor(' #2b2d31 '), '#2b2d31');
  assert.equal(normalizeThemeBaseColor('#abc'), undefined);
  assert.equal(normalizeThemeBaseColor('aabbcc'), undefined);
  assert.equal(normalizeThemeBaseColor('red'), undefined);
  assert.equal(normalizeThemeBaseColor(undefined), undefined);
});

test('resolveThemeBaseColor falls back to the Discord-like charcoal default', () => {
  assert.equal(resolveThemeBaseColor(undefined), DEFAULT_THEME_BASE_COLOR);
  assert.equal(resolveThemeBaseColor('#FF0000'), '#ff0000');
});

test('derived ramps apply only to the light and dark Discord-like themes', () => {
  assert.equal(shouldApplyDerivedThemeRamp('light-theme'), true);
  assert.equal(shouldApplyDerivedThemeRamp('dark-theme'), true);
  assert.equal(shouldApplyDerivedThemeRamp(undefined), true);
  assert.equal(shouldApplyDerivedThemeRamp('butter-theme'), false);
  assert.equal(shouldApplyDerivedThemeRamp('silver-theme'), false);
});

test('chrome roles map rail / room list / chat / composer to distinct folds stops', () => {
  assert.equal(THEME_CHROME_ROLES.rail, 'background');
  assert.equal(THEME_CHROME_ROLES.roomList, 'surface');
  assert.equal(THEME_CHROME_ROLES.chat, 'surfaceVariant');
  assert.equal(THEME_CHROME_ROLES.composer, 'secondaryContainer');

  const ramp = deriveThemeSurfaceRamp(DEFAULT_THEME_BASE_COLOR, 'dark');
  const assignments = themeChromeAssignments(ramp);
  const chrome = chromeColorsForRamp(ramp);

  assert.equal(assignments.background.Container, chrome.rail);
  assert.equal(assignments.surface.Container, chrome.roomList);
  assert.equal(assignments.surfaceVariant.Container, chrome.chat);
  assert.equal(assignments.secondaryContainer.Container, chrome.composer);
  assert.notEqual(chrome.rail, chrome.roomList);
  assert.notEqual(chrome.roomList, chrome.chat);
  assert.notEqual(chrome.chat, chrome.composer);
});

test('default dark ramp stacks rail darker than room list darker than chat', () => {
  const chrome = chromeColorsForRamp(deriveThemeSurfaceRamp(DEFAULT_THEME_BASE_COLOR, 'dark'));

  assert.ok(themeSurfaceLuminance(chrome.rail) < themeSurfaceLuminance(chrome.roomList));
  assert.ok(themeSurfaceLuminance(chrome.roomList) < themeSurfaceLuminance(chrome.chat));
  assert.ok(themeSurfaceLuminance(chrome.rail) < 0.04);
  assert.ok(themeContrastRatio('#f2f2f2', chrome.chat) >= 7);
});

test('default light ramp is a real light theme with a darker sidebar', () => {
  const ramp = deriveThemeSurfaceRamp(DEFAULT_THEME_BASE_COLOR, 'light');
  const chrome = chromeColorsForRamp(ramp);

  assert.ok(themeSurfaceLuminance(chrome.rail) < themeSurfaceLuminance(chrome.roomList));
  assert.ok(themeSurfaceLuminance(chrome.roomList) < themeSurfaceLuminance(chrome.chat));
  assert.ok(themeSurfaceLuminance(chrome.chat) > 0.8);
  assert.ok(themeContrastRatio(ramp.surfaceVariant.OnContainer, chrome.chat) >= 7);
  assert.ok(themeContrastRatio('#666666', chrome.chat) >= 4.5);
});

test('white black and saturated bases produce user-visible ramp differences', () => {
  const charcoal = chromeColorsForRamp(deriveThemeSurfaceRamp(DEFAULT_THEME_BASE_COLOR, 'dark'));
  const white = chromeColorsForRamp(deriveThemeSurfaceRamp('#ffffff', 'dark'));
  const black = chromeColorsForRamp(deriveThemeSurfaceRamp('#000000', 'dark'));
  const blurple = chromeColorsForRamp(deriveThemeSurfaceRamp('#5865f2', 'dark'));

  assert.notEqual(white.roomList, black.roomList);
  assert.notEqual(white.roomList, charcoal.roomList);
  assert.notEqual(black.roomList, charcoal.roomList);
  assert.notEqual(blurple.roomList, charcoal.roomList);
  assert.ok(themeSurfaceLuminance(white.rail) < themeSurfaceLuminance(white.chat));
  assert.ok(themeSurfaceLuminance(black.rail) < themeSurfaceLuminance(black.chat));
  assert.ok(themeContrastRatio('#f2f2f2', blurple.chat) >= 4.5);
});

test('semantic content roles keep calm hierarchy and accessible floors across every preset', () => {
  for (const preset of THEME_BASE_PRESETS) {
    for (const kind of ['light', 'dark'] as const) {
      const ramp = deriveThemeSurfaceRamp(preset.hex, kind);
      const { content } = ramp;

      assert.ok(themeContrastRatio(content.heading, ramp.surfaceVariant.Container) >= 12);
      assert.ok(themeContrastRatio(content.primary, ramp.surfaceVariant.Container) >= 8);
      assert.ok(themeContrastRatio(content.secondary, ramp.surfaceVariant.Container) >= 5.5);
      assert.ok(themeContrastRatio(content.tertiary, ramp.surfaceVariant.Container) >= 4.5);
      for (const surface of [
        ramp.background.Container,
        ramp.surface.Container,
        ramp.surfaceVariant.Container,
        ramp.secondaryContainer.Container,
      ]) {
        assert.ok(themeContrastRatio(content.primary, surface) >= 4.5);
      }
      assert.ok(themeContrastRatio(content.primary, content.tableHeader) >= 4.5);
      assert.ok(themeContrastRatio(content.primary, content.tableOdd) >= 4.5);
      assert.ok(themeContrastRatio(content.primary, content.tableEven) >= 4.5);
      assert.notEqual(content.tableHeader, content.tableOdd);
      assert.notEqual(content.tableOdd, content.tableEven);
    }
  }
});

test('desktop chrome source files consume the stacked stops', () => {
  const page = readFileSync(join(srcRoot, 'app/components/page/Page.tsx'), 'utf8');
  const editor = readFileSync(join(srcRoot, 'app/components/editor/Editor.css.ts'), 'utf8');
  const roomNav = readFileSync(join(srcRoot, 'app/features/room-nav/RoomNavItem.tsx'), 'utf8');
  const sidebar = readFileSync(join(srcRoot, 'app/components/sidebar/Sidebar.css.ts'), 'utf8');

  assert.match(page, /PageNavHeader[\s\S]*variant="Surface"/);
  assert.match(page, /Scroll[\s\S]*variant="Surface"/);
  assert.match(page, /ContainerColor\(\{ variant: 'SurfaceVariant' \}\)/);
  assert.match(editor, /color\.Secondary\.Container/);
  assert.match(roomNav, /variant="Surface"/);
  assert.match(sidebar, /color\.Background\.Container/);
});

test('non-derived themes keep their own surface-matched content roles', () => {
  const manager = readFileSync(join(srcRoot, 'app/pages/ThemeManager.tsx'), 'utf8');
  assert.match(
    manager,
    /if \(!shouldApplyDerivedThemeRamp\(themeId\)\) \{[\s\S]*clearContentRoles\(target\)[\s\S]*return themeKind[\s\S]*\}\n{2} {2}syncContentRoles\(target, ramp\.content\)/
  );
});

test('desktop room navigation uses stable channel text instead of decorative room tiles', () => {
  const roomNav = readFileSync(join(srcRoot, 'app/features/room-nav/RoomNavItem.tsx'), 'utf8');
  const roomHeader = readFileSync(join(srcRoot, 'app/features/room/RoomViewHeader.tsx'), 'utf8');

  assert.match(roomNav, /className=\{css\.RoomGlyph\}[\s\S]{0,80}#/);
  assert.doesNotMatch(roomNav, /RoomAvatar/);
  assert.match(roomHeader, /className=\{css\.RoomChannelGlyph\}[\s\S]{0,80}#/);
  assert.doesNotMatch(roomHeader, /RoomAvatar/);
});

test('roomAvatarTone stays independent of the theme base hue', () => {
  const avatar = roomAvatarTone('!room:example.org');
  const blurple = chromeColorsForRamp(deriveThemeSurfaceRamp('#5865f2', 'dark'));

  assert.match(avatar.background, /^#[0-9a-f]{6}$/i);
  assert.notEqual(avatar.background, blurple.roomList);
  assert.notEqual(avatar.background, blurple.chat);
  assert.equal(roomAvatarTone('!room:example.org').background, avatar.background);
});
