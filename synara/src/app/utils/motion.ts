import type { StyleRule } from '@vanilla-extract/css';
import type { DesktopRenderingStatus } from './desktopRendering';

/** The user asked the system to reduce motion (WCAG 2.3.3). */
export const prefersReducedMotion = (): boolean =>
  typeof window !== 'undefined' &&
  typeof window.matchMedia === 'function' &&
  window.matchMedia('(prefers-reduced-motion: reduce)').matches;

/** Smooth scrolling, unless reduced motion is on: then jump. */
export const preferredScrollBehavior = (): 'auto' | 'smooth' =>
  prefersReducedMotion() ? 'auto' : 'smooth';

/*
 * Motion budget for continuous (infinite) animations.
 *
 * An infinite CSS animation repaints the page at the display rate for as long
 * as it is on screen. In Linux "safe" rendering WebKit paints every frame on
 * the CPU, on the same main thread that handles keys. So every infinite
 * animation goes through one policy, driven by two attributes on <html> that
 * `useMotionPolicy` keeps up to date:
 *
 * - `data-window-active="false"`: the window is unfocused or hidden. Infinite
 *   animations pause and pick up where they stopped on focus.
 * - `data-render-mode="safe"`: frames are painted on the CPU. Infinite
 *   animations switch to a `steps()` form that changes a few times a second.
 *   WebKit schedules the next animation update at the next step boundary
 *   when every keyframe interval uses `steps()` (AnimationEffect /
 *   KeyframeEffect `timeToNextTick` → `progressUntilNextStep`), so a stepped
 *   animation costs a few paints a second instead of one per frame. Keyframes
 *   used in safe mode must not carry their own (non-steps) timing function.
 *
 * `prefers-reduced-motion: reduce` stays the strongest setting: whatever a
 * component does under it is left alone, and the stepped form only applies
 * when the user has no motion preference.
 */

export const WINDOW_ACTIVE_ATTRIBUTE = 'data-window-active';
export const RENDER_MODE_ATTRIBUTE = 'data-render-mode';

export const WINDOW_INACTIVE_SELECTOR = `:root[${WINDOW_ACTIVE_ATTRIBUTE}="false"]`;
export const SAFE_RENDER_SELECTOR = `:root[${RENDER_MODE_ATTRIBUTE}="safe"]`;
export const MOTION_ALLOWED_QUERY = '(prefers-reduced-motion: no-preference)';

/**
 * folds does not export the class names of its two infinite animations, the
 * Spinner's rotating svg and the indeterminate ProgressBar packet. A test
 * checks these against folds' stylesheet so a folds upgrade cannot drop them
 * silently.
 */
export const FOLDS_SPINNER_SVG_CLASS = '_31czpkn';
export const FOLDS_PROGRESS_PACKET_CLASS = 'w2djj9p';

export type WindowActivityInput = {
  visibilityState: DocumentVisibilityState;
  hasFocus: boolean;
};

/** "true" only for a visible, focused window. */
export const windowActiveAttribute = ({
  visibilityState,
  hasFocus,
}: WindowActivityInput): 'true' | 'false' =>
  visibilityState === 'visible' && hasFocus ? 'true' : 'false';

/** The running rendering mode, or `undefined` where the shell has no such setting. */
export const renderModeAttribute = (
  status: Pick<DesktopRenderingStatus, 'available' | 'active'>
): DesktopRenderingStatus['active'] | undefined => (status.available ? status.active : undefined);

/** Sets the attribute, or removes it for `undefined`; skips no-op writes. */
export const writeRootAttribute = (
  root: Pick<Element, 'getAttribute' | 'setAttribute' | 'removeAttribute'>,
  name: string,
  value: string | undefined
): void => {
  if (value === undefined) {
    if (root.getAttribute(name) !== null) root.removeAttribute(name);
    return;
  }
  if (root.getAttribute(name) !== value) root.setAttribute(name, value);
};

/** Longhands only, so a component's own `animation-delay` and the pause survive. */
export type SafeRenderAnimation = Pick<
  StyleRule,
  'animationName' | 'animationDuration' | 'animationTimingFunction' | 'animationDirection'
>;

/**
 * Wraps the style rule of an element that runs an infinite animation: pauses
 * it while the window is inactive and swaps in `safe` (a `steps()` form)
 * in safe rendering mode. The rule's own look and its reduced-motion handling
 * are kept as they are.
 */
export const continuousMotion = (rule: StyleRule, safe: SafeRenderAnimation): StyleRule => {
  const media = rule['@media'] ?? {};
  const allowed = media[MOTION_ALLOWED_QUERY] ?? {};
  return {
    ...rule,
    selectors: {
      ...rule.selectors,
      [`${WINDOW_INACTIVE_SELECTOR} &`]: { animationPlayState: 'paused' },
    },
    '@media': {
      ...media,
      [MOTION_ALLOWED_QUERY]: {
        ...allowed,
        selectors: {
          ...allowed.selectors,
          [`${SAFE_RENDER_SELECTOR} &`]: safe,
        },
      },
    },
  };
};
