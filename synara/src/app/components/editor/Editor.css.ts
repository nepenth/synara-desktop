import { style } from '@vanilla-extract/css';
import { color, config, DefaultReset, toRem } from 'folds';

export const Editor = style([
  DefaultReset,
  {
    backgroundColor: color.Secondary.Container,
    color: color.Secondary.OnContainer,
    boxShadow: `inset 0 0 0 ${config.borderWidth.B300} ${color.Secondary.ContainerLine}`,
    borderRadius: config.radii.R400,
    overflow: 'hidden',
  },
]);

export const EditorOptions = style([
  DefaultReset,
  {
    padding: config.space.S200,
  },
]);

export const EditorTextareaArea = style([
  DefaultReset,
  {
    position: 'relative',
    flexGrow: 1,
    minWidth: 0,
  },
]);

export const EditorTextareaScroll = style({
  width: '100%',
  minWidth: 0,
});

export const EditorTextarea = style([
  DefaultReset,
  {
    flexGrow: 1,
    height: '100%',
    padding: `${toRem(13)} ${toRem(1)}`,
    selectors: {
      [`${EditorTextareaScroll}:first-child &`]: {
        paddingLeft: toRem(13),
      },
      [`${EditorTextareaScroll}:last-child &`]: {
        paddingRight: toRem(13),
      },
      '&:focus': {
        outline: 'none',
      },
    },
  },
]);

export const EditorTextareaWithFloatingOptions = style({
  selectors: {
    '&::before': {
      content: '""',
      float: 'right',
      width: toRem(184),
      height: toRem(36),
      pointerEvents: 'none',
    },
  },
});

export const EditorPlaceholderContainer = style([
  DefaultReset,
  {
    opacity: config.opacity.Placeholder,
    pointerEvents: 'none',
    userSelect: 'none',
  },
]);

export const EditorPlaceholderTextVisual = style([
  DefaultReset,
  {
    display: 'block',
    paddingTop: toRem(13),
    paddingLeft: toRem(1),
  },
]);

export const EditorToolbarBase = style({
  padding: `0 ${config.borderWidth.B300}`,
});

export const EditorToolbar = style({
  padding: config.space.S100,
});

export const MarkdownBtnBox = style({
  paddingRight: config.space.S100,
});

export const EditorFloatingOptions = style([
  DefaultReset,
  {
    position: 'absolute',
    zIndex: 1,
    top: toRem(6),
    right: toRem(6),
    display: 'flex',
    alignItems: 'center',
    gap: toRem(2),
    padding: toRem(3),
    borderRadius: config.radii.R300,
    backgroundColor: color.SurfaceVariant.Container,
    boxShadow: `0 0 0 ${config.borderWidth.B300} ${color.SurfaceVariant.ContainerLine}, 0 ${toRem(
      4
    )} ${toRem(18)} #00000026`,
    backdropFilter: 'blur(16px)',
    pointerEvents: 'auto',
  },
]);
