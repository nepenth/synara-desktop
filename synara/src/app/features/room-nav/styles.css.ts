import { style } from '@vanilla-extract/css';
import { config, toRem } from 'folds';

export const CategoryButton = style({
  flexGrow: 1,
});
export const CategoryButtonIcon = style({
  opacity: config.opacity.P400,
});

export const RoomGlyph = style({
  width: toRem(18),
  flex: 'none',
  color: 'var(--synara-content-secondary)',
  fontSize: toRem(17),
  fontWeight: 500,
  lineHeight: 1,
  textAlign: 'center',
});

export const RoomName = style({
  color: 'var(--synara-content-primary)',
  letterSpacing: '-0.005em',
});
