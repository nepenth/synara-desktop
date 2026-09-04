import { style } from '@vanilla-extract/css';
import { toRem } from 'folds';

export const TitleBar = style({
  height: toRem(40),
  flexShrink: 0,
  userSelect: 'none',
});

export const DragRegion = style({
  minWidth: 0,
  height: '100%',
});
