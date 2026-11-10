import { style } from '@vanilla-extract/css';
import { color, config, toRem } from 'folds';

export const ToggleWrap = style({
  position: 'relative',
  display: 'inline-flex',
  flexShrink: 0,
});

/** The hidden list's unread, pinned to the button's top-right corner. */
export const ToggleBadge = style({
  position: 'absolute',
  top: toRem(-2),
  right: toRem(-2),
  display: 'inline-flex',
  pointerEvents: 'none',
  borderRadius: config.radii.Pill,
  boxShadow: `0 0 0 ${toRem(2)} ${color.SurfaceVariant.Container}`,
});
