import { globalStyle } from '@vanilla-extract/css';
import { Disabled, color, config } from 'folds';

/**
 * App-wide focus indicators for folds text fields. folds removes the field's
 * own outline and marks focus with its resting border colour (~1.2:1), and its
 * TextArea draws nothing at all. An outline in Primary.Main reaches 3:1 on every
 * surface and, unlike a box-shadow, survives forced-colours mode.
 *
 * folds does not export its field classes; both fields carry its `Disabled`
 * class, which is the stable hook used here.
 */
const fieldFocusRing = {
  outline: `${config.borderWidth.B500} solid ${color.Primary.Main}`,
  outlineOffset: `calc(-1 * ${config.borderWidth.B500})`,
};

// folds Input: the <input> sits directly inside the bordered field. Checkboxes
// and radios share the class but draw the folds focus ring themselves.
globalStyle(
  `div:has(> input.${Disabled}:not([type=checkbox]):not([type=radio]):focus-visible)`,
  fieldFocusRing
);

// folds TextArea is the bordered <textarea> itself.
globalStyle(`textarea.${Disabled}:focus-visible`, fieldFocusRing);
