import { style } from '@vanilla-extract/css';
import { DefaultReset, color, config, toRem } from 'folds';

/**
 * The list column's frame: the title (space switcher) on top and the bottom
 * bar below. Type follows review-2: 15/600 pane title, 13 rows in menus, 11
 * bar labels; selected rows use Primary.Container with Primary.OnContainer at
 * 600 (wave 3).
 */

const focusRing = {
  outline: `${config.borderWidth.B600} solid ${color.Primary.Main}`,
  outlineOffset: `calc(-1 * ${config.borderWidth.B600})`,
};

/* ---------- Header: the switcher trigger ---------- */

export const SwitcherTrigger = style([
  DefaultReset,
  {
    flex: '0 1 auto',
    minWidth: 0,
    display: 'inline-flex',
    alignItems: 'center',
    gap: config.space.S200,
    height: toRem(32),
    padding: `0 ${config.space.S100}`,
    marginLeft: `calc(-1 * ${config.space.S100})`,
    border: 0,
    borderRadius: config.radii.R300,
    background: 'transparent',
    color: 'inherit',
    cursor: 'pointer',
    selectors: {
      '&:hover, &[aria-expanded=true]': {
        backgroundColor: color.Surface.ContainerHover,
      },
      '&:focus-visible': focusRing,
    },
  },
]);

/** The leading mark: Synara logo for Home, the space's avatar in a space. */
export const TriggerMark = style({
  flex: 'none',
  display: 'inline-flex',
  width: toRem(22),
  height: toRem(22),
  borderRadius: toRem(6),
  overflow: 'hidden',
});

export const TriggerChevron = style({
  position: 'relative',
  flex: 'none',
  display: 'inline-flex',
  color: 'var(--synara-content-secondary)',
});

/** Another space has unread: a dot on the chevron (red for attention). */
export const TriggerDot = style({
  position: 'absolute',
  top: toRem(-3),
  right: toRem(-4),
  display: 'inline-flex',
  pointerEvents: 'none',
});

/** The rest of the header row drags the window, as the header itself does. */
export const HeaderDragSpace = style({
  flexGrow: 1,
  alignSelf: 'stretch',
  minWidth: toRem(8),
});

/* ---------- Switcher menu ---------- */

export const SwitcherMenu = style({
  width: toRem(288),
  maxWidth: 'calc(100vw - 16px)',
  display: 'flex',
  flexDirection: 'column',
  maxHeight: 'min(70vh, 560px)',
});

export const SwitcherFilter = style({
  padding: `${config.space.S200} ${config.space.S200} 0`,
});

export const SwitcherList = style([
  DefaultReset,
  {
    listStyle: 'none',
    margin: 0,
    padding: config.space.S100,
    overflowY: 'auto',
    minHeight: 0,
    flexShrink: 1,
  },
]);

export const SwitcherItem = style({
  position: 'relative',
  selectors: {
    '&[data-drop-above=true]::after, &[data-drop-below=true]::after': {
      content: '',
      position: 'absolute',
      left: config.space.S100,
      right: config.space.S100,
      height: config.borderWidth.B700,
      borderRadius: config.radii.Pill,
      backgroundColor: color.Primary.Main,
      pointerEvents: 'none',
    },
    '&[data-drop-above=true]::after': { top: toRem(-1) },
    '&[data-drop-below=true]::after': { bottom: toRem(-1) },
    '&[data-dragging=true]': { opacity: 0.5 },
  },
});

export const SwitcherRow = style([
  DefaultReset,
  {
    width: '100%',
    display: 'flex',
    alignItems: 'center',
    gap: config.space.S200,
    minHeight: toRem(36),
    padding: `0 ${config.space.S200}`,
    border: 0,
    borderRadius: config.radii.R300,
    background: 'transparent',
    color: 'var(--synara-content-primary)',
    fontSize: toRem(13),
    lineHeight: toRem(18),
    textAlign: 'start',
    cursor: 'pointer',
    selectors: {
      '&:hover, &:focus-visible': {
        backgroundColor: color.Surface.ContainerHover,
      },
      '&:focus-visible': focusRing,
      '&[aria-current=page]': {
        backgroundColor: color.Primary.Container,
        color: color.Primary.OnContainer,
        fontWeight: config.fontWeight.W600,
      },
      '&[data-depth="1"]': {
        paddingLeft: toRem(28),
      },
    },
  },
]);

/** Space rows leave room on the right for the hover options button. */
export const SwitcherRowWithOptions = style({
  paddingRight: toRem(36),
});

export const SwitcherRowName = style({
  flexGrow: 1,
  minWidth: 0,
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
});

export const SwitcherRowUnread = style({
  fontWeight: config.fontWeight.W600,
});

export const SwitcherAvatar = style({
  flex: 'none',
  width: toRem(24),
  height: toRem(24),
});

export const SwitcherFolderChevron = style({
  flex: 'none',
  display: 'inline-flex',
  color: 'var(--synara-content-secondary)',
  transition: 'transform 120ms ease-out',
  selectors: {
    '[aria-expanded=false] > &': {
      transform: 'rotate(-90deg)',
    },
  },
  '@media': {
    '(prefers-reduced-motion: reduce)': {
      transition: 'none',
    },
  },
});

export const SwitcherOptions = style({
  position: 'absolute',
  top: '50%',
  right: config.space.S100,
  transform: 'translateY(-50%)',
  opacity: 0,
  selectors: {
    [`${SwitcherItem}:hover &, ${SwitcherItem}:focus-within &, &[aria-expanded=true]`]: {
      opacity: 1,
    },
  },
});

export const SwitcherSeparator = style({
  height: config.borderWidth.B300,
  margin: `${config.space.S100} ${config.space.S200}`,
  backgroundColor: color.Surface.ContainerLine,
});

export const SwitcherEmpty = style({
  padding: `${config.space.S200} ${config.space.S300}`,
  color: 'var(--synara-content-secondary)',
  fontSize: toRem(13),
});

export const SwitcherError = style({
  display: 'flex',
  alignItems: 'center',
  gap: config.space.S200,
  margin: `0 ${config.space.S100}`,
  padding: config.space.S200,
  borderRadius: config.radii.R300,
  color: color.Critical.OnContainer,
  backgroundColor: color.Critical.Container,
  fontSize: toRem(13),
});

export const SwitcherFooter = style({
  borderTop: `${config.borderWidth.B300} solid ${color.Surface.ContainerLine}`,
  padding: config.space.S100,
});

/* ---------- Bottom bar ---------- */

export const Bar = style([
  DefaultReset,
  {
    flexShrink: 0,
    display: 'flex',
    alignItems: 'stretch',
    gap: config.space.S100,
    padding: `${config.space.S100} ${config.space.S200}`,
    borderTop: `${config.borderWidth.B300} solid ${color.Surface.ContainerLine}`,
    backgroundColor: color.Surface.Container,
  },
]);

export const BarItem = style([
  DefaultReset,
  {
    position: 'relative',
    flex: '1 1 0',
    minWidth: 0,
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'center',
    gap: toRem(2),
    minHeight: toRem(40),
    padding: `${config.space.S100} 0`,
    border: 0,
    borderRadius: config.radii.R300,
    background: 'transparent',
    color: 'var(--synara-content-secondary)',
    cursor: 'pointer',
    selectors: {
      '&:hover': {
        backgroundColor: color.Surface.ContainerHover,
        color: 'var(--synara-content-primary)',
      },
      '&:focus-visible': focusRing,
      '&[aria-current=page]': {
        color: color.Primary.Main,
      },
      '&[aria-expanded=true]': {
        backgroundColor: color.Surface.ContainerActive,
      },
    },
  },
]);

/** The icon (or avatar) with its badge pinned to the top-right corner. */
export const BarIcon = style({
  position: 'relative',
  display: 'inline-flex',
  alignItems: 'center',
  justifyContent: 'center',
  width: toRem(24),
  height: toRem(24),
});

export const BarBadge = style({
  position: 'absolute',
  top: toRem(-4),
  left: '60%',
  display: 'inline-flex',
  pointerEvents: 'none',
  // Keeps the pill legible over the icon on any surface.
  borderRadius: config.radii.Pill,
  boxShadow: `0 0 0 ${toRem(2)} ${color.Surface.Container}`,
});

export const BarLabel = style({
  maxWidth: '100%',
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
  fontSize: toRem(11),
  lineHeight: toRem(14),
  fontWeight: config.fontWeight.W500,
  selectors: {
    '[aria-current=page] > &': {
      fontWeight: config.fontWeight.W600,
    },
  },
});

export const BarAvatar = style({
  width: toRem(24),
  height: toRem(24),
});

/* ---------- Manage spaces ---------- */

export const ManageList = style([
  DefaultReset,
  {
    listStyle: 'none',
    margin: 0,
    padding: 0,
    display: 'flex',
    flexDirection: 'column',
    gap: config.space.S100,
  },
]);

export const ManageRow = style({
  display: 'flex',
  alignItems: 'center',
  gap: config.space.S200,
  minHeight: toRem(36),
  padding: `0 ${config.space.S100}`,
  borderRadius: config.radii.R300,
  fontSize: toRem(13),
  selectors: {
    '&[data-depth="1"]': {
      paddingLeft: toRem(28),
    },
    '&:hover, &:focus-within': {
      backgroundColor: color.Surface.ContainerHover,
    },
  },
});

export const ManageSelect = style({
  height: toRem(32),
  padding: `0 ${config.space.S200}`,
  borderRadius: config.radii.R300,
  border: `${config.borderWidth.B300} solid ${color.Background.ContainerLine}`,
  backgroundColor: color.Background.Container,
  color: color.Background.OnContainer,
  fontSize: toRem(13),
  selectors: {
    '&:focus-visible': focusRing,
  },
});

export const ManageFolder = style({
  backgroundColor: color.SurfaceVariant.Container,
});
