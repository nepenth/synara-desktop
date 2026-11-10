import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { ScreenSize, getScreenSize, getZoomedScreenSize } from '../useScreenSize';

test('breakpoints: Desktop above 1124px, Tablet 751-1124px, Mobile up to 750px', () => {
  assert.equal(getScreenSize(1125), ScreenSize.Desktop);
  assert.equal(getScreenSize(1124), ScreenSize.Tablet);
  assert.equal(getScreenSize(751), ScreenSize.Tablet);
  assert.equal(getScreenSize(750), ScreenSize.Mobile);
  assert.equal(getScreenSize(360), ScreenSize.Mobile);
});

test('page zoom counts: a window holds what it would hold at 100% divided by the zoom', () => {
  // 100%: unchanged.
  assert.equal(getZoomedScreenSize(1200, 1), ScreenSize.Desktop);
  // 150%: a 1200px window holds an 800px layout.
  assert.equal(getZoomedScreenSize(1200, 1.5), ScreenSize.Tablet);
  // 200%: a 1400px window is Mobile.
  assert.equal(getZoomedScreenSize(1400, 2), ScreenSize.Mobile);
  // 75%: a 900px window holds 1200px of layout.
  assert.equal(getZoomedScreenSize(900, 0.75), ScreenSize.Desktop);
  // A probe that could not measure falls back to the plain width.
  assert.equal(getZoomedScreenSize(1200, 0), ScreenSize.Desktop);
  assert.equal(getZoomedScreenSize(1200, Number.NaN), ScreenSize.Desktop);
});

test('the window is measured from its own body, and re-measured when only the zoom changes', () => {
  const source = readFileSync('src/app/hooks/useScreenSize.ts', 'utf8');
  assert.ok(source.includes('observer.observe(document.body)'));
  assert.ok(source.includes('observer.observe(probe)'));
  assert.ok(!source.includes('window.screen'));
  // Page zoom scales the root font size, which the 1rem probe follows.
  const zoom = readFileSync('src/app/pages/client/ClientNonUIFeatures.tsx', 'utf8');
  assert.ok(zoom.includes("setProperty('font-size'"));
});
