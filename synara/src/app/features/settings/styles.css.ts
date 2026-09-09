import { style } from '@vanilla-extract/css';
import { config } from 'folds';

/**
 * Settings groups rest on the reading plane like every other settings surface
 * (room, space, common). Depth is reserved for interactive controls inside
 * them; the card itself is separated only by its container tint and border.
 */
export const SequenceCardStyle = style({
  padding: config.space.S300,
});
