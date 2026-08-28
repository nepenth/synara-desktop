import { style } from '@vanilla-extract/css';
import { DefaultReset, config, toRem } from 'folds';
import { avatarSurface, floatingSurface, tactileSurface } from '../../../styles/Depth.css';

export const MessageBase = style({
  contain: 'layout style',
  overflowAnchor: 'none',
  position: 'relative',
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

export const ReactionsContainer = style({
  selectors: {
    '&:empty': {
      display: 'none',
    },
  },
});

export const ReactionsTooltipText = style({
  wordBreak: 'break-word',
});
