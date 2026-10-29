import { style } from '@vanilla-extract/css';
import { color, config, toRem } from 'folds';
import { floatingShadow, quietSurfaceFold } from '../../styles/Depth.css';

const twoColumn = `(min-width: ${toRem(920)})`;

const ink = (percent: number) =>
  `color-mix(in srgb, ${color.Background.OnContainer} ${percent}%, transparent)`;

/** Accent tint from the active theme, so every theme stays coherent. */
const accent = (percent: number) =>
  `color-mix(in srgb, ${color.Primary.Main} ${percent}%, transparent)`;

export const AuthLayout = style({
  minHeight: '100%',
  backgroundColor: color.Background.Container,
  color: color.Background.OnContainer,
  position: 'relative',
  isolation: 'isolate',
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
});

/** One still, faint accent field behind the brand column. */
export const AuthField = style({
  position: 'absolute',
  inset: 0,
  zIndex: -1,
  pointerEvents: 'none',
  backgroundImage: `radial-gradient(ellipse 50% 45% at 28% 46%, ${accent(10)}, transparent 70%)`,
  '@media': {
    '(prefers-contrast: more)': { display: 'none' },
  },
});

export const AuthStage = style({
  flexGrow: 1,
  width: '100%',
  maxWidth: toRem(1040),
  display: 'grid',
  gridTemplateColumns: 'minmax(0, 1fr)',
  justifyItems: 'center',
  alignContent: 'center',
  gap: config.space.S500,
  padding: `${toRem(40)} ${config.space.S400} ${config.space.S400}`,
  '@media': {
    [twoColumn]: {
      gridTemplateColumns: `minmax(0, 1fr) minmax(0, ${toRem(440)})`,
      alignItems: 'center',
      justifyItems: 'stretch',
      gap: toRem(72),
      padding: `${toRem(48)} ${toRem(48)} ${config.space.S400}`,
    },
  },
});

export const AuthBrand = style({
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  textAlign: 'center',
  gap: config.space.S300,
  // The brand block stays centered in its panel in both layouts: it is the
  // screen's focal brand moment.
  '@media': {
    [twoColumn]: { gap: config.space.S400 },
  },
});

/** Mark centered over the wordmark, 16px apart as on Welcome and iOS. */
export const AuthLockup = style({
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  gap: config.space.S400,
});

export const AuthBrandMark = style({
  // The mark's own soft glow is its only light; no halo behind it.
  width: toRem(96),
  height: toRem(96),
  '@media': {
    [twoColumn]: { width: toRem(128), height: toRem(128) },
  },
});

export const AuthWordmark = style({
  margin: 0,
  fontSize: toRem(32),
  lineHeight: 1.2,
  fontWeight: 650,
  letterSpacing: '-0.02em',
  '@media': { [twoColumn]: { fontSize: toRem(44) } },
});

export const AuthTagline = style({
  margin: 0,
  maxWidth: toRem(380),
  fontSize: toRem(16),
  lineHeight: 1.5,
  color: ink(72),
  '@media': { [twoColumn]: { fontSize: toRem(17) } },
});

export const AuthFeatures = style({
  display: 'none',
  listStyle: 'none',
  margin: 0,
  padding: 0,
  flexDirection: 'column',
  gap: config.space.S200,
  // A centered block whose lines stay left-aligned, so the glyphs line up.
  textAlign: 'left',
  '@media': { [twoColumn]: { display: 'flex' } },
});

export const AuthFeature = style({
  display: 'flex',
  alignItems: 'center',
  gap: config.space.S300,
  fontSize: toRem(14),
  color: ink(78),
});

/** A plain glyph in the accent, with no tile behind it. */
export const AuthFeatureIcon = style({
  display: 'inline-flex',
  flexShrink: 0,
  color: color.Primary.Main,
});

export const AuthCard = style({
  width: '100%',
  maxWidth: toRem(440),
  borderRadius: toRem(16),
  color: color.SurfaceVariant.OnContainer,
  backgroundColor: color.SurfaceVariant.Container,
  backgroundImage: quietSurfaceFold,
  border: `${config.borderWidth.B300} solid ${color.SurfaceVariant.ContainerLine}`,
  boxShadow: floatingShadow,
  '@media': {
    '(prefers-contrast: more)': {
      backgroundImage: 'none',
      boxShadow: 'none',
      borderColor: 'var(--synara-depth-contrast-strong-edge)',
    },
  },
});

export const AuthCardContent = style({
  display: 'flex',
  flexDirection: 'column',
  gap: config.space.S500,
  padding: `${toRem(32)} ${toRem(32)} ${toRem(28)}`,
  '@media': {
    [`(max-width: ${toRem(480)})`]: { padding: `${toRem(24)} ${toRem(20)}` },
  },
});

export const AuthNotice = style({
  padding: `${config.space.S200} ${config.space.S300}`,
  borderRadius: config.radii.R400,
  backgroundColor: `color-mix(in srgb, ${color.Warning.Main} 14%, transparent)`,
  color: color.SurfaceVariant.OnContainer,
});

/** One fixed-height line, so status changes never move the form below. */
export const ServerStatus = style({
  display: 'flex',
  alignItems: 'center',
  gap: config.space.S200,
  minHeight: toRem(20),
  minWidth: 0,
  fontSize: toRem(12.5),
  color: ink(62),
});

export const ServerStatusText = style({
  minWidth: 0,
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
});

export const ServerStatusError = style({ color: color.Critical.Main });

export const ServerStatusDot = style({
  width: toRem(7),
  height: toRem(7),
  flexShrink: 0,
  borderRadius: '50%',
  backgroundColor: color.Success.Main,
});

export const ServerStatusRetry = style({
  flexShrink: 0,
  marginLeft: 'auto',
  padding: 0,
  border: 0,
  background: 'none',
  color: color.Primary.Main,
  font: 'inherit',
  fontWeight: 600,
  cursor: 'pointer',
  selectors: { '&:hover': { textDecoration: 'underline' } },
});

export const AuthLoading = style({
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  gap: config.space.S200,
  minHeight: toRem(220),
  color: ink(62),
});

export const AuthHeading = style({
  display: 'flex',
  flexDirection: 'column',
  gap: config.space.S100,
});

export const AuthDivider = style({
  height: '1px',
  border: 0,
  margin: 0,
  backgroundColor: color.SurfaceVariant.ContainerLine,
});

export const AuthFooter = style({
  padding: `${config.space.S300} ${config.space.S400} ${config.space.S400}`,
  color: ink(60),
});
