import { globalStyle, keyframes, style } from '@vanilla-extract/css';
import { recipe, RecipeVariants } from '@vanilla-extract/recipes';
import { color, config, toRem } from 'folds';

/** Readable column shared by every App, Room and Space settings page. */
export const SETTINGS_CONTENT_MAX_WIDTH = 720;

/** The settings dialog: roomier than the generic 500 modal, never taller than the window. */
export const SettingsModal = style({
  width: `min(${toRem(1080)}, 94vw)`,
  height: `min(${toRem(800)}, 92vh)`,
  maxWidth: '94vw',
  maxHeight: '92vh',
  display: 'flex',
});

export const NavFilter = style({
  padding: `0 ${config.space.S200} ${config.space.S200}`,
});

export const NavGroup = style({
  display: 'flex',
  flexDirection: 'column',
  gap: toRem(2),
  selectors: {
    '& + &': {
      marginTop: config.space.S400,
    },
  },
});

export const NavGroupLabel = style({
  padding: `0 ${config.space.S300} ${config.space.S100}`,
  textTransform: 'uppercase',
  letterSpacing: '0.06em',
  fontSize: toRem(11),
  fontWeight: config.fontWeight.W600,
  color: color.Surface.OnContainer,
  opacity: 0.6,
});

export const NavItemButton = style({
  display: 'flex',
  alignItems: 'center',
  gap: config.space.S200,
  width: '100%',
  minHeight: toRem(36),
  padding: `0 ${config.space.S300}`,
  textAlign: 'start',
});

/** A search result: the setting's title over a muted description snippet. */
export const SearchResultButton = style({
  display: 'flex',
  alignItems: 'center',
  gap: config.space.S200,
  width: '100%',
  minHeight: toRem(44),
  padding: `${config.space.S100} ${config.space.S300}`,
  textAlign: 'start',
});

export const SearchResultCopy = style({
  display: 'flex',
  flexDirection: 'column',
  gap: toRem(2),
  minWidth: 0,
});

export const SearchMatch = style({
  backgroundColor: 'transparent',
  color: 'inherit',
  fontWeight: config.fontWeight.W600,
  textDecoration: 'underline',
  textDecorationColor: color.Primary.Main,
  textDecorationThickness: toRem(2),
  textUnderlineOffset: toRem(2),
});

const revealFade = keyframes({
  from: { backgroundColor: color.Primary.Container },
  to: { backgroundColor: 'transparent' },
});

/** Calm marker on the row a search result opened. */
export const RevealedSetting = style({
  borderRadius: config.radii.R300,
  animation: `${revealFade} 1400ms ease-out`,
  '@media': {
    '(prefers-reduced-motion: reduce)': {
      animation: 'none',
      outline: `${config.borderWidth.B600} solid ${color.Primary.Main}`,
      outlineOffset: toRem(2),
    },
  },
});

export const NavEmpty = style({
  padding: `${config.space.S300} ${config.space.S300}`,
});

export const NavFooter = style({
  padding: config.space.S200,
  borderTop: `${config.borderWidth.B300} solid ${color.Background.ContainerLine}`,
});

export const PageScroll = style({
  position: 'relative',
  flexGrow: 1,
  minHeight: 0,
});

/** Centred column: page intro, then sections, one rhythm for every page. */
export const PageColumn = style({
  width: '100%',
  maxWidth: toRem(SETTINGS_CONTENT_MAX_WIDTH),
  margin: '0 auto',
  padding: `${config.space.S200} ${config.space.S500} ${config.space.S700}`,
  boxSizing: 'border-box',
  display: 'flex',
  flexDirection: 'column',
  gap: config.space.S600,
  '@media': {
    'screen and (max-width: 720px)': {
      padding: `${config.space.S200} ${config.space.S300} ${config.space.S600}`,
    },
  },
});

export const PageIntro = style({
  display: 'flex',
  flexDirection: 'column',
  gap: config.space.S100,
});

export const Section = style({
  display: 'flex',
  flexDirection: 'column',
  gap: config.space.S200,
  minWidth: 0,
});

export const SectionHeader = style({
  padding: `0 ${config.space.S100}`,
});

export const SectionTitle = style({
  fontWeight: config.fontWeight.W600,
});

export const SectionTitleCritical = style({
  color: color.Critical.Main,
});

/**
 * One card per section. Rows inside it are flattened onto the card and divided
 * by hairlines, so a section reads as one group instead of a stack of cards.
 */
export const SectionCard = recipe({
  base: {
    display: 'flex',
    flexDirection: 'column',
    minWidth: 0,
    // The page sits on Surface; cards use SurfaceVariant, which is the
    // lighter of the two in both the light and dark themes.
    backgroundColor: color.SurfaceVariant.Container,
    color: color.SurfaceVariant.OnContainer,
    border: `${config.borderWidth.B300} solid ${color.SurfaceVariant.ContainerLine}`,
    borderRadius: config.radii.R400,
    overflow: 'hidden',
  },
  variants: {
    tone: {
      default: {},
      critical: {
        borderColor: color.Critical.ContainerLine,
      },
    },
  },
  defaultVariants: {
    tone: 'default',
  },
});
export type SectionCardVariants = RecipeVariants<typeof SectionCard>;

/** Marker class so the row rules below have a stable, unique selector. */
export const SectionCardRows = style({});

// Rows (setting cards placed in a section) sit flat on the section card.
globalStyle(`${SectionCardRows}${SectionCardRows} [data-sequence-card]`, {
  backgroundColor: 'transparent',
  borderWidth: 0,
  borderRadius: 0,
  boxShadow: 'none',
});

globalStyle(`${SectionCardRows}${SectionCardRows} > *`, {
  paddingLeft: config.space.S400,
  paddingRight: config.space.S400,
});

globalStyle(`${SectionCardRows}${SectionCardRows} > [data-sequence-card]`, {
  paddingTop: config.space.S300,
  paddingBottom: config.space.S300,
});

// Hairline between consecutive rows, at the section level and inside lists.
globalStyle(`${SectionCardRows} > * + *`, {
  borderTop: `${config.borderWidth.B300} solid ${color.SurfaceVariant.ContainerLine}`,
});

globalStyle(`${SectionCardRows} [data-sequence-card] + [data-sequence-card]`, {
  borderTopWidth: config.borderWidth.B300,
  borderTopStyle: 'solid',
  borderTopColor: color.SurfaceVariant.ContainerLine,
});

// Plain text or status lines placed directly in a section get row padding.
globalStyle(`${SectionCardRows} > :not([data-sequence-card])`, {
  paddingTop: config.space.S300,
  paddingBottom: config.space.S300,
});
