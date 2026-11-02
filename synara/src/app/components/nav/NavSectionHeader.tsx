import React, { MouseEventHandler, ReactNode } from 'react';
import classNames from 'classnames';
import { Icon, Icons } from 'folds';
import * as css from './styles.css';

type NavSectionHeaderProps = {
  /** Section title, 14px semibold in the row's glyph column. */
  label: ReactNode;
  /** Collapsed state; omit `onToggle` for a section that cannot collapse. */
  closed?: boolean;
  /** Receives the click; `categoryId` is exposed as `data-category-id`. */
  onToggle?: MouseEventHandler<HTMLButtonElement>;
  categoryId?: string;
  /** Hover-revealed controls on the right (sort menu, add). */
  actions?: ReactNode;
  className?: string;
};

/**
 * One full-width 28px header per room-list section (Slack): label, a chevron
 * that appears on hover or while collapsed, and hover-revealed controls.
 */
export function NavSectionHeader({
  label,
  closed,
  onToggle,
  categoryId,
  actions,
  className,
}: NavSectionHeaderProps) {
  const collapsible = typeof onToggle === 'function';
  return (
    <div className={classNames(css.NavSectionHeader, className)}>
      {collapsible ? (
        <button
          type="button"
          className={css.NavSectionToggle}
          onClick={onToggle}
          data-category-id={categoryId}
          aria-expanded={!closed}
        >
          <span className={css.NavSectionLabel}>{label}</span>
          <Icon
            className={css.NavSectionChevron}
            data-closed={closed ? 'true' : undefined}
            size="50"
            src={closed ? Icons.ChevronRight : Icons.ChevronBottom}
          />
        </button>
      ) : (
        <div className={css.NavSectionStatic} role="heading" aria-level={2}>
          <span className={css.NavSectionLabel}>{label}</span>
        </div>
      )}
      {actions && <div className={css.NavSectionActions}>{actions}</div>}
    </div>
  );
}
