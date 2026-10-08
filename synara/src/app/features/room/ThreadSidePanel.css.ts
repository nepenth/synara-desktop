import { style } from '@vanilla-extract/css';
import { color, config, toRem } from 'folds';

export const Pane = style({
  position: 'relative',
  minWidth: 0,
  height: '100%',
  borderLeft: `1px solid ${color.Background.ContainerLine}`,
  backgroundColor: color.Background.Container,
});

export const Header = style({
  height: toRem(48),
  padding: `0 ${config.space.S200} 0 ${config.space.S400}`,
  borderBottom: `1px solid ${color.Background.ContainerLine}`,
});

/** A 6px grab strip over the pane's left border. */
export const ResizeHandle = style({
  position: 'absolute',
  top: 0,
  bottom: 0,
  left: toRem(-3),
  width: toRem(6),
  cursor: 'col-resize',
  zIndex: 1,
  touchAction: 'none',
  selectors: {
    '&:hover, &:focus-visible': {
      backgroundColor: color.Primary.ContainerLine,
      outline: 'none',
    },
  },
});
