import { style } from '@vanilla-extract/css';
import { DefaultReset, config, toRem } from 'folds';
import {
  avatarSurface,
  floatingSurface,
  raisedShadow,
  tactileSurface,
} from '../../../styles/Depth.css';

export const MessageBase = style({
  contain: 'layout style',
  overflowAnchor: 'none',
  position: 'relative',
  border: `${config.borderWidth.B300} solid transparent`,
  borderRadius: config.radii.R400,
  boxShadow: 'none',
  transition:
    'background-color 140ms ease-out, border-color 140ms ease-out, box-shadow 140ms ease-out',
  selectors: {
    '&:hover, &:focus-within': {
      backgroundColor: 'color-mix(in srgb, currentColor 3%, transparent)',
      borderColor: 'color-mix(in srgb, currentColor 10%, transparent)',
      boxShadow: raisedShadow,
    },
  },
  '@media': {
    '(prefers-reduced-motion: reduce)': {
      transition: 'none',
    },
    '(prefers-contrast: more)': {
      boxShadow: 'none',
      borderColor: 'var(--synara-depth-contrast-edge)',
      selectors: {
        '&:hover, &:focus-within': {
          borderColor: 'var(--synara-depth-contrast-strong-edge)',
          boxShadow: 'none',
        },
      },
    },
  },
});
export const MessageBaseBubbleCollapsed = style({
  paddingTop: 0,
});

export const MessageOptionsBase = style([
  DefaultReset,
  {
    position: 'absolute',
    top: toRem(24),
    right: config.space.S200,
    zIndex: 20,
  },
]);
export const MessageOptionsBar = style([
  DefaultReset,
  floatingSurface,
  tactileSurface,
  {
    padding: config.space.S100,
    borderRadius: config.radii.R400,
    overflow: 'hidden',
  },
]);

export const BubbleAvatarBase = style({
  paddingTop: 0,
});

export const MessageAvatar = style([avatarSurface, { cursor: 'pointer' }]);

export const MessageQuickReaction = style({
  minWidth: toRem(32),
});

export const MessageMenuGroup = style({
  padding: config.space.S100,
});

export const MessageMenuItemText = style({
  flexGrow: 1,
});
