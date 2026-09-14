import { style } from '@vanilla-extract/css';
import { config } from 'folds';
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
