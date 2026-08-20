import { style } from '@vanilla-extract/css';
import { toRem } from 'folds';

/** Compact sort glyphs — sit under the Favorites/Rooms title size, not beside as large buttons. */
export const SortIconButton = style({
  width: `${toRem(14)} !important`,
  height: `${toRem(14)} !important`,
  minWidth: `${toRem(14)} !important`,
  minHeight: `${toRem(14)} !important`,
  padding: '0 !important',
});

export const SortIcon = style({
  width: `${toRem(11)} !important`,
  height: `${toRem(11)} !important`,
  minWidth: `${toRem(11)} !important`,
  minHeight: `${toRem(11)} !important`,
});
