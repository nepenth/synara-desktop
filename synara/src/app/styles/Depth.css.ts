import { style } from '@vanilla-extract/css';
import { color, config, toRem } from 'folds';

/**
 * Synara's depth system is deliberately quiet: edge light establishes the
 * upper plane and soft occlusion separates it from the surface beneath. Text
 * never receives depth directly.
 */
export const quietEdgeLight = 'var(--synara-depth-edge-top)';
const quietEdgeLightStrong = 'var(--synara-depth-edge-top-strong)';
const quietEdgeDark = 'var(--synara-depth-edge-bottom)';
const quietShadowNear = 'var(--synara-depth-shadow-near)';
const quietShadowFar = 'var(--synara-depth-shadow-far)';
const quietRestEdge = 'var(--synara-depth-rest-edge)';
const quietAvatarShadow = 'var(--synara-depth-avatar-shadow)';

/**
 * A tiny opaque tonal fold makes depth legible even on near-black WebKit/WebView
 * surfaces, where a conventional black drop shadow has no visible contrast.
 */
export const quietSurfaceFold = `linear-gradient(180deg, var(--synara-depth-surface-highlight), var(--synara-depth-surface-shade))`;

/**
 * Resting collection content stays on the reading plane. This single inner
 * edge is intentionally below card-level contrast; elevation arrives only
 * when an item is selected, hovered, or focused.
 */
export const restingInnerEdge = `inset 0 1px 0 ${quietRestEdge}`;

export const raisedShadow = `inset 0 1px 0 ${quietEdgeLightStrong}, inset 0 -1px 0 ${quietEdgeDark}, 0 ${toRem(
  1
)} ${toRem(3)} color-mix(in srgb, ${quietShadowNear} 72%, transparent), 0 ${toRem(3)} ${toRem(
  8
)} color-mix(in srgb, ${quietShadowFar} 68%, transparent)`;

export const floatingShadow = `inset 0 1px 0 ${quietEdgeLightStrong}, inset 0 -1px 0 ${quietEdgeDark}, 0 ${toRem(
  3
)} ${toRem(7)} ${quietShadowNear}, 0 ${toRem(12)} ${toRem(30)} ${quietShadowFar}`;

export const criticalShadow = `inset 0 1px 0 ${quietEdgeLightStrong}, 0 ${toRem(3)} ${toRem(
  8
)} ${quietShadowNear}, 0 ${toRem(14)} ${toRem(36)} ${quietShadowFar}`;

const accessibilityFallbacks = {
  '@media': {
    '(prefers-reduced-transparency: reduce)': {
      backdropFilter: 'none',
    },
    '(prefers-contrast: more)': {
      backgroundImage: 'none',
      boxShadow: 'none',
      borderColor: 'var(--synara-depth-contrast-strong-edge)',
    },
  },
} as const;

export const floatingSurface = style({
  backgroundColor: `color-mix(in srgb, ${color.SurfaceVariant.Container} 96%, transparent)`,
  backgroundImage: quietSurfaceFold,
  border: `${config.borderWidth.B300} solid ${color.SurfaceVariant.ContainerLine}`,
  boxShadow: floatingShadow,
  backdropFilter: 'blur(18px) saturate(1.08)',
  '@media': {
    '(prefers-reduced-transparency: reduce)': {
      backgroundColor: color.SurfaceVariant.Container,
      backdropFilter: 'none',
    },
    '(prefers-contrast: more)': {
      backgroundImage: 'none',
      boxShadow: 'none',
      borderColor: 'var(--synara-depth-contrast-strong-edge)',
    },
  },
});

export const criticalSurface = style({
  boxShadow: criticalShadow,
  ...accessibilityFallbacks,
});

export const avatarSurface = style({
  boxShadow: `inset 0 0 0 ${config.borderWidth.B300} var(--synara-depth-avatar-boundary), 0 ${toRem(
    1
  )} ${toRem(2)} ${quietAvatarShadow}`,
  '@media': {
    '(prefers-contrast: more)': {
      boxShadow: `inset 0 0 0 ${config.borderWidth.B300} var(--synara-depth-contrast-edge)`,
    },
  },
});

export const avatarMedia = style({
  outline: `${config.borderWidth.B300} solid var(--synara-depth-avatar-boundary)`,
  outlineOffset: `calc(-1 * ${config.borderWidth.B300})`,
  '@media': {
    '(prefers-contrast: more)': {
      outlineColor: 'var(--synara-depth-contrast-strong-edge)',
    },
  },
});

export const tactileSurface = style({
  transition: 'transform 140ms ease-out, box-shadow 140ms ease-out, border-color 140ms ease-out',
  selectors: {
    '&:active': {
      transform: `translateY(${toRem(1)})`,
      boxShadow: `inset 0 1px 0 ${quietEdgeLight}`,
    },
  },
  '@media': {
    '(prefers-reduced-motion: reduce)': {
      transition: 'none',
      transform: 'none',
      selectors: {
        '&:active': {
          transform: 'none',
        },
      },
    },
    '(prefers-contrast: more)': {
      selectors: {
        '&:active': {
          boxShadow: 'none',
        },
      },
    },
  },
});

/**
 * Shared hover/selected treatment for compact controls that otherwise read as
 * flat tint changes. Resting controls stay on the reading plane.
 */
export const quietInteractiveSurface = style({
  transition:
    'background-color 140ms ease-out, box-shadow 140ms ease-out, transform 140ms ease-out',
  selectors: {
    '&:not(:disabled):not([aria-disabled=true]):hover, &:not(:disabled):not([aria-disabled=true]):focus-visible':
      {
        boxShadow: raisedShadow,
        transform: `translateY(-${toRem(1)})`,
      },
    '&:not(:disabled):not([aria-disabled=true])[aria-pressed=true], &:not(:disabled):not([aria-disabled=true])[aria-selected=true], &:not(:disabled):not([aria-disabled=true])[aria-current=page]':
      {
        boxShadow: raisedShadow,
      },
    '&:not(:disabled):not([aria-disabled=true]):active': {
      boxShadow: `inset 0 1px 0 ${quietEdgeLight}`,
      transform: 'translateY(0)',
    },
    '&:disabled, &[aria-disabled=true]': {
      boxShadow: 'none',
      transform: 'none',
    },
  },
  '@media': {
    '(prefers-reduced-motion: reduce)': {
      transition: 'none',
      transform: 'none',
      selectors: {
        '&:hover, &:focus-visible, &[aria-pressed=true], &[aria-selected=true], &[aria-current=page], &:active':
          {
            transform: 'none',
          },
      },
    },
    '(prefers-contrast: more)': {
      boxShadow: 'none',
      selectors: {
        '&:not(:disabled):not([aria-disabled=true]):hover, &:not(:disabled):not([aria-disabled=true]):focus-visible, &:not(:disabled):not([aria-disabled=true])[aria-pressed=true], &:not(:disabled):not([aria-disabled=true])[aria-selected=true], &:not(:disabled):not([aria-disabled=true])[aria-current=page], &:not(:disabled):not([aria-disabled=true]):active':
          {
            boxShadow: 'none',
            outline: `${config.borderWidth.B600} solid var(--synara-depth-contrast-strong-edge)`,
            outlineOffset: `calc(-1 * ${config.borderWidth.B600})`,
          },
        '&:disabled, &[aria-disabled=true]': {
          boxShadow: 'none',
          outline: 'none',
        },
      },
    },
  },
});
