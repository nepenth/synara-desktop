import { style } from '@vanilla-extract/css';
import { color, config, toRem } from 'folds';
import { raisedShadow } from '../../styles/Depth.css';

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

/** Resting rows remain on the reading plane; interaction supplies elevation. */
export const RoomSurface = style({
  backgroundColor: 'transparent',
  borderColor: 'transparent',
  boxShadow: 'none',
  selectors: {
    '&:hover, &:focus-within': {
      backgroundColor: color.Surface.ContainerHover,
      borderColor: color.Surface.ContainerLine,
      boxShadow: raisedShadow,
    },
  },
  '@media': {
    '(prefers-contrast: more)': {
      boxShadow: 'none',
      borderColor: 'var(--synara-depth-contrast-edge)',
      selectors: {
        '&:hover, &:focus-within': {
          borderColor: 'var(--synara-depth-contrast-strong-edge)',
          boxShadow: 'none',
        },
      },
    },
  },
});
