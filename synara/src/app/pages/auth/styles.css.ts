import { style } from '@vanilla-extract/css';
import { color, config, toRem } from 'folds';

const twoColumn = `(min-width: ${toRem(920)})`;

const ink = (percent: number) =>
  `color-mix(in srgb, ${color.Background.OnContainer} ${percent}%, transparent)`;

export const AuthLayout = style({
  minHeight: '100%',
  backgroundColor: color.Background.Container,
  color: color.Background.OnContainer,
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
});

export const AuthStage = style({
  flexGrow: 1,
  width: '100%',
  maxWidth: toRem(1040),
  display: 'grid',
  gridTemplateColumns: `minmax(0, ${toRem(440)})`,
  justifyContent: 'center',
  alignContent: 'center',
  gap: config.space.S500,
  padding: `${toRem(40)} ${config.space.S400} ${config.space.S400}`,
  '@media': {
    [twoColumn]: {
      gridTemplateColumns: `minmax(0, 1fr) minmax(0, ${toRem(440)})`,
      justifyContent: 'stretch',
      alignItems: 'start',
      gap: toRem(72),
      padding: `${toRem(48)} ${toRem(48)} ${config.space.S400}`,
    },
  },
});

export const AuthBrand = style({
  display: 'flex',
  flexDirection: 'column',
  gap: config.space.S200,
  '@media': {
    [twoColumn]: { paddingTop: toRem(32) },
  },
});

/** Mark and wordmark side by side, as on the app icon's title bar. */
export const AuthLockup = style({
  display: 'flex',
  alignItems: 'center',
  gap: config.space.S300,
});

export const AuthBrandMark = style({
  width: toRem(40),
  height: toRem(40),
  flexShrink: 0,
});

export const AuthWordmark = style({
  margin: 0,
  fontSize: toRem(28),
  lineHeight: 1.2,
  fontWeight: 650,
  letterSpacing: '-0.02em',
});

export const AuthTagline = style({
  margin: 0,
  maxWidth: toRem(380),
  fontSize: toRem(15),
  lineHeight: 1.5,
  color: ink(72),
});

export const AuthCard = style({
  width: '100%',
  maxWidth: toRem(440),
  borderRadius: config.radii.R500,
  color: color.SurfaceVariant.OnContainer,
  backgroundColor: color.SurfaceVariant.Container,
  border: `${config.borderWidth.B300} solid ${color.SurfaceVariant.ContainerLine}`,
  '@media': {
    '(prefers-contrast: more)': {
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
