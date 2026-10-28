import { style } from '@vanilla-extract/css';
import { color, config } from 'folds';

export const ApprovalCard = style({
  border: `${config.borderWidth.B300} solid ${color.Surface.ContainerLine}`,
  borderRadius: config.radii.R400,
  backgroundColor: color.Surface.Container,
  overflow: 'hidden',
  '@media': {
    '(prefers-contrast: more)': { borderColor: color.Surface.OnContainer },
  },
});

export const InboxApprovalCard = style({
  borderRadius: config.radii.R400,
  backgroundColor: color.Surface.Container,
  overflow: 'hidden',
});
