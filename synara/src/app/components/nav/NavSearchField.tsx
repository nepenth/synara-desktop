import React from 'react';
import { Icon, Icons } from 'folds';
import { useAtom } from 'jotai';
import { useTranslation } from 'react-i18next';
import { searchModalAtom } from '../../state/searchModal';
import { KeySymbol } from '../../utils/key-symbol';
import { isMacOS } from '../../utils/user-agent';
import * as css from './styles.css';

/**
 * The pane's single search entry (Slack's search bar): a field-shaped button
 * under the pane header. It opens the global search (the quick switcher, also
 * ⌘K / Ctrl+K), which jumps to rooms, people and spaces and offers message
 * search, create, join and explore as actions. Since wave 3 it replaces the
 * rail's Search item, so every pane opens the same surface.
 */
export function NavSearchField() {
  const { t } = useTranslation();
  const [opened, setOpened] = useAtom(searchModalAtom);
  const shortcut = isMacOS() ? `${KeySymbol.Command}K` : 'Ctrl K';
  return (
    <div className={css.NavSearchFieldRow}>
      <button
        type="button"
        className={css.NavSearchField}
        onClick={() => setOpened(true)}
        aria-haspopup="dialog"
        aria-expanded={opened}
        aria-keyshortcuts={isMacOS() ? 'Meta+K' : 'Control+K'}
        data-testid="nav-search-field"
      >
        <Icon size="100" src={Icons.Search} filled={opened} />
        <span className={css.NavSearchFieldLabel}>{t('modernization.nav.search', 'Search')}</span>
        <kbd className={css.NavSearchFieldShortcut} aria-hidden="true">
          {shortcut}
        </kbd>
      </button>
    </div>
  );
}
