import { style } from '@vanilla-extract/css';
import { recipe } from '@vanilla-extract/recipes';
import { DefaultReset, color, config, toRem } from 'folds';

export const RoomViewFollowingPlaceholder = style([
  DefaultReset,
  {
    height: toRem(28),
  },
]);

export const RoomViewFollowing = recipe({
  base: [
    DefaultReset,
    {
      minHeight: toRem(28),
      padding: `0 ${config.space.S400}`,
      width: '100%',
      backgroundColor: color.Surface.Container,
      color: color.Surface.OnContainer,
      outline: 'none',
    },
  ],
  variants: {
    clickable: {
      true: {
        cursor: 'pointer',
        selectors: {
          '&:hover, &:focus-visible': {
            color: color.Primary.Main,
          },
          // A colour change alone is ~1.1:1; keyboard focus needs a ring.
          '&:focus-visible': {
            outline: `${config.borderWidth.B600} solid ${color.Primary.Main}`,
            outlineOffset: `calc(-1 * ${config.borderWidth.B600})`,
          },
          '&:active': {
            color: color.Primary.Main,
          },
        },
      },
    },
  },
});
