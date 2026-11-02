import { style } from '@vanilla-extract/css';

/** A flex row so the start can shrink while the tail keeps its width. */
export const MiddleTruncatedText = style({
  display: 'flex',
  minWidth: 0,
  maxWidth: '100%',
  overflow: 'hidden',
  whiteSpace: 'nowrap',
});

export const MiddleTruncatedStart = style({
  minWidth: 0,
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
});

export const MiddleTruncatedEnd = style({
  flexShrink: 0,
  whiteSpace: 'nowrap',
});
