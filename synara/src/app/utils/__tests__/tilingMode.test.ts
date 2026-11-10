import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { normalizeDesktopWindowEnvironment } from '../desktop';
import { resolveTilingMode } from '../tilingMode';
import { normalizeTilingModeSetting } from '../../state/settings';

test('an absent or unknown window environment is floating', () => {
  assert.equal(normalizeDesktopWindowEnvironment('tiling'), 'tiling');
  assert.equal(normalizeDesktopWindowEnvironment('floating'), 'floating');
  assert.equal(normalizeDesktopWindowEnvironment(undefined), 'floating');
  assert.equal(normalizeDesktopWindowEnvironment('Tiling'), 'floating');
  assert.equal(normalizeDesktopWindowEnvironment(true), 'floating');
});

test('Auto follows the shell; On and Off override it; only Linux has tiling mode', () => {
  assert.equal(resolveTilingMode('auto', 'tiling', true), true);
  assert.equal(resolveTilingMode('auto', 'floating', true), false);
  assert.equal(resolveTilingMode('on', 'floating', true), true);
  assert.equal(resolveTilingMode('off', 'tiling', true), false);
  for (const setting of ['auto', 'on', 'off'] as const) {
    assert.equal(resolveTilingMode(setting, 'tiling', false), false, setting);
  }
  assert.equal(normalizeTilingModeSetting('on'), 'on');
  assert.equal(normalizeTilingModeSetting('yes'), 'auto');
});

test('the bridge defaults to floating until the shell says otherwise', () => {
  const bridge = readFileSync('../src-tauri/src/desktop_bridge.js', 'utf8');
  assert.match(bridge, /windowEnvironment: "floating"/);
});
