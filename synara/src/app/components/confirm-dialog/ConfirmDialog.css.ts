import { style } from '@vanilla-extract/css';
import { color, config, toRem } from 'folds';

/** One confirmation: a short title, one consequence sentence, two buttons. */
export const ConfirmDialog = style({
  width: `min(${toRem(440)}, 92vw)`,
});

export const Header = style({
  padding: `0 ${config.space.S200} 0 ${config.space.S400}`,
  borderBottomWidth: config.borderWidth.B300,
});

export const Body = style({
  padding: config.space.S400,
  display: 'flex',
  flexDirection: 'column',
  gap: config.space.S400,
});

export const Footer = style({
  display: 'flex',
  justifyContent: 'flex-end',
  flexWrap: 'wrap',
  gap: config.space.S200,
  padding: `0 ${config.space.S400} ${config.space.S400}`,
});

/** Pushes the primary pair to the right edge when a secondary action is present. */
export const FooterSpacer = style({
  flexGrow: 1,
});

export const Title = style({
  color: color.Surface.OnContainer,
});
