import { style } from '@vanilla-extract/css';
import { color, config } from 'folds';

/**
 * A question is a request for input, not a risk: it keeps the approval
 * card's shape on the neutral reading plane, edged in the primary tone.
 */
export const QuestionCard = style({
  border: `${config.borderWidth.B300} solid color-mix(in srgb, ${color.Primary.Main} 35%, ${color.Surface.ContainerLine})`,
  borderRadius: config.radii.R400,
  backgroundColor: color.Surface.Container,
  overflow: 'hidden',
});

export const QuestionText = style({
  fontWeight: config.fontWeight.W600,
  whiteSpace: 'pre-wrap',
  overflowWrap: 'anywhere',
});

/** Read-only choices on another requester's card. */
export const ChoiceList = style({
  margin: 0,
  paddingLeft: config.space.S500,
});

export const OtherRow = style({
  display: 'flex',
  gap: config.space.S200,
  alignItems: 'center',
});

export const ActionSpacer = style({
  flexGrow: 1,
});
