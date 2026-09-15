import { style } from '@vanilla-extract/css';
import { color, config, toRem } from 'folds';
import { quietInteractiveSurface } from '../../styles/Depth.css';

/**
 * Settings groups rest on the reading plane like every other settings surface
 * (room, space, common). Depth is reserved for interactive controls inside
 * them; the card itself is separated only by its container tint and border.
 */
export const SequenceCardStyle = style({
  padding: config.space.S300,
});

/**
 * Shared quiet rest state for every settings control (close buttons, menu
 * triggers, selector chips/buttons, menu options). Resting controls stay
 * flat; hover/focus-visible/pressed gain the house edge light + soft
 * occlusion shadow. Selection is carried by `aria-pressed`/`aria-selected`,
 * never by a saturated variant fill.
 */
export const SettingsQuietControl = style([quietInteractiveSurface]);

/**
 * Color swatches must keep a focus ring even when unselected. Inline
 * `outline: none` on the unselected rest made keyboard focus invisible,
 * and the previous unselected border was a hardcoded light-only rgba.
 */
export const SettingsThemeSwatch = style({
  width: toRem(22),
  height: toRem(22),
  padding: 0,
  borderRadius: '50%',
  cursor: 'pointer',
  border: `${config.borderWidth.B300} solid ${color.Surface.ContainerLine}`,
  selectors: {
    '&[aria-pressed=true]': {
      outline: '2px solid currentColor',
      outlineOffset: '2px',
    },
    '&:focus-visible': {
      outline: `2px solid ${color.Primary.Main}`,
      outlineOffset: '2px',
    },
    '&:disabled, &[aria-disabled=true]': {
      cursor: 'not-allowed',
    },
  },
});
