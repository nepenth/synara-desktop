import { style } from '@vanilla-extract/css';
import { color, config } from 'folds';
import { criticalSurface, raisedShadow } from '../../styles/Depth.css';

export const ApprovalCard = style([
  criticalSurface,
  {
    border: `${config.borderWidth.B300} solid ${color.Critical.Main}`,
    borderRadius: config.radii.R400,
    backgroundColor: color.Surface.Container,
    overflow: 'hidden',
  },
]);

export const ApprovalDetail = style({
  boxShadow: raisedShadow,
  '@media': {
    '(prefers-contrast: more)': {
      boxShadow: 'none',
      borderColor: color.SurfaceVariant.OnContainer,
    },
  },
});
