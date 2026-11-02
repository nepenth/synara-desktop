import React, { CSSProperties, ReactNode } from 'react';
import { Box, toRem } from 'folds';
import classNames from 'classnames';
import * as css from './UnreadBadge.css';

export type UnreadBadgeProps = {
  /**
   * Mention role (red): mentions and keywords, DMs, invites, pending
   * approvals. Otherwise the neutral role for plain unread counts.
   */
  highlight?: boolean;
  /** Digits to show; `0` renders the 8px dot instead of a count pill. */
  count: number;
  className?: string;
};
const styles: CSSProperties = {
  minWidth: toRem(18),
};
export function UnreadBadgeCenter({ children }: { children: ReactNode }) {
  return (
    <Box as="span" style={styles} shrink="No" alignItems="Center" justifyContent="Center">
      {children}
    </Box>
  );
}

/** Slack-style cap: a 16px pill cannot carry "1.2k" legibly. */
export const unreadBadgeLabel = (count: number): string => (count > 99 ? '99+' : String(count));

export function UnreadBadge({ highlight, count, className }: UnreadBadgeProps) {
  const role = highlight ? css.MentionRole : css.NeutralRole;
  if (count > 0) {
    return (
      <span
        className={classNames(css.CountBadge, role, className)}
        data-badge-role={highlight ? 'mention' : 'neutral'}
      >
        {unreadBadgeLabel(count)}
      </span>
    );
  }
  return (
    <span
      className={classNames(css.DotBadge, role, className)}
      data-badge-role={highlight ? 'mention' : 'neutral'}
      aria-hidden="true"
    />
  );
}
