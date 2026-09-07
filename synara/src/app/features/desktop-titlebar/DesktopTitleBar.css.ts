import { style } from '@vanilla-extract/css';
import { color, config, toRem } from 'folds';

export const TitleBar = style({
  height: toRem(40),
  padding: `0 ${config.space.S200}`,
  flexShrink: 0,
  userSelect: 'none',
  borderBottom: `1px solid ${color.Background.ContainerLine}`,
});

export const DragRegion = style({
  minWidth: 0,
  height: '100%',
});
