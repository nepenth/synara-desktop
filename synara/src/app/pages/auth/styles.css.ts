import { keyframes, style } from '@vanilla-extract/css';
import { color, config, toRem } from 'folds';
import { floatingShadow, quietSurfaceFold } from '../../styles/Depth.css';

const reducedMotion = '(prefers-reduced-motion: reduce)';
const twoColumn = `(min-width: ${toRem(920)})`;

/** Accent glow derived from the active theme so every theme stays coherent. */
const accent = (percent: number) =>
  `color-mix(in srgb, ${color.Primary.Main} ${percent}%, transparent)`;
const ink = (percent: number) =>
  `color-mix(in srgb, ${color.Background.OnContainer} ${percent}%, transparent)`;

export const AuthLayout = style({
  minHeight: '100%',
  backgroundColor: color.Background.Container,
  color: color.Background.OnContainer,
  position: 'relative',
  isolation: 'isolate',
  overflow: 'hidden',
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
});

const breathe = keyframes({
  '0%, 100%': { opacity: 0.75, transform: 'scale(1)' },
  '50%': { opacity: 1, transform: 'scale(1.06)' },
});

/** Two soft accent fields behind everything. */
export const AuthAurora = style({
  position: 'absolute',
  inset: 0,
  zIndex: -2,
  pointerEvents: 'none',
  backgroundImage: [
    `radial-gradient(ellipse 55% 45% at 22% 30%, ${accent(26)}, transparent 70%)`,
    `radial-gradient(ellipse 45% 40% at 82% 78%, ${accent(16)}, transparent 70%)`,
  ].join(', '),
  animation: `${breathe} 14s ease-in-out infinite`,
  '@media': { [reducedMotion]: { animation: 'none' } },
});

/** A receding grid floor that fades toward the horizon. */
export const AuthGrid = style({
  position: 'absolute',
  left: '-25%',
  right: '-25%',
  bottom: '-12%',
  height: '62%',
  zIndex: -1,
  pointerEvents: 'none',
  backgroundImage: [
    `linear-gradient(${ink(9)} 1px, transparent 1px)`,
    `linear-gradient(90deg, ${ink(9)} 1px, transparent 1px)`,
  ].join(', '),
  backgroundSize: `${toRem(56)} ${toRem(56)}`,
  transform: 'perspective(700px) rotateX(62deg)',
  transformOrigin: 'center top',
  maskImage: 'linear-gradient(to bottom, transparent, black 30%, black 55%, transparent)',
  WebkitMaskImage: 'linear-gradient(to bottom, transparent, black 30%, black 55%, transparent)',
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
  '@media': {
    [twoColumn]: { alignItems: 'flex-start', textAlign: 'left', gap: config.space.S400 },
  },
});

const spin = keyframes({ to: { transform: 'rotate(360deg)' } });

/** Mark centered over the wordmark, whatever the column alignment. */
export const AuthLockup = style({
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  gap: config.space.S300,
});

export const AuthBrandMark = style({
  position: 'relative',
  width: toRem(92),
  height: toRem(92),
  // Brand-blue halo: the mark keeps its own color in every theme.
  '::before': {
    content: '""',
    position: 'absolute',
    inset: '-28%',
    zIndex: -1,
    borderRadius: '50%',
    background:
      'radial-gradient(circle, rgba(63, 184, 255, 0.28), rgba(47, 107, 255, 0.08) 45%, transparent 70%)',
  },
  '::after': {
    content: '""',
    position: 'absolute',
    inset: '-14%',
    zIndex: -1,
    borderRadius: '50%',
    border: '1px dashed rgba(63, 184, 255, 0.35)',
    animation: `${spin} 80s linear infinite`,
  },
  '@media': {
    [twoColumn]: { width: toRem(168), height: toRem(168), marginBottom: toRem(12) },
    [reducedMotion]: { selectors: { '&::after': { animation: 'none' } } },
  },
});

export const AuthWordmark = style({
  margin: 0,
  fontSize: toRem(34),
  // Room for the descender: background-clip text clips to the line box.
  lineHeight: 1.25,
  paddingBottom: '0.04em',
  fontWeight: 650,
  letterSpacing: '-0.02em',
  backgroundImage: `linear-gradient(100deg, ${color.Background.OnContainer} 30%, color-mix(in srgb, ${color.Primary.Main} 70%, ${color.Background.OnContainer}))`,
  WebkitBackgroundClip: 'text',
  backgroundClip: 'text',
  color: 'transparent',
  '@media': {
    [twoColumn]: { fontSize: toRem(56) },
    '(prefers-contrast: more)': { color: color.Background.OnContainer, backgroundImage: 'none' },
  },
});

export const AuthTagline = style({
  margin: 0,
  maxWidth: toRem(380),
  fontSize: toRem(16),
  lineHeight: 1.5,
  color: ink(72),
  '@media': { [twoColumn]: { fontSize: toRem(18) } },
});

export const AuthFeatures = style({
  display: 'none',
  listStyle: 'none',
  margin: 0,
  padding: 0,
  flexDirection: 'column',
  gap: config.space.S300,
  '@media': { [twoColumn]: { display: 'flex', marginTop: config.space.S200 } },
});

export const AuthFeature = style({
  display: 'flex',
  alignItems: 'center',
  gap: config.space.S300,
  fontSize: toRem(14),
  color: ink(80),
});

export const AuthFeatureIcon = style({
  display: 'grid',
  placeItems: 'center',
  width: toRem(30),
  height: toRem(30),
  flexShrink: 0,
  borderRadius: config.radii.R300,
  color: color.Primary.Main,
  backgroundColor: accent(12),
  boxShadow: `inset 0 0 0 1px ${accent(28)}`,
});

export const AuthCard = style({
  width: '100%',
  maxWidth: toRem(440),
  position: 'relative',
  borderRadius: toRem(20),
  color: color.SurfaceVariant.OnContainer,
  backgroundColor: `color-mix(in srgb, ${color.SurfaceVariant.Container} 88%, transparent)`,
  backgroundImage: quietSurfaceFold,
  border: `${config.borderWidth.B300} solid ${color.SurfaceVariant.ContainerLine}`,
  boxShadow: `${floatingShadow}, 0 0 0 1px ${accent(10)}, 0 ${toRem(24)} ${toRem(80)} ${accent(14)}`,
  backdropFilter: 'blur(22px) saturate(1.15)',
  WebkitBackdropFilter: 'blur(22px) saturate(1.15)',
  '::before': {
    content: '""',
    position: 'absolute',
    top: 0,
    left: toRem(28),
    right: toRem(28),
    height: '1px',
    background: `linear-gradient(90deg, transparent, ${accent(80)}, transparent)`,
  },
  '@media': {
    '(prefers-reduced-transparency: reduce)': {
      backgroundColor: color.SurfaceVariant.Container,
      backdropFilter: 'none',
      WebkitBackdropFilter: 'none',
    },
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
  boxShadow: `0 0 0 3px color-mix(in srgb, ${color.Success.Main} 22%, transparent)`,
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

// Mark motion
const nodePulse = keyframes({
  '0%, 70%, 100%': { opacity: 1 },
  '20%': { opacity: 0.45 },
});
const corePulse = keyframes({
  '0%, 100%': { opacity: 0.35 },
  '50%': { opacity: 0.85 },
});
const sweep = keyframes({
  '0%': { transform: 'translateX(-120px)' },
  '55%, 100%': { transform: 'translateX(170px)' },
});
const trace = keyframes({
  from: { strokeDashoffset: 100 },
  to: { strokeDashoffset: -100 },
});

export const MarkAnimated = style({ overflow: 'visible' });

export const MarkNode = style({
  selectors: {
    [`${MarkAnimated} &`]: { animation: `${nodePulse} 2.8s ease-in-out infinite` },
  },
  '@media': { [reducedMotion]: { animation: 'none !important' } },
});

export const MarkCore = style({
  opacity: 0.55,
  selectors: {
    [`${MarkAnimated} &`]: { animation: `${corePulse} 3.6s ease-in-out infinite` },
  },
  '@media': { [reducedMotion]: { animation: 'none !important' } },
});

export const MarkSheen = style({
  opacity: 0,
  selectors: {
    [`${MarkAnimated} &`]: { opacity: 1, animation: `${sweep} 6s ease-in-out infinite` },
  },
  '@media': { [reducedMotion]: { opacity: '0 !important', animation: 'none !important' } },
});

export const MarkOrbitTrace = style({
  stroke: '#D9FDFF',
  strokeDasharray: '18 82',
  strokeDashoffset: 100,
  opacity: 0,
  selectors: {
    [`${MarkAnimated} &`]: { opacity: 0.9, animation: `${trace} 4.5s linear infinite` },
  },
  '@media': { [reducedMotion]: { opacity: '0 !important', animation: 'none !important' } },
});
