import { style } from '@vanilla-extract/css';
import { color, config, DefaultReset, toRem } from 'folds';

export const RoomComposer = style([
  DefaultReset,
  {
    width: '100%',
    minWidth: 0,
  },
]);

export const RoomComposerReply = style([
  DefaultReset,
  {
    minWidth: 0,
  },
]);

export const RoomComposerLeadingAction = style([
  DefaultReset,
  {
    display: 'flex',
    alignItems: 'center',
    minWidth: 0,
  },
]);

export const RoomComposerFloatingActions = style([
  DefaultReset,
  {
    display: 'flex',
    alignItems: 'center',
    gap: toRem(2),
    minWidth: 0,
  },
]);

export const RoomComposerToolbar = style([
  DefaultReset,
  {
    minWidth: 0,
  },
]);

export const ComposerError = style({
  margin: `0 ${config.space.S300} ${config.space.S200}`,
  padding: `${config.space.S200} ${config.space.S300}`,
  border: `1px solid ${color.Critical.Main}`,
  borderRadius: config.radii.R300,
  background: color.Critical.Container,
  color: color.Critical.OnContainer,
});
