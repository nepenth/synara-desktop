import { style } from '@vanilla-extract/css';
import { color, config, toRem } from 'folds';

/**
 * The leading glyph column: `#`, a lock for non-public rooms, or a DM peer.
 * 18px wide so names start at one x whatever the glyph. The glyph follows the
 * name: primary when unread, secondary when read, tertiary when muted.
 */
export const RoomGlyph = style({
  width: toRem(18),
  flex: 'none',
  display: 'inline-flex',
  alignItems: 'center',
  justifyContent: 'center',
  color: 'var(--synara-content-secondary)',
  fontSize: toRem(15),
  fontWeight: config.fontWeight.W600,
  lineHeight: 1,
  textAlign: 'center',
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

export const RoomName = style({
  color: 'var(--synara-content-primary)',
  letterSpacing: '-0.005em',
});

/**
 * Read rooms recede through an opaque secondary colour, not opacity: primary at
 * 0.75 fell to 3.8:1 on a selected light row. Unread rooms add weight instead.
 */
export const RoomNameRead = style({
  color: 'var(--synara-content-secondary)',
});

/** Muted rooms step back one more tone; the small bell-mute glyph names it. */
export const RoomNameMuted = style({
  color: 'var(--synara-content-tertiary)',
});

export const RoomGlyphUnread = style({
  color: 'var(--synara-content-primary)',
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
});

/**
 * Rows are flat: hover is the surface tint from `NavItem`, nothing else.
 * Rows are 32px so a long room list scans like a list, not a stack of tiles.
 */
export const RoomSurface = style({
  minHeight: toRem(32),
  backgroundColor: 'transparent',
  borderColor: 'transparent',
  boxShadow: 'none',
  selectors: {
    '&:hover, &:focus-within': {
      backgroundColor: color.Surface.ContainerHover,
    },
  },
});
