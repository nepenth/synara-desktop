import { style } from '@vanilla-extract/css';
import { color, config, DefaultReset, toRem } from 'folds';

/** In the composer's place, in its frame and height band, so the timeline holds still. */
export const ComposerFallback = style([
  DefaultReset,
  {
    boxSizing: 'border-box',
    display: 'flex',
    alignItems: 'center',
    flexWrap: 'wrap',
    gap: `${config.space.S200} ${config.space.S300}`,
    minHeight: toRem(52),
    padding: `${config.space.S200} ${config.space.S300}`,
    backgroundColor: color.Secondary.Container,
    color: color.Secondary.OnContainer,
    border: `${config.borderWidth.B300} solid ${color.Secondary.ContainerLine}`,
    borderRadius: config.radii.R400,
  },
]);

export const ComposerFallbackMessage = style({
  display: 'flex',
  alignItems: 'center',
  gap: config.space.S200,
  flex: '1 1 16rem',
  minWidth: 0,
});

export const ComposerFallbackIcon = style({
  flexShrink: 0,
  color: color.Warning.Main,
});

export const ComposerFallbackActions = style({
  display: 'flex',
  alignItems: 'center',
  flexWrap: 'wrap',
  gap: config.space.S100,
});
