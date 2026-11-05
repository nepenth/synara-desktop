import { keyframes, style, styleVariants } from '@vanilla-extract/css';
import { color, toRem } from 'folds';

/** Zero-height slot under the titlebar: the beam overlays, nothing reflows. */
export const Slot = style({
  position: 'relative',
  height: 0,
  flexShrink: 0,
  zIndex: 1000,
  pointerEvents: 'none',
});

const sweep = keyframes({
  '0%': { transform: 'translateX(-100%)' },
  '100%': { transform: 'translateX(250%)' },
});

const fadeOut = keyframes({
  '0%': { opacity: 1 },
  '60%': { opacity: 1 },
  '100%': { opacity: 0 },
});

export const Beam = style({
  position: 'absolute',
  top: 0,
  left: 0,
  right: 0,
  height: toRem(2),
  overflow: 'hidden',
});

export const BeamPhase = styleVariants({
  // A dim red track with a bright pulse running across it.
  trying: {
    background: `color-mix(in srgb, ${color.Critical.Main} 35%, transparent)`,
  },
  // The track fills green, holds, then fades away.
  connected: {
    background: color.Success.Main,
    animation: `${fadeOut} 1500ms ease-out forwards`,
    '@media': {
      '(prefers-reduced-motion: reduce)': { animation: 'none' },
    },
  },
});

export const Pulse = style({
  position: 'absolute',
  top: 0,
  bottom: 0,
  left: 0,
  width: '40%',
  background: `linear-gradient(90deg, transparent, ${color.Critical.Main}, transparent)`,
  animation: `${sweep} 1400ms ease-in-out infinite`,
  '@media': {
    '(prefers-reduced-motion: reduce)': {
      animation: 'none',
      width: '100%',
      background: color.Critical.Main,
    },
  },
});
