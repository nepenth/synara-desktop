import { style } from '@vanilla-extract/css';
import { color, config, DefaultReset, toRem } from 'folds';
import {
  floatingShadow,
  quietSurfaceFold,
  raisedShadow,
  tactileSurface,
} from '../../styles/Depth.css';

export const Editor = style([
  DefaultReset,
  {
    backgroundColor: color.Secondary.Container,
    backgroundImage: quietSurfaceFold,
    color: color.Secondary.OnContainer,
    border: `${config.borderWidth.B300} solid ${color.Secondary.ContainerLine}`,
    boxShadow: raisedShadow,
    borderRadius: config.radii.R400,
    overflow: 'hidden',
    transition: 'border-color 140ms ease-out, box-shadow 140ms ease-out',
    selectors: {
      '&:focus-within': {
        borderColor: color.Primary.MainLine,
        boxShadow: `${raisedShadow}, 0 0 0 ${config.borderWidth.B300} color-mix(in srgb, ${color.Primary.Main} 26%, transparent)`,
      },
    },
    '@media': {
      '(prefers-reduced-motion: reduce)': {
        transition: 'none',
      },
      '(prefers-contrast: more)': {
        backgroundImage: 'none',
        boxShadow: 'none',
        borderColor: color.Secondary.OnContainer,
        selectors: {
          '&:focus-within': {
            boxShadow: 'none',
            borderColor: color.Primary.Main,
            outline: `${config.borderWidth.B600} solid ${color.Primary.Main}`,
            outlineOffset: `calc(-1 * ${config.borderWidth.B600})`,
          },
        },
      },
    },
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
    color: 'var(--synara-message-foreground)',
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
  tactileSurface,
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
    backgroundImage: quietSurfaceFold,
    border: `${config.borderWidth.B300} solid ${color.SurfaceVariant.ContainerLine}`,
    boxShadow: floatingShadow,
    backdropFilter: 'blur(18px) saturate(1.08)',
    pointerEvents: 'auto',
    '@media': {
      '(prefers-reduced-transparency: reduce)': {
        backdropFilter: 'none',
        backgroundColor: color.SurfaceVariant.Container,
      },
      '(prefers-contrast: more)': {
        backgroundImage: 'none',
        boxShadow: 'none',
        borderColor: color.SurfaceVariant.OnContainer,
      },
    },
  },
]);
