import { style } from '@vanilla-extract/css';
import { color, toRem } from 'folds';

/**
 * Below the Desktop width the members drawer and the room's side panels open
 * over the room's right edge instead of beside it, so the timeline keeps its
 * width. Never wider than the room area.
 */
export const Overlay = style({
  position: 'absolute',
  top: 0,
  right: 0,
  bottom: 0,
  zIndex: 2,
  display: 'flex',
  maxWidth: '100%',
  borderLeft: `1px solid ${color.Background.ContainerLine}`,
  backgroundColor: color.Background.Container,
  boxShadow: `0 0 ${toRem(24)} rgba(0, 0, 0, 0.24)`,
});
