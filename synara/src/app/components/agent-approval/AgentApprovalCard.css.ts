import { style } from '@vanilla-extract/css';
import { color, config } from 'folds';
import { criticalSurface } from '../../styles/Depth.css';

/**
 * A pending approval sits on the critical plane with a softened red edge;
 * the Critical pill names the risk, so the edge only has to whisper it.
 */
export const ApprovalCard = style([
  criticalSurface,
  {
    border: `${config.borderWidth.B300} solid color-mix(in srgb, ${color.Critical.Main} 45%, ${color.Surface.ContainerLine})`,
    borderRadius: config.radii.R400,
    backgroundColor: color.Surface.Container,
    overflow: 'hidden',
  },
]);

export const InboxApprovalCard = style({
  borderRadius: config.radii.R400,
  backgroundColor: color.Surface.Container,
  overflow: 'hidden',
});
