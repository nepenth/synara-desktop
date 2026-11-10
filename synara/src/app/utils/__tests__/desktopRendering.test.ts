import test from 'node:test';
import assert from 'node:assert/strict';
import {
  DESKTOP_RENDERING_UNAVAILABLE,
  hardwareAccelerationView,
  normalizeDesktopRenderingStatus,
} from '../desktopRendering';

test('a well-formed answer from the shell is kept', () => {
  assert.deepEqual(
    normalizeDesktopRenderingStatus({
      available: true,
      saved: 'gpu',
      active: 'safe',
      overriddenByEnvironment: false,
    }),
    { available: true, saved: 'gpu', active: 'safe', overriddenByEnvironment: false }
  );
});

test('anything else from the shell means the setting is not available', () => {
  for (const value of [
    undefined,
    null,
    'gpu',
    {},
    { available: false, saved: 'gpu', active: 'gpu', overriddenByEnvironment: true },
    { available: 'yes', saved: 'gpu', active: 'gpu' },
  ]) {
    assert.deepEqual(normalizeDesktopRenderingStatus(value), DESKTOP_RENDERING_UNAVAILABLE);
  }
});

test('no saved choice stays null', () => {
  for (const saved of [null, undefined]) {
    assert.deepEqual(normalizeDesktopRenderingStatus({ available: true, saved, active: 'gpu' }), {
      available: true,
      saved: null,
      active: 'gpu',
      overriddenByEnvironment: false,
    });
  }
});

test('with nothing saved the switch shows what is running', () => {
  assert.deepEqual(
    hardwareAccelerationView({
      available: true,
      saved: null,
      active: 'gpu',
      overriddenByEnvironment: false,
    }),
    { enabled: true, locked: false, restartNeeded: false }
  );
  assert.deepEqual(
    hardwareAccelerationView({
      available: true,
      saved: null,
      active: 'safe',
      overriddenByEnvironment: false,
    }),
    { enabled: false, locked: false, restartNeeded: false }
  );
});

test('an unknown mode reads as safe', () => {
  assert.deepEqual(normalizeDesktopRenderingStatus({ available: true, saved: 'fast', active: 7 }), {
    available: true,
    saved: 'safe',
    active: 'safe',
    overriddenByEnvironment: false,
  });
});

test('the switch shows the saved choice and asks for a restart when it differs', () => {
  assert.deepEqual(
    hardwareAccelerationView({
      available: true,
      saved: 'gpu',
      active: 'safe',
      overriddenByEnvironment: false,
    }),
    { enabled: true, locked: false, restartNeeded: true }
  );
  assert.deepEqual(
    hardwareAccelerationView({
      available: true,
      saved: 'safe',
      active: 'gpu',
      overriddenByEnvironment: false,
    }),
    { enabled: false, locked: false, restartNeeded: true }
  );
  assert.deepEqual(
    hardwareAccelerationView({
      available: true,
      saved: 'gpu',
      active: 'gpu',
      overriddenByEnvironment: false,
    }),
    { enabled: true, locked: false, restartNeeded: false }
  );
});

test('the environment variable locks the switch to what is running', () => {
  assert.deepEqual(
    hardwareAccelerationView({
      available: true,
      saved: 'safe',
      active: 'gpu',
      overriddenByEnvironment: true,
    }),
    { enabled: true, locked: true, restartNeeded: false }
  );
  assert.deepEqual(
    hardwareAccelerationView({
      available: true,
      saved: 'gpu',
      active: 'safe',
      overriddenByEnvironment: true,
    }),
    { enabled: false, locked: true, restartNeeded: false }
  );
});
