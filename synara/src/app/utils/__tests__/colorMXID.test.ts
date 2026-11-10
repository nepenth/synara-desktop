import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readFileSync } from 'node:fs';
import colorMXID, { cssColorMXID, mxidPaletteIndex } from '../../../util/colorMXID';

// Expected buckets come from the iOS algorithm (`SynaraAvatarPalette.hex(for:)`
// in synara-ios/Synara/SharedUI/SynaraDesign.swift), recomputed outside this
// code with arbitrary-precision integers (Python and Node BigInt agree):
//   h = 0xcbf29ce484222325
//   for b in seed.lower().encode('utf-8'): h = ((h ^ b) * 0x100000001b3) % 2**64
//   index = (h >> 32) % 8      # iOS hue = [212, 248, 285, 330, 8, 28, 140, 195][index]
const IOS_BUCKETS: [string, number][] = [
  ['', 4],
  ['@alice:example.org', 1],
  ['@Alice:Example.org', 1],
  ['@bob:example.org', 6],
  ['@机器人:example.org', 1],
  ['@carol:matrix.org', 7],
  ['@dave:synara.chat', 3],
  ['@erin:example.com', 5],
  ['@grace:localhost', 4],
  ['!room:example.org', 0],
];

test('Matrix ID colors pick the same palette slot as iOS (FNV-1a, lower-cased)', () => {
  for (const [userId, index] of IOS_BUCKETS) {
    assert.equal(mxidPaletteIndex(userId), index, userId);
    assert.equal(cssColorMXID(userId), `--mx-uc-${index + 1}`, userId);
    assert.equal(colorMXID(userId), `var(--mx-uc-${index + 1})`, userId);
  }
});

test('the --mx-uc slots carry the iOS hues in iOS order, light and dark', () => {
  const css = readFileSync('src/index.css', 'utf8');
  const hues = [212, 248, 285, 330, 8, 28, 140, 195];
  for (const selector of [':root {', '.dark-theme,\n.butter-theme {']) {
    const start = css.indexOf(selector);
    const block = css.slice(start, css.indexOf('}', start));
    hues.forEach((hue, i) => {
      assert.match(block, new RegExp(`--mx-uc-${i + 1}: hsl\\(${hue}, `), `${selector} ${i + 1}`);
    });
  }
});
