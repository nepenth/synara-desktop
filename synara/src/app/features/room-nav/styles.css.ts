import { style } from '@vanilla-extract/css';
import { color, config, toRem } from 'folds';

/**
 * Reading hierarchy (wave 3): the list recedes until something needs you.
 *
 * | state    | name                 | glyph     | weight |
 * | read     | secondary            | tertiary  | 400    |
 * | unread   | primary              | secondary | 600    |
 * | muted    | tertiary             | tertiary  | 400    |
 * | selected | Primary.OnContainer  | same      | 600    |
 *
 * Weights come from `NavItemContent`; colors are the opaque reading roles,
 * never opacity, so each holds AA on the room list and its hover tint.
 */

/**
 * Rows are flat: hover is the neutral surface tint, the selected row keeps the
 * Primary container fill from `NavItem` under the pointer too. Rows are 32px
 * so a long room list scans like a list, not a stack of tiles. An unread row
 * carries a 3×16px rounded gutter marker in the left padding (text never
 * moves); read and muted rows have none.
 */
export const RoomSurface = style({
  minHeight: toRem(32),
  backgroundColor: 'transparent',
  borderColor: 'transparent',
  boxShadow: 'none',
  selectors: {
    '&:not([data-selected=true]):hover, &:not([data-selected=true]):focus-within': {
      backgroundColor: color.Surface.ContainerHover,
    },
    '&[data-highlight=true]:not([data-muted])::before': {
      content: '',
      position: 'absolute',
      left: 0,
      top: '50%',
      width: toRem(3),
      height: toRem(16),
      marginTop: toRem(-8),
      borderRadius: config.radii.Pill,
      backgroundColor: 'var(--synara-content-primary)',
      pointerEvents: 'none',
    },
  },
});

const selectedRow = `${RoomSurface}[data-selected=true]`;

/**
 * The leading glyph column: `#`, a lock for non-public rooms, or a DM peer.
 * 18px wide so names start at one x whatever the glyph. Read rows draw it in
 * tertiary, one step below their name.
 */
export const RoomGlyph = style({
  width: toRem(18),
  flex: 'none',
  display: 'inline-flex',
  alignItems: 'center',
  justifyContent: 'center',
  color: 'var(--synara-content-tertiary)',
  fontSize: toRem(15),
  fontWeight: config.fontWeight.W600,
  lineHeight: 1,
  textAlign: 'center',
  selectors: {
    [`${selectedRow} &`]: {
      color: color.Primary.OnContainer,
    },
  },
});

export const RoomGlyphIcon = style({
  width: toRem(15),
  height: toRem(15),
});

/** A DM's peer in the `#` slot, at the glyph's size so rows keep their height. */
export const RoomPersonGlyph = style({
  width: toRem(18),
  height: toRem(18),
  fontSize: toRem(11),
});

/** Holds the DM glyph and its presence dot. */
export const RoomPersonGlyphSlot = style({
  position: 'relative',
  flex: 'none',
  display: 'inline-flex',
  width: toRem(18),
  height: toRem(18),
});

/**
 * DM presence: an 8px dot on the glyph's bottom-right, Success when online,
 * tertiary otherwise, absent while unknown. A 2px ring in the row's own fill
 * (rest, hover or selected) keeps it apart from the avatar.
 */
export const RoomPresenceDot = style({
  position: 'absolute',
  right: toRem(-2),
  bottom: toRem(-2),
  width: toRem(8),
  height: toRem(8),
  borderRadius: config.radii.Pill,
  boxShadow: `0 0 0 ${toRem(2)} ${color.Surface.Container}`,
  pointerEvents: 'none',
  selectors: {
    '&[data-presence=online]': {
      backgroundColor: color.Success.Main,
    },
    '&[data-presence=offline]': {
      backgroundColor: 'var(--synara-content-tertiary)',
    },
    [`${RoomSurface}:hover &, ${RoomSurface}:focus-within &`]: {
      boxShadow: `0 0 0 ${toRem(2)} ${color.Surface.ContainerHover}`,
    },
    [`${selectedRow} &`]: {
      boxShadow: `0 0 0 ${toRem(2)} ${color.Primary.Container}`,
    },
  },
});

/** Screen-reader text for the presence dot ("Online" / "Offline"). */
export const RoomPresenceLabel = style({
  position: 'absolute',
  width: 1,
  height: 1,
  margin: -1,
  padding: 0,
  overflow: 'hidden',
  clip: 'rect(0 0 0 0)',
  whiteSpace: 'nowrap',
  border: 0,
});

export const RoomName = style({
  minWidth: 0,
  overflow: 'hidden',
  color: 'var(--synara-content-primary)',
  letterSpacing: '-0.005em',
  selectors: {
    [`${selectedRow} &`]: {
      color: color.Primary.OnContainer,
    },
  },
});

/**
 * Read rooms recede through an opaque secondary colour, not opacity: primary at
 * 0.75 fell to 3.8:1 on a selected light row. Unread rooms are primary and W600.
 */
export const RoomNameRead = style({
  color: 'var(--synara-content-secondary)',
});

/** Muted rooms step back one more tone; the slashed bell names it. */
export const RoomNameMuted = style({
  color: 'var(--synara-content-tertiary)',
});

export const RoomGlyphUnread = style({
  color: 'var(--synara-content-secondary)',
});

export const RoomGlyphMuted = style({
  color: 'var(--synara-content-tertiary)',
});

/** Trailing state glyphs (favourite star, notification mode) in tertiary. */
export const RoomStateGlyph = style({
  flex: 'none',
  color: 'var(--synara-content-tertiary)',
  width: toRem(12),
  height: toRem(12),
  selectors: {
    [`${selectedRow} &`]: {
      color: color.Primary.OnContainer,
    },
  },
});
