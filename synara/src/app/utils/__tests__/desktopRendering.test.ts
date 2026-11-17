import test from 'node:test';
import assert from 'node:assert/strict';
import {
  DESKTOP_RENDERING_UNAVAILABLE,
  type DesktopRenderingStatus,
  hardwareAccelerationFallbackNote,
  hardwareAccelerationView,
  normalizeDesktopRenderingStatus,
} from '../desktopRendering';

const status = (fields: Partial<DesktopRenderingStatus>): DesktopRenderingStatus => ({
  available: true,
  saved: null,
  active: 'safe',
  overriddenByEnvironment: false,
  fallback: null,
  ...fields,
});

test('a well-formed answer from the shell is kept', () => {
  assert.deepEqual(
    normalizeDesktopRenderingStatus({
      available: true,
      saved: 'gpu',
      active: 'safe',
      overriddenByEnvironment: false,
      fallback: null,
    }),
    status({ saved: 'gpu', active: 'safe' })
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
    assert.deepEqual(
      normalizeDesktopRenderingStatus({ available: true, saved, active: 'gpu' }),
      status({ active: 'gpu' })
    );
  }
});

test('with nothing saved the switch shows what is running', () => {
  assert.deepEqual(hardwareAccelerationView(status({ active: 'gpu' })), {
    mode: 'gpu',
    locked: false,
    restartNeeded: false,
    fallback: null,
  });
  assert.deepEqual(hardwareAccelerationView(status({ active: 'safe' })), {
    mode: 'safe',
    locked: false,
    restartNeeded: false,
    fallback: null,
  });
});

test('an unknown mode reads as safe', () => {
  assert.deepEqual(
    normalizeDesktopRenderingStatus({ available: true, saved: 'fast', active: 7 }),
    status({ saved: 'safe', active: 'safe' })
  );
});

test('the switch shows the saved choice and asks for a restart when it differs', () => {
  assert.deepEqual(hardwareAccelerationView(status({ saved: 'gpu', active: 'safe' })), {
    mode: 'gpu',
    locked: false,
    restartNeeded: true,
    fallback: null,
  });
  assert.deepEqual(hardwareAccelerationView(status({ saved: 'safe', active: 'gpu' })), {
    mode: 'safe',
    locked: false,
    restartNeeded: true,
    fallback: null,
  });
  assert.deepEqual(hardwareAccelerationView(status({ saved: 'gpu', active: 'gpu' })), {
    mode: 'gpu',
    locked: false,
    restartNeeded: false,
    fallback: null,
  });
});

test('the environment variable locks the switch to what is running', () => {
  assert.deepEqual(
    hardwareAccelerationView(
      status({ saved: 'safe', active: 'gpu', overriddenByEnvironment: true })
    ),
    { mode: 'gpu', locked: true, restartNeeded: false, fallback: null }
  );
  assert.deepEqual(
    hardwareAccelerationView(
      status({
        saved: 'gpu',
        active: 'safe',
        overriddenByEnvironment: true,
        fallback: { mode: 'gpu', reason: 'web-view-crashed' },
      })
    ),
    { mode: 'safe', locked: true, restartNeeded: false, fallback: null }
  );
});

test('balanced is kept as its own mode', () => {
  assert.deepEqual(
    normalizeDesktopRenderingStatus({ available: true, saved: 'balanced', active: 'balanced' }),
    status({ saved: 'balanced', active: 'balanced' })
  );
  assert.deepEqual(hardwareAccelerationView(status({ saved: 'balanced', active: 'safe' })), {
    mode: 'balanced',
    locked: false,
    restartNeeded: true,
    fallback: null,
  });
});

test('a fallback from the shell is kept', () => {
  for (const fallback of [
    { mode: 'balanced', reason: 'web-view-crashed' },
    { mode: 'gpu', reason: 'start-failed' },
  ] as const) {
    assert.deepEqual(
      normalizeDesktopRenderingStatus({
        available: true,
        saved: 'balanced',
        active: 'safe',
        overriddenByEnvironment: false,
        fallback,
      }),
      status({ saved: 'balanced', active: 'safe', fallback })
    );
  }
});

test('a missing or malformed fallback is normalised', () => {
  for (const fallback of [null, undefined, 'gpu', 3]) {
    assert.equal(
      normalizeDesktopRenderingStatus({ available: true, active: 'safe', fallback }).fallback,
      null
    );
  }
  assert.deepEqual(
    normalizeDesktopRenderingStatus({
      available: true,
      active: 'safe',
      fallback: { mode: 'fast', reason: 'bored' },
    }).fallback,
    { mode: 'safe', reason: 'start-failed' }
  );
});

test('after a fallback the switch shows Off with no restart needed', () => {
  const fallback = { mode: 'balanced', reason: 'web-view-crashed' } as const;
  for (const saved of [null, 'balanced', 'gpu'] as const) {
    assert.deepEqual(hardwareAccelerationView(status({ saved, active: 'safe', fallback })), {
      mode: 'safe',
      locked: false,
      restartNeeded: false,
      fallback,
    });
  }
});

test('the fallback note names the mode that failed and how', () => {
  assert.equal(
    hardwareAccelerationFallbackNote({ mode: 'balanced', reason: 'web-view-crashed' }),
    'Synara switched to Off because the display crashed in Balanced mode. Choose a mode to try again.'
  );
  assert.equal(
    hardwareAccelerationFallbackNote({ mode: 'gpu', reason: 'start-failed' }),
    'Synara switched to Off because it closed while starting in Full mode. Choose a mode to try again.'
  );
});
