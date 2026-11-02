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
 * Rows are flat (Slack): hover is a neutral surface tint; the selected row is
 * anchored by the Primary container fill with its on-container text, clearly
 * stronger than hover (wave 3), and keeps that fill under the pointer. No
 * border or shadow, so a dense list never "lifts". `prefers-contrast: more`
 * adds a solid edge to the selected row.
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
      backgroundColor: color.Primary.Container,
      color: color.Primary.OnContainer,
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
 * One type scale with the timeline: 15px names on a 20px line. Read rows are
 * W400 so they recede; unread and selected rows are W600. Rows stay 32px.
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
  fontWeight: config.fontWeight.W400,

  selectors: {
    '&:hover': {
      textDecoration: 'unset',
    },
    [`.${NavItemBase}[data-highlight=true] &, .${NavItemBase}[data-selected=true] &`]: {
      fontWeight: config.fontWeight.W600,
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
 * Section header: one full-width 28px caption row per section. The chevron
 * sits in the rows' glyph column and is always shown (it turns when the
 * section collapses); the label lines up with the room names. The right-hand
 * controls (sort) are hover-revealed, or shown while their menu is open.
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

/** Space above every section but the first, so groups read as groups. */
export const NavSectionHeaderSpaced = style({
  marginTop: config.space.S500,
});

export const NavSectionToggle = style([
  DefaultReset,
  {
    display: 'flex',
    alignItems: 'center',
    gap: config.space.S200,
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

/** Caption style: 12px semibold, sentence case, a touch of tracking. */
export const NavSectionLabel = style({
  fontSize: toRem(12),
  lineHeight: toRem(16),
  fontWeight: config.fontWeight.W600,
  letterSpacing: '0.02em',
  minWidth: 0,
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
});

/** The rows' 18px glyph column, so the chevron centres over the glyphs. */
export const NavSectionChevronSlot = style({
  width: toRem(18),
  flexShrink: 0,
  display: 'inline-flex',
  alignItems: 'center',
  justifyContent: 'center',
});

export const NavSectionChevron = style({
  flexShrink: 0,
  transition: 'transform 140ms ease-out',
  selectors: {
    '&[data-closed=true]': {
      transform: 'rotate(-90deg)',
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
      '&[aria-expanded=true]': {
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

/** The ⌘K hint at the field's end, tertiary like the placeholder. */
export const NavSearchFieldShortcut = style({
  flexShrink: 0,
  fontFamily: 'inherit',
  fontSize: toRem(11),
  lineHeight: toRem(16),
  color: 'var(--synara-content-tertiary)',
});

/**
 * The pane header's title (the homeserver or space name): 15px semibold, so a
 * typical host name fits, middle-truncated so its end stays readable.
 */
export const NavPaneTitle = style({
  flex: '0 1 auto',
  minWidth: 0,
  fontSize: toRem(15),
  lineHeight: toRem(20),
  fontWeight: config.fontWeight.W600,
  color: 'var(--synara-content-heading)',
});

export const NavSearchFieldLabel = style({
  flexGrow: 1,
  minWidth: 0,
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
});
