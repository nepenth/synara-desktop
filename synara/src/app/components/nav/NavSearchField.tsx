import React, { MouseEventHandler } from 'react';
import { Icon, Icons } from 'folds';
import { useTranslation } from 'react-i18next';
import * as css from './styles.css';

type NavSearchFieldProps = {
  /** Opens message search for this pane. */
  onOpen: MouseEventHandler<HTMLButtonElement>;
  /** True while that search page is the open route. */
  selected?: boolean;
  /** Field text; defaults to "Search messages". */
  label?: string;
};

/**
 * The pane's single search entry (Slack's search bar): a field-shaped button
 * under the pane header. Message search for Home or a space lives behind it;
 * the quick switcher keeps ⌘K.
 */
export function NavSearchField({ onOpen, selected, label }: NavSearchFieldProps) {
  const { t } = useTranslation();
  const text = label ?? t('modernization.nav.search_messages', 'Search messages');
  return (
    <div className={css.NavSearchFieldRow}>
      <button
        type="button"
        className={css.NavSearchField}
        onClick={onOpen}
        aria-current={selected ? 'page' : undefined}
        data-testid="nav-search-field"
      >
        <Icon size="100" src={Icons.Search} filled={selected} />
        <span className={css.NavSearchFieldLabel}>{text}</span>
      </button>
    </div>
  );
}
