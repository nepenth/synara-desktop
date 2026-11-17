import { globalStyle } from '@vanilla-extract/css';
import {
  FOLDS_PROGRESS_PACKET_CLASS,
  FOLDS_SPINNER_SVG_CLASS,
  SAFE_RENDER_SELECTOR,
  WINDOW_INACTIVE_SELECTOR,
} from '../utils/motion';

/**
 * The motion budget (`utils/motion.ts`) for folds' two infinite animations,
 * applied app-wide so no Spinner or ProgressBar call site has to opt in.
 * App components use `continuousMotion()` in their own stylesheet instead.
 *
 * Spinners keep turning under reduced motion (index.css): they show progress.
 * The stepped form below is less motion, so it applies either way.
 */
const foldsInfinite = [FOLDS_SPINNER_SVG_CLASS, FOLDS_PROGRESS_PACKET_CLASS];

globalStyle(foldsInfinite.map((name) => `${WINDOW_INACTIVE_SELECTOR} .${name}`).join(', '), {
  animationPlayState: 'paused',
});

// A turn every 1.6 s in eight 45° steps: five paints a second. The folds
// keyframes set no timing function of their own, so steps() governs them.
globalStyle(`${SAFE_RENDER_SELECTOR} .${FOLDS_SPINNER_SVG_CLASS}`, {
  animationDuration: '1.6s',
  animationTimingFunction: 'steps(8)',
});

// The packet has two keyframe intervals of 1 s; five steps each.
globalStyle(`${SAFE_RENDER_SELECTOR} .${FOLDS_PROGRESS_PACKET_CLASS}`, {
  animationTimingFunction: 'steps(5)',
});
