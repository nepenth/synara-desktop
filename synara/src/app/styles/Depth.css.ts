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

/**
 * Hover of a quiet action: faint tint plus a raised edge. Pressed/expanded
 * must not reuse this shadow — a hovered unpressed control would otherwise
 * look identical to a pressed control, and hovering a pressed control
 * would replace its inset indicator with a raised one.
 */
const quietActionHoverShadow = `inset 0 1px 0 ${quietEdgeLight}, 0 1px 2px color-mix(in srgb, var(--synara-depth-shadow-near) 45%, transparent)`;
const quietActionPressedShadow = `inset 0 1px 0 ${quietEdgeLight}, inset 0 0 0 ${config.borderWidth.B300} color-mix(in srgb, currentColor 22%, transparent)`;

/**
 * Composer-exact quiet action recipe: transparent at rest, a faint
 * `currentColor` tint plus edge light on hover, a stronger tint and inset
 * ring while `aria-pressed`/`aria-expanded`, and a primary-colored focus ring.
 * Pair with `fill="None"` on folds buttons so the resting control sits flat
 * on the reading plane. Text never receives depth.
 *
 * The 7% hover vs 12% pressed tints are below 3:1 against both theme
 * surfaces; selection therefore also uses the inset ring (and, on compact
 * icon toggles, a filled glyph). Hover of a pressed control keeps the inset
 * because the pressed shadow rule is declared after hover.
 */
export const quietActionButton = style({
  transition: 'background-color 140ms ease-out, color 140ms ease-out, box-shadow 140ms ease-out',
  selectors: {
    '&&': {
      backgroundColor: 'transparent',
      boxShadow: 'none',
    },
    '&&:not(:disabled):not([aria-disabled=true]):hover': {
      backgroundColor: 'color-mix(in srgb, currentColor 7%, transparent)',
      boxShadow: quietActionHoverShadow,
    },
    '&&:not(:disabled):not([aria-disabled=true])[aria-pressed=true], &&:not(:disabled):not([aria-disabled=true])[aria-expanded=true]':
      {
        backgroundColor: 'color-mix(in srgb, currentColor 12%, transparent)',
        boxShadow: quietActionPressedShadow,
      },
    '&&:not(:disabled):not([aria-disabled=true]):active': {
      backgroundColor: 'color-mix(in srgb, currentColor 12%, transparent)',
      boxShadow: `inset 0 1px 0 ${quietEdgeLight}`,
    },
    '&&:focus-visible': {
      outline: `2px solid ${color.Primary.Main}`,
      outlineOffset: '-2px',
    },
    '&&:disabled, &&[aria-disabled=true]': {
      backgroundColor: 'transparent',
      boxShadow: 'none',
    },
  },
  '@media': {
    '(prefers-reduced-motion: reduce)': {
      transition: 'none',
    },
    '(prefers-contrast: more)': {
      selectors: {
        '&&:not(:disabled):not([aria-disabled=true]):hover, &&:not(:disabled):not([aria-disabled=true]):active':
          {
            boxShadow: 'none',
          },
        '&&[aria-pressed=true], &&[aria-expanded=true]': {
          outline: '1px solid currentColor',
          outlineOffset: '-2px',
        },
        '&&:focus-visible': {
          outlineWidth: '2px',
        },
      },
    },
  },
});
