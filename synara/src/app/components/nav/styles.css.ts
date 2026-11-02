import { ComplexStyleRule, createVar, style } from '@vanilla-extract/css';
import { RecipeVariants, recipe } from '@vanilla-extract/recipes';
import { ContainerColor, DefaultReset, Disabled, RadiiVariant, color, config, toRem } from 'folds';

export const NavCategory = style([
  DefaultReset,
  {
    position: 'relative',
  },
]);

export const NavCategoryHeader = style({
  gap: config.space.S100,
});

export const NavLink = style({
  color: 'inherit',
  minWidth: 0,
  display: 'flex',
  alignItems: 'center',
  cursor: 'pointer',
  flexGrow: 1,
  ':hover': {
    textDecoration: 'unset',
  },
  ':focus': {
    outline: 'none',
  },
});

const Container = createVar();
const ContainerHover = createVar();
const ContainerActive = createVar();
const ContainerLine = createVar();
const OnContainer = createVar();

const getVariant = (variant: ContainerColor): ComplexStyleRule => ({
  vars: {
    [Container]: color[variant].Container,
    [ContainerHover]: color[variant].ContainerHover,
    [ContainerActive]: color[variant].ContainerActive,
    [ContainerLine]: color[variant].ContainerLine,
    [OnContainer]: color[variant].OnContainer,
  },
});

/**
 * Rows are flat (Slack): hover is a surface tint, the selected row is the
 * stronger selected tint plus the 3px accent bar. No border or shadow, so a
 * dense list never "lifts" under the pointer. `prefers-contrast: more` adds a
 * solid edge to the selected row instead.
 */
const NavItemBase = style({
  position: 'relative',
  width: '100%',
  display: 'flex',
  justifyContent: 'start',
  cursor: 'pointer',
  backgroundColor: 'transparent',
  color: OnContainer,
  outline: 'none',
  minHeight: toRem(32),
  border: `${config.borderWidth.B300} solid transparent`,
  boxShadow: 'none',
  transition: 'background-color 140ms ease-out',

  selectors: {
    '&::before': {
      content: '',
      position: 'absolute',
      left: 0,
      top: toRem(7),
      bottom: toRem(7),
      width: toRem(3),
      borderRadius: `0 ${toRem(3)} ${toRem(3)} 0`,
      background: color.Primary.Main,
      opacity: 0,
    },
    '&:hover, &:focus-visible': {
      backgroundColor: ContainerHover,
    },
    '&[data-hover=true]': {
      backgroundColor: ContainerHover,
    },
    [`&:has(.${NavLink}:active)`]: {
      backgroundColor: ContainerActive,
    },
    '&[data-selected=true]': {
      backgroundColor: 'var(--synara-selected-surface)',
    },
    '&[data-selected=true]::before': {
      opacity: 1,
    },
    // Primary, not ContainerLine: the line tone matches the hover fill (~1.2:1).
    [`&:has(.${NavLink}:focus-visible)`]: {
      outline: `${config.borderWidth.B600} solid ${color.Primary.Main}`,
      outlineOffset: `calc(-1 * ${config.borderWidth.B600})`,
    },
  },
  '@media': {
    '(prefers-reduced-motion: reduce)': {
      transition: 'none',
    },
    '(prefers-contrast: more)': {
      selectors: {
        '&[data-selected=true]': {
          borderColor: 'var(--synara-depth-contrast-strong-edge)',
        },
      },
    },
  },
  '@supports': {
    [`not selector(:has(.${NavLink}:focus-visible))`]: {
      ':focus-within': {
        outline: `${config.borderWidth.B600} solid ${color.Primary.Main}`,
        outlineOffset: `calc(-1 * ${config.borderWidth.B600})`,
      },
    },
  },
});
export const NavItem = recipe({
  base: [DefaultReset, NavItemBase, Disabled],
  variants: {
    variant: {
      Background: getVariant('Background'),
      Surface: getVariant('Surface'),
      SurfaceVariant: getVariant('SurfaceVariant'),
      Primary: getVariant('Primary'),
      Secondary: getVariant('Secondary'),
      Success: getVariant('Success'),
      Warning: getVariant('Warning'),
      Critical: getVariant('Critical'),
    },
    radii: RadiiVariant,
  },
  defaultVariants: {
    variant: 'Surface',
    radii: '400',
  },
});

export type RoomSelectorVariants = RecipeVariants<typeof NavItem>;

/**
 * One type scale with the timeline: 15px names on a 20px line, read rows W500,
 * unread rows W700 (the sender-name weight). Rows stay 32px.
 */
export const NavItemContent = style({
  paddingLeft: config.space.S200,
  paddingRight: config.space.S200,
  height: 'inherit',
  minWidth: 0,
  flexGrow: 1,
  display: 'flex',
  alignItems: 'center',
  fontSize: toRem(15),
  lineHeight: toRem(20),
  fontWeight: config.fontWeight.W500,

  selectors: {
    '&:hover': {
      textDecoration: 'unset',
    },
    [`.${NavItemBase}[data-highlight=true] &, .${NavItemBase}[data-selected=true] &`]: {
      fontWeight: config.fontWeight.W700,
    },
  },
});

/**
 * The row's trailing control slot (kebab). Always laid out at 28px so names
 * truncate at one width; revealed by hover, focus within the row, or an open
 * menu. Badges and glyphs sit beside it and never unmount.
 */
export const NavItemOptions = style({
  width: toRem(28),
  paddingRight: config.space.S100,
  justifyContent: 'flex-end',
  opacity: 0,
  transition: 'opacity 140ms ease-out',
  selectors: {
    [`.${NavItemBase}:hover &, .${NavItemBase}:focus-within &, .${NavItemBase}[data-hover=true] &, &:has([aria-expanded=true])`]:
      {
        opacity: 1,
      },
  },
  '@media': {
    '(prefers-reduced-motion: reduce)': {
      transition: 'none',
    },
  },
});

/**
 * Section header: one full-width 28px row per section. The label sits in the
 * row's glyph column; the collapse chevron and the right-hand controls are
 * hover-revealed (or shown while collapsed / while a menu is open).
 */
export const NavSectionHeader = style({
  position: 'relative',
  display: 'flex',
  alignItems: 'center',
  minHeight: toRem(28),
  paddingLeft: config.space.S100,
  paddingRight: config.space.S100,
  gap: config.space.S100,
  borderRadius: config.radii.R400,
  color: 'var(--synara-content-secondary)',
});

export const NavSectionToggle = style([
  DefaultReset,
  {
    display: 'flex',
    alignItems: 'center',
    gap: config.space.S100,
    flexGrow: 1,
    minWidth: 0,
    height: toRem(28),
    padding: `0 ${config.space.S100}`,
    border: 'none',
    background: 'transparent',
    color: 'inherit',
    cursor: 'pointer',
    borderRadius: config.radii.R400,
    textAlign: 'left',
    selectors: {
      '&:hover, &:focus-visible': {
        backgroundColor: color.Surface.ContainerHover,
        color: 'var(--synara-content-primary)',
      },
      '&:focus-visible': {
        outline: `${config.borderWidth.B600} solid ${color.Primary.Main}`,
        outlineOffset: `calc(-1 * ${config.borderWidth.B600})`,
      },
    },
  },
]);

/** A section title that cannot collapse: same metrics, no hover tint. */
export const NavSectionStatic = style([
  DefaultReset,
  {
    display: 'flex',
    alignItems: 'center',
    flexGrow: 1,
    minWidth: 0,
    height: toRem(28),
    padding: `0 ${config.space.S100}`,
    color: 'inherit',
  },
]);

export const NavSectionLabel = style({
  fontSize: toRem(14),
  lineHeight: toRem(20),
  fontWeight: config.fontWeight.W600,
  letterSpacing: '-0.005em',
  minWidth: 0,
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
});

export const NavSectionChevron = style({
  flexShrink: 0,
  opacity: 0,
  transition: 'opacity 140ms ease-out',
  selectors: {
    [`.${NavSectionHeader}:hover &, .${NavSectionHeader}:focus-within &, &[data-closed=true]`]: {
      opacity: 1,
    },
  },
  '@media': {
    '(prefers-reduced-motion: reduce)': {
      transition: 'none',
    },
  },
});

export const NavSectionActions = style({
  display: 'flex',
  alignItems: 'center',
  gap: config.space.S100,
  flexShrink: 0,
  opacity: 0,
  transition: 'opacity 140ms ease-out',
  selectors: {
    [`.${NavSectionHeader}:hover &, .${NavSectionHeader}:focus-within &, &:has([aria-expanded=true])`]:
      {
        opacity: 1,
      },
  },
  '@media': {
    '(prefers-reduced-motion: reduce)': {
      transition: 'none',
    },
  },
});

/** 28px square control inside a section header or the pane header. */
export const NavSmallIconButton = style({
  width: `${toRem(28)} !important`,
  height: `${toRem(28)} !important`,
  minWidth: `${toRem(28)} !important`,
  minHeight: `${toRem(28)} !important`,
  padding: '0 !important',
});

export const NavSmallIcon = style({
  width: `${toRem(14)} !important`,
  height: `${toRem(14)} !important`,
  minWidth: `${toRem(14)} !important`,
  minHeight: `${toRem(14)} !important`,
});

/**
 * The pane's one search entry: a 32px field-shaped button under the header
 * that opens message search for this pane (Slack's search bar).
 */
export const NavSearchField = style([
  DefaultReset,
  {
    display: 'flex',
    alignItems: 'center',
    gap: config.space.S200,
    width: '100%',
    height: toRem(32),
    padding: `0 ${config.space.S300}`,
    border: `${config.borderWidth.B300} solid ${color.Surface.ContainerLine}`,
    borderRadius: config.radii.R400,
    backgroundColor: color.Background.Container,
    color: 'var(--synara-content-tertiary)',
    fontSize: toRem(13),
    lineHeight: toRem(18),
    fontWeight: config.fontWeight.W500,
    cursor: 'pointer',
    textAlign: 'left',
    transition: 'background-color 140ms ease-out, border-color 140ms ease-out',
    selectors: {
      '&:hover, &:focus-visible': {
        backgroundColor: color.Background.ContainerHover,
        borderColor: color.Surface.ContainerLine,
        color: 'var(--synara-content-secondary)',
      },
      '&:focus-visible': {
        outline: `${config.borderWidth.B600} solid ${color.Primary.Main}`,
        outlineOffset: `calc(-1 * ${config.borderWidth.B600})`,
      },
      '&[aria-current=page]': {
        borderColor: color.Primary.Main,
        color: 'var(--synara-content-primary)',
      },
    },
    '@media': {
      '(prefers-reduced-motion: reduce)': {
        transition: 'none',
      },
    },
  },
]);

export const NavSearchFieldRow = style({
  padding: `${config.space.S200} ${config.space.S200} 0 ${config.space.S200}`,
  flexShrink: 0,
});

export const NavSearchFieldLabel = style({
  flexGrow: 1,
  minWidth: 0,
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
});
