import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  MESSAGE_TEXT_FOREGROUNDS,
  messageTextForeground,
  normalizeMessageTextTone,
} from '../messageTextTone';

const relativeLuminance = (hex: string): number => {
  const channels = hex
    .slice(1)
    .match(/.{2}/g)!
    .map((channel) => Number.parseInt(channel, 16) / 255)
    .map((channel) => (channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4));
  return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
};

const contrastRatio = (foreground: string, background: string): number => {
  const foregroundLuminance = relativeLuminance(foreground);
  const backgroundLuminance = relativeLuminance(background);
  const lighter = Math.max(foregroundLuminance, backgroundLuminance);
  const darker = Math.min(foregroundLuminance, backgroundLuminance);
  return (lighter + 0.05) / (darker + 0.05);
};

test('message text tone rejects unknown persisted values', () => {
  assert.equal(normalizeMessageTextTone('soft'), 'soft');
  assert.equal(normalizeMessageTextTone('balanced'), 'balanced');
  assert.equal(normalizeMessageTextTone('bright'), 'bright');
  assert.equal(normalizeMessageTextTone('white'), 'bright');
  assert.equal(normalizeMessageTextTone(undefined), 'bright');
});

test('message text tones are distinct and bright is true black or white', () => {
  assert.equal(messageTextForeground('bright', 'light'), '#000000');
  assert.equal(messageTextForeground('bright', 'dark'), '#ffffff');
  assert.equal(new Set(Object.values(MESSAGE_TEXT_FOREGROUNDS.light)).size, 3);
  assert.equal(new Set(Object.values(MESSAGE_TEXT_FOREGROUNDS.dark)).size, 3);
});

test('every selectable tone remains AAA-readable on supported message surfaces', () => {
  const surfaces = {
    light: ['#ffffff', '#f2f3f5', '#eceef1'],
    dark: ['#1e1f22', '#25272b', '#2d3036'],
  } as const;

  (['light', 'dark'] as const).forEach((appearance) => {
    Object.values(MESSAGE_TEXT_FOREGROUNDS[appearance]).forEach((foreground) => {
      surfaces[appearance].forEach((background) => {
        assert.ok(
          contrastRatio(foreground, background) >= 7,
          `${foreground} on ${background} must meet 7:1 contrast`
        );
      });
    });
  });
});

test('increased contrast does not collapse explicit message tone choices', () => {
  const globalCss = readFileSync('src/index.css', 'utf8');
  assert.doesNotMatch(
    globalCss,
    /@media \(prefers-contrast: more\)[\s\S]*--synara-message-foreground:.*!important/
  );
});

test('message renderers and composer consume the same foreground', () => {
  const nativeCss = readFileSync('src/app/features/room/nativeTimelineHtml.css.ts', 'utf8');
  const compatibilityCss = readFileSync('src/app/components/message/layout/layout.css.ts', 'utf8');
  const composerCss = readFileSync('src/app/components/editor/Editor.css.ts', 'utf8');
  assert.match(nativeCss, /color: 'var\(--synara-message-foreground\)'/);
  assert.match(compatibilityCss, /color: 'var\(--synara-message-foreground\)'/);
  assert.match(composerCss, /color: 'var\(--synara-message-foreground\)'/);
});
