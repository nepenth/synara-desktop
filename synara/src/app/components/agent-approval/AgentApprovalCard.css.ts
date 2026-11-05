import { style } from '@vanilla-extract/css';
import { color, config } from 'folds';
import { criticalSurface } from '../../styles/Depth.css';

/**
 * A pending approval sits on the critical plane with a softened red edge; the
 * edge carries the risk on its own, so the card needs no "Critical" chip.
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

export const PromptTitle = style({
  fontWeight: config.fontWeight.W600,
});

/** Keeps Deny apart from the two approvals on the right. */
export const ActionSpacer = style({
  flexGrow: 1,
});

/** "Approve always" confirmation: a caution, not a failure, so Warning tone. */
export const ConfirmAlways = style({
  border: `${config.borderWidth.B300} solid ${color.Warning.ContainerLine}`,
  borderRadius: config.radii.R300,
  padding: config.space.S300,
  backgroundColor: color.Warning.Container,
  color: color.Warning.OnContainer,
});

export const MoreWays = style({
  display: 'flex',
  flexDirection: 'column',
  gap: config.space.S100,
});

export const MoreWaysSummary = style({
  cursor: 'pointer',
  listStyle: 'none',
  width: 'fit-content',
  selectors: {
    '&::-webkit-details-marker': { display: 'none' },
    '&:focus-visible': {
      outline: `${config.borderWidth.B500} solid ${color.Primary.Main}`,
      outlineOffset: 2,
      borderRadius: config.radii.R300,
    },
  },
});
