import { style } from '@vanilla-extract/css';

/** A flex row so the start can shrink while the tail keeps its width. */
export const MiddleTruncatedText = style({
  position: 'relative',
  display: 'flex',
  minWidth: 0,
  maxWidth: '100%',
  overflow: 'hidden',
  whiteSpace: 'nowrap',
});

/** A name short enough not to split: ordinary end truncation. */
export const MiddleTruncatedWhole = style({
  minWidth: 0,
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
});

/**
 * The split start clips (no ellipsis of its own, which would leave a gap of up
 * to one glyph before the tail) and fades out over its last 12px while it
 * overflows; the end span then leads with the ellipsis.
 */
export const MiddleTruncatedStart = style({
  minWidth: 0,
  overflow: 'hidden',
  textOverflow: 'clip',
  whiteSpace: 'nowrap',
  selectors: {
    '&[data-overflowing=true]': {
      maskImage: 'linear-gradient(to right, black calc(100% - 12px), transparent)',
      WebkitMaskImage: 'linear-gradient(to right, black calc(100% - 12px), transparent)',
    },
  },
});

/** The full name for assistive tech; the two visual spans are aria-hidden. */
export const MiddleTruncatedFull = style({
  position: 'absolute',
  width: 1,
  height: 1,
  margin: -1,
  padding: 0,
  overflow: 'hidden',
  clip: 'rect(0 0 0 0)',
  whiteSpace: 'nowrap',
  border: 0,
});

export const MiddleTruncatedEnd = style({
  flexShrink: 0,
  whiteSpace: 'nowrap',
});
