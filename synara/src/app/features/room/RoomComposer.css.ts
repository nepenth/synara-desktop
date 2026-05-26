import { style } from '@vanilla-extract/css';
import { DefaultReset, toRem } from 'folds';

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
