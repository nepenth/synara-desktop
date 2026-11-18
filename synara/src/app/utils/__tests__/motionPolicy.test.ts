import assert from 'node:assert/strict';
import test from 'node:test';
import {
  MOTION_ALLOWED_QUERY,
  SAFE_RENDER_SELECTOR,
  WINDOW_INACTIVE_SELECTOR,
  continuousMotion,
  renderModeAttribute,
  windowActiveAttribute,
  writeRootAttribute,
} from '../motion';

test('only a visible, focused window is active', () => {
  assert.equal(windowActiveAttribute({ visibilityState: 'visible', hasFocus: true }), 'true');
  assert.equal(windowActiveAttribute({ visibilityState: 'visible', hasFocus: false }), 'false');
  assert.equal(windowActiveAttribute({ visibilityState: 'hidden', hasFocus: true }), 'false');
  assert.equal(windowActiveAttribute({ visibilityState: 'hidden', hasFocus: false }), 'false');
});

test('the render mode attribute is the running mode, and absent where there is no setting', () => {
  assert.equal(renderModeAttribute({ available: true, active: 'safe' }), 'safe');
  assert.equal(renderModeAttribute({ available: true, active: 'gpu' }), 'gpu');
  // The "not available" status reports `active: 'safe'`; that must not
  // step animations on macOS, Windows or X11.
  assert.equal(renderModeAttribute({ available: false, active: 'safe' }), undefined);
});

test('root attributes are written only when they change, and removed for undefined', () => {
  const attributes = new Map<string, string>();
  const writes: string[] = [];
  const root = {
    getAttribute: (name: string) => attributes.get(name) ?? null,
    setAttribute: (name: string, value: string) => {
      writes.push(`set ${name}=${value}`);
      attributes.set(name, value);
    },
    removeAttribute: (name: string) => {
      writes.push(`remove ${name}`);
      attributes.delete(name);
    },
  };

  writeRootAttribute(root, 'data-window-active', 'true');
  writeRootAttribute(root, 'data-window-active', 'true');
  writeRootAttribute(root, 'data-window-active', 'false');
  writeRootAttribute(root, 'data-render-mode', undefined);
  writeRootAttribute(root, 'data-window-active', undefined);

  assert.deepEqual(writes, [
    'set data-window-active=true',
    'set data-window-active=false',
    'remove data-window-active',
  ]);
});

test('continuousMotion keeps the rule and its reduced-motion form, and adds pause and safe steps', () => {
  const reduced = { animation: 'none' };
  const rule = continuousMotion(
    {
      animation: 'spin 1s infinite',
      selectors: { '&:hover': { opacity: 0.5 } },
      '@media': { '(prefers-reduced-motion: reduce)': reduced },
    },
    { animationTimingFunction: 'steps(4)' }
  );

  assert.equal(rule.animation, 'spin 1s infinite');
  assert.deepEqual(rule.selectors, {
    '&:hover': { opacity: 0.5 },
    [`${WINDOW_INACTIVE_SELECTOR} &`]: { animationPlayState: 'paused' },
  });
  assert.equal(rule['@media']?.['(prefers-reduced-motion: reduce)'], reduced);
  // The stepped form never reaches someone who asked for reduced motion.
  assert.deepEqual(rule['@media']?.[MOTION_ALLOWED_QUERY], {
    selectors: { [`${SAFE_RENDER_SELECTOR} &`]: { animationTimingFunction: 'steps(4)' } },
  });
  assert.equal(WINDOW_INACTIVE_SELECTOR, ':root[data-window-active="false"]');
  assert.equal(SAFE_RENDER_SELECTOR, ':root[data-render-mode="safe"]');
});
