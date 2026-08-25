import { style } from '@vanilla-extract/css';
import { config, toRem } from 'folds';

export const RoomChannelGlyph = style({
  color: 'var(--synara-content-secondary)',
  fontSize: toRem(22),
  fontWeight: 500,
  lineHeight: 1,
});

export const RoomTitle = style({
  color: 'var(--synara-content-heading)',
  letterSpacing: '-0.01em',
});

export const HeaderTopic = style({
  ':hover': {
    cursor: 'pointer',
    opacity: config.opacity.P500,
    textDecoration: 'underline',
  },
});
