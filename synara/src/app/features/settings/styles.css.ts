import { style } from '@vanilla-extract/css';
import { config } from 'folds';
import { quietSurfaceFold, restingInnerEdge } from '../../styles/Depth.css';

export const SequenceCardStyle = style({
  padding: config.space.S300,
  backgroundImage: quietSurfaceFold,
  boxShadow: restingInnerEdge,
  '@media': {
    '(prefers-contrast: more)': {
      backgroundImage: 'none',
      boxShadow: 'none',
      outline: '1px solid var(--synara-depth-contrast-edge)',
      outlineOffset: '-1px',
    },
  },
});
