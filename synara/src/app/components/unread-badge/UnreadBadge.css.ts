import { style } from '@vanilla-extract/css';
import { color, config, toRem } from 'folds';

/**
 * Attention roles for the room list and rail. The count pill is 18px with
 * 11px bold digits (Slack's proportions); the dot is 8px. Both roles are
 * opaque fills, so they read the same on hover and selected rows.
 */
const pill = {
  display: 'inline-flex',
  alignItems: 'center',
  justifyContent: 'center',
  boxSizing: 'border-box' as const,
  borderRadius: config.radii.Pill,
  flexShrink: 0,
};

export const CountBadge = style({
  ...pill,
  height: toRem(18),
  minWidth: toRem(18),
  padding: `0 ${toRem(5)}`,
  fontSize: toRem(11),
  lineHeight: 1,
  fontWeight: config.fontWeight.W700,
  letterSpacing: '0.01em',
  fontVariantNumeric: 'tabular-nums',
});

export const DotBadge = style({
  ...pill,
  width: toRem(8),
  height: toRem(8),
});

/** Mentions, DMs, invites, pending approvals: the one red in the shell. */
export const MentionRole = style({
  backgroundColor: color.Critical.Main,
  color: color.Critical.OnMain,
});

/** Plain unread counts (Inbox, DM rows): neutral so red stays for attention. */
export const NeutralRole = style({
  backgroundColor: color.Secondary.Main,
  color: color.Secondary.OnMain,
});
