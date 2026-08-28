import { style } from '@vanilla-extract/css';
import { color, config, toRem } from 'folds';

/**
 * Synara's depth system is deliberately quiet: edge light establishes the
 * upper plane and soft occlusion separates it from the surface beneath. Text
 * never receives depth directly.
 */
export const quietEdgeLight = 'rgba(255, 255, 255, 0.1)';
export const quietEdgeLightStrong = 'rgba(255, 255, 255, 0.16)';
export const quietShadowNear = `color-mix(in srgb, ${color.Other.Shadow} 48%, transparent)`;
export const quietShadowFar = `color-mix(in srgb, ${color.Other.Shadow} 28%, transparent)`;

export const raisedShadow = `inset 0 1px 0 ${quietEdgeLight}, 0 ${toRem(1)} ${toRem(
  2
)} ${quietShadowNear}, 0 ${toRem(4)} ${toRem(12)} ${quietShadowFar}`;

export const floatingShadow = `inset 0 1px 0 ${quietEdgeLightStrong}, 0 ${toRem(2)} ${toRem(
  4
)} ${quietShadowNear}, 0 ${toRem(10)} ${toRem(28)} ${quietShadowFar}`;

export const criticalShadow = `inset 0 1px 0 ${quietEdgeLightStrong}, 0 ${toRem(3)} ${toRem(
  8
)} ${quietShadowNear}, 0 ${toRem(14)} ${toRem(36)} ${quietShadowFar}`;

const accessibilityFallbacks = {
  '@media': {
    '(prefers-reduced-transparency: reduce)': {
      backdropFilter: 'none',
    },
    '(prefers-contrast: more)': {
      boxShadow: 'none',
      borderColor: color.Surface.OnContainer,
    },
  },
} as const;

export const raisedSurface = style({
  border: `${config.borderWidth.B300} solid ${color.Surface.ContainerLine}`,
  boxShadow: raisedShadow,
  ...accessibilityFallbacks,
});

export const floatingSurface = style({
  backgroundColor: `color-mix(in srgb, ${color.SurfaceVariant.Container} 96%, transparent)`,
  border: `${config.borderWidth.B300} solid ${color.SurfaceVariant.ContainerLine}`,
  boxShadow: floatingShadow,
  backdropFilter: 'blur(18px) saturate(1.08)',
  '@media': {
    '(prefers-reduced-transparency: reduce)': {
      backgroundColor: color.SurfaceVariant.Container,
      backdropFilter: 'none',
    },
    '(prefers-contrast: more)': {
      boxShadow: 'none',
      borderColor: color.Surface.OnContainer,
    },
  },
});

export const criticalSurface = style({
  boxShadow: criticalShadow,
  ...accessibilityFallbacks,
});

export const avatarSurface = style({
  boxShadow: `0 ${toRem(1)} ${toRem(2)} ${quietShadowNear}, 0 ${toRem(3)} ${toRem(
    8
  )} ${quietShadowFar}`,
  '@media': {
    '(prefers-contrast: more)': {
      boxShadow: 'none',
    },
  },
});

export const avatarMedia = style({
  outline: `${config.borderWidth.B300} solid ${color.Surface.ContainerLine}`,
  outlineOffset: `calc(-1 * ${config.borderWidth.B300})`,
  '@media': {
    '(prefers-contrast: more)': {
      outlineColor: color.Surface.OnContainer,
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
