import { style } from '@vanilla-extract/css';
import { color, config, toRem } from 'folds';
import {
  quietInteractiveSurface,
  quietSurfaceFold,
  raisedShadow,
  restingInnerEdge,
} from '../../../styles/Depth.css';

export const Panel = style({
  backgroundImage: quietSurfaceFold,
  border: `${config.borderWidth.B300} solid ${color.Surface.ContainerLine}`,
  '@media': {
    '(prefers-contrast: more)': {
      backgroundImage: 'none',
      borderColor: 'var(--synara-depth-contrast-strong-edge)',
    },
  },
});

export const HeaderAction = style([quietInteractiveSurface]);

export const ComposerCard = style({
  minWidth: 0,
  margin: config.space.S300,
  padding: config.space.S300,
  border: `${config.borderWidth.B300} solid ${color.SurfaceVariant.ContainerLine}`,
  borderRadius: config.radii.R400,
  backgroundColor: color.SurfaceVariant.Container,
  backgroundImage: quietSurfaceFold,
  boxShadow: restingInnerEdge,
  '@media': {
    '(prefers-contrast: more)': {
      backgroundImage: 'none',
      boxShadow: 'none',
      borderColor: 'var(--synara-depth-contrast-strong-edge)',
    },
  },
});

export const KindSwitch = style({
  width: 'fit-content',
  padding: toRem(3),
  border: `${config.borderWidth.B300} solid ${color.SurfaceVariant.ContainerLine}`,
  borderRadius: config.radii.R400,
  backgroundColor: color.Surface.Container,
  boxShadow: restingInnerEdge,
  overflow: 'visible',
  '@media': {
    '(prefers-contrast: more)': {
      boxShadow: 'none',
      borderColor: 'var(--synara-depth-contrast-strong-edge)',
    },
  },
});

export const KindButton = style([
  quietInteractiveSurface,
  {
    minWidth: toRem(72),
    overflow: 'visible',
  },
]);

export const ComposerTextArea = style({
  boxShadow: restingInnerEdge,
  transition: 'border-color 140ms ease-out, box-shadow 140ms ease-out',
  selectors: {
    '&:focus': {
      borderColor: color.Primary.MainLine,
      boxShadow: raisedShadow,
    },
  },
  '@media': {
    '(prefers-reduced-motion: reduce)': {
      transition: 'none',
    },
    '(prefers-contrast: more)': {
      boxShadow: 'none',
      selectors: {
        '&:focus': {
          boxShadow: 'none',
          outline: `${config.borderWidth.B600} solid var(--synara-depth-contrast-strong-edge)`,
          outlineOffset: `calc(-1 * ${config.borderWidth.B600})`,
        },
      },
    },
  },
});

export const AddAction = style([quietInteractiveSurface]);

export const NotesScroll = style({
  borderTop: `${config.borderWidth.B300} solid ${color.Surface.ContainerLine}`,
});

export const NoteItem = style({
  padding: config.space.S300,
  border: `${config.borderWidth.B300} solid ${color.SurfaceVariant.ContainerLine}`,
  borderRadius: config.radii.R400,
  backgroundColor: color.SurfaceVariant.Container,
  backgroundImage: quietSurfaceFold,
  boxShadow: restingInnerEdge,
  transition: 'border-color 140ms ease-out, box-shadow 140ms ease-out',
  selectors: {
    '&:hover, &:focus-within': {
      borderColor: color.SurfaceVariant.OnContainer,
      boxShadow: raisedShadow,
    },
  },
  '@media': {
    '(prefers-reduced-motion: reduce)': {
      transition: 'none',
    },
    '(prefers-contrast: more)': {
      backgroundImage: 'none',
      boxShadow: 'none',
      selectors: {
        '&:hover, &:focus-within': {
          borderColor: 'var(--synara-depth-contrast-strong-edge)',
          boxShadow: 'none',
        },
      },
    },
  },
});

export const ItemAction = style([quietInteractiveSurface]);

export const EmptyState = style({
  minHeight: toRem(180),
  padding: config.space.S400,
  textAlign: 'center',
  border: `${config.borderWidth.B300} solid ${color.SurfaceVariant.ContainerLine}`,
  borderRadius: config.radii.R400,
  backgroundColor: color.SurfaceVariant.Container,
  backgroundImage: quietSurfaceFold,
  boxShadow: restingInnerEdge,
  '@media': {
    '(prefers-contrast: more)': {
      backgroundImage: 'none',
      boxShadow: 'none',
    },
  },
});
