import React, { MouseEventHandler, ReactNode } from 'react';
import classNames from 'classnames';
import { Icon, Icons } from 'folds';
import * as css from './styles.css';

type NavSectionHeaderProps = {
  /** Section title, a 12px semibold caption aligned with the room names. */
  label: ReactNode;
  /** Collapsed state; omit `onToggle` for a section that cannot collapse. */
  closed?: boolean;
  /** Receives the click; `categoryId` is exposed as `data-category-id`. */
  onToggle?: MouseEventHandler<HTMLButtonElement>;
  categoryId?: string;
  /** Hover-revealed controls on the right (sort menu, add). */
  actions?: ReactNode;
  /** Adds the group gap above; set on every section but the first. */
  spaced?: boolean;
  className?: string;
};

/**
 * One full-width 28px header per room-list section (Slack): an always-visible
 * chevron in the glyph column (turned when collapsed), the caption label, and
 * hover-revealed controls on the right.
 */
export function NavSectionHeader({
  label,
  closed,
  onToggle,
  categoryId,
  actions,
  spaced,
  className,
}: NavSectionHeaderProps) {
  const collapsible = typeof onToggle === 'function';
  return (
    <div
      className={classNames(css.NavSectionHeader, spaced && css.NavSectionHeaderSpaced, className)}
    >
      {collapsible ? (
        <button
          type="button"
          className={css.NavSectionToggle}
          onClick={onToggle}
          data-category-id={categoryId}
          aria-expanded={!closed}
        >
          <span className={css.NavSectionChevronSlot} aria-hidden="true">
            <Icon
              className={css.NavSectionChevron}
              data-closed={closed ? 'true' : undefined}
              size="50"
              src={Icons.ChevronBottom}
            />
          </span>
          <span className={css.NavSectionLabel} data-section-label="">
            {label}
          </span>
        </button>
      ) : (
        <div className={css.NavSectionStatic} role="heading" aria-level={2}>
          <span className={css.NavSectionLabel} data-section-label="">
            {label}
          </span>
        </div>
      )}
      {actions && <div className={css.NavSectionActions}>{actions}</div>}
    </div>
  );
}
