import { keyframes, style } from '@vanilla-extract/css';
import { color, config, toRem } from 'folds';
import { floatingSurface } from '../../styles/Depth.css';
import { MOTION_ALLOWED_QUERY } from '../../utils/motion';

/*
 * The card floats in the top-right corner of the client shell, below the
 * pane headers: the composer and the list's bottom bar are at the bottom of
 * the window, so it never covers them. On a narrow window it spans the
 * width, with the same gutters.
 */
const GUTTER = config.space.S300;
// Below the pane header (folds Header size 600) and a gutter.
const TOP = `calc(${toRem(56)} + ${GUTTER})`;
// Room for the composer (or the bottom bar) and its padding at the bottom.
const BOTTOM_RESERVE = toRem(120);

const enter = keyframes({
  from: { opacity: 0, transform: `translateY(${toRem(-6)})` },
  to: { opacity: 1, transform: 'none' },
});

export const WhatsNewCard = style([
  floatingSurface,
  {
    position: 'absolute',
    zIndex: 3,
    top: TOP,
    right: GUTTER,
    width: `min(${toRem(340)}, calc(100vw - 2 * ${GUTTER}))`,
    maxHeight: `calc(100% - ${TOP} - ${BOTTOM_RESERVE})`,
    overflowY: 'auto',
    borderRadius: config.radii.R400,
    padding: `${config.space.S300} ${config.space.S300} ${config.space.S400} ${config.space.S400}`,
    color: color.SurfaceVariant.OnContainer,
    '@media': {
      [MOTION_ALLOWED_QUERY]: {
        animation: `${enter} 160ms ease-out`,
      },
    },
    selectors: {
      '&:focus-visible': {
        outline: `${config.borderWidth.B600} solid ${color.Primary.Main}`,
        outlineOffset: config.borderWidth.B300,
      },
    },
  },
]);

export const WhatsNewList = style({
  margin: 0,
  paddingLeft: config.space.S400,
  display: 'flex',
  flexDirection: 'column',
  gap: config.space.S100,
});
