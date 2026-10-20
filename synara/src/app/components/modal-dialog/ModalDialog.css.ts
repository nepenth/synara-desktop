import { style } from '@vanilla-extract/css';

/**
 * The dialog surface takes focus when it opens so screen readers announce its
 * name; it is not itself a control, so it draws no focus ring.
 */
export const ModalDialog = style({
  ':focus': {
    outline: 'none',
  },
});
