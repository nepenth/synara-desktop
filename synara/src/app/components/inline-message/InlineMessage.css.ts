import { style, styleVariants } from '@vanilla-extract/css';
import { color, config } from 'folds';

/** Inline status beneath a form or control: icon, then one or two lines of T200. */
export const InlineMessage = style({
  display: 'flex',
  alignItems: 'flex-start',
  gap: config.space.S100,
  minWidth: 0,
});

export const Glyph = style({
  flexShrink: 0,
  // Optical alignment with the first line of T200 text.
  marginTop: '0.1em',
});

export const Copy = style({
  minWidth: 0,
  overflowWrap: 'anywhere',
});

export const tone = styleVariants({
  critical: { color: color.Critical.Main },
  success: { color: color.Success.Main },
  info: { color: 'var(--synara-content-secondary)' },
});
