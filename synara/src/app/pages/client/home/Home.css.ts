import { style } from '@vanilla-extract/css';
import { toRem } from 'folds';

/** One discoverable sort menu with a comfortable pointer target. */
export const SortIconButton = style({
  width: `${toRem(28)} !important`,
  height: `${toRem(28)} !important`,
  minWidth: `${toRem(28)} !important`,
  minHeight: `${toRem(28)} !important`,
  padding: '0 !important',
});

export const SortIcon = style({
  width: `${toRem(14)} !important`,
  height: `${toRem(14)} !important`,
  minWidth: `${toRem(14)} !important`,
  minHeight: `${toRem(14)} !important`,
});
