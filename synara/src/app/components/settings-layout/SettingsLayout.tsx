import React, {
  KeyboardEvent,
  MutableRefObject,
  ReactNode,
  useId,
  useMemo,
  useRef,
  useState,
} from 'react';
import {
  Box,
  Icon,
  IconButton,
  Icons,
  IconSrc,
  Input,
  Modal,
  Overlay,
  OverlayBackdrop,
  OverlayCenter,
  Scroll,
  Text,
} from 'folds';
import classNames from 'classnames';
import FocusTrap from '../FocusTrap';
import { PageHeader, PageNav, PageNavContent, PageNavHeader } from '../page';
import { ContainerColor } from '../../styles/ContainerColor.css';
import { NavItem } from '../nav';
import { ScreenSize, useScreenSizeContext } from '../../hooks/useScreenSize';
import { stopPropagation } from '../../utils/keyboard';
import * as depthCss from '../../styles/Depth.css';
import * as css from './style.css';
import {
  matchRanges,
  matchSnippet,
  searchSettings,
  searchWords,
  SettingsSearchEntry,
  SettingsSearchResult,
} from './settingsSearch';

export * from './settingsSearch';

type SettingsModalProps = {
  requestClose: () => void;
  children: ReactNode;
};

/** The dialog every settings surface (App, Room, Space) opens in. */
export function SettingsModal({ requestClose, children }: SettingsModalProps) {
  return (
    <Overlay open backdrop={<OverlayBackdrop />}>
      <OverlayCenter>
        <FocusTrap
          focusTrapOptions={{
            initialFocus: false,
            clickOutsideDeactivates: true,
            onDeactivate: requestClose,
            escapeDeactivates: stopPropagation,
          }}
        >
          <Modal size="500" variant="Background" className={css.SettingsModal}>
            {children}
          </Modal>
        </FocusTrap>
      </OverlayCenter>
    </Overlay>
  );
}

export type SettingsNavItem<T> = {
  id: T;
  name: string;
  icon: IconSrc;
  /** Extra words the nav filter matches, e.g. the settings on that page. */
  keywords?: string[];
};

export type SettingsNavGroup<T> = {
  label: string;
  items: SettingsNavItem<T>[];
};

/** Wraps the parts of `text` that match the search words in a highlight. */
function Highlighted({ text, words }: { text: string; words: string[] }) {
  const ranges = matchRanges(text, words);
  if (ranges.length === 0) return <>{text}</>;
  const parts: ReactNode[] = [];
  let at = 0;
  ranges.forEach(([from, to]) => {
    if (from > at) parts.push(text.slice(at, from));
    parts.push(
      <mark key={from} className={css.SearchMatch}>
        {text.slice(from, to)}
      </mark>
    );
    at = to;
  });
  if (at < text.length) parts.push(text.slice(at));
  return <>{parts}</>;
}

/**
 * Scrolls the settings row or section titled `title` into view once its page
 * has rendered, then marks it briefly. Gives up quietly if it never appears.
 */
export const revealSetting = (title: string) => {
  if (typeof window === 'undefined') return;
  const reduceMotion = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
  let frames = 0;
  const attempt = () => {
    const target = Array.from(document.querySelectorAll<HTMLElement>('[data-setting-title]')).find(
      (element) => element.dataset.settingTitle === title
    );
    if (!target) {
      frames += 1;
      if (frames < 60) window.requestAnimationFrame(attempt);
      return;
    }
    target.scrollIntoView({ block: 'center', behavior: reduceMotion ? 'auto' : 'smooth' });
    target.classList.remove(css.RevealedSetting);
    // Restart the highlight when the same row is revealed twice in a row.
    target.getBoundingClientRect();
    target.classList.add(css.RevealedSetting);
    window.setTimeout(() => target.classList.remove(css.RevealedSetting), 1600);
  };
  window.requestAnimationFrame(attempt);
};

/** Moves focus between nav entries with the arrow, Home and End keys. */
const handleNavKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
  const keys = ['ArrowDown', 'ArrowUp', 'Home', 'End'];
  if (!keys.includes(event.key)) return;
  const nav = event.currentTarget.closest('[data-settings-nav]');
  if (!nav) return;
  const buttons = Array.from(
    nav.querySelectorAll<HTMLButtonElement>('button[data-settings-nav-item]')
  );
  if (buttons.length === 0) return;
  const current = buttons.indexOf(event.currentTarget);
  let next = current;
  if (event.key === 'ArrowDown') next = current < 0 ? 0 : Math.min(buttons.length - 1, current + 1);
  if (event.key === 'ArrowUp') next = current < 0 ? 0 : Math.max(0, current - 1);
  if (event.key === 'Home') next = 0;
  if (event.key === 'End') next = buttons.length - 1;
  event.preventDefault();
  buttons[next]?.focus();
};

type SettingsNavProps<T> = {
  /** Avatar and title shown at the top of the nav. */
  header: ReactNode;
  /** Trailing header control, e.g. the mobile close button. */
  headerAfter?: ReactNode;
  groups: SettingsNavGroup<T>[];
  active?: T;
  onSelect: (id: T) => void;
  footer?: ReactNode;
  /** Every setting the pages render, so search finds rows and not only pages. */
  searchIndex?: SettingsSearchEntry<T>[];
};

/** Search shows for long navigations, or wherever there are many settings to find. */
const SEARCH_MIN_NAV_ITEMS = 6;
const SEARCH_MIN_INDEX_ENTRIES = 8;

/** Grouped, filterable settings navigation shared by App, Room and Space settings. */
export function SettingsNav<T>({
  header,
  headerAfter,
  groups,
  active,
  onSelect,
  footer,
  searchIndex,
}: SettingsNavProps<T>) {
  const [query, setQuery] = useState('');
  const filterId = useId();
  const resultsRef = useRef<HTMLDivElement>(null);
  const pages = useMemo(() => groups.flatMap((group) => group.items), [groups]);
  const words = useMemo(() => searchWords(query), [query]);
  const searching = words.length > 0;
  const resultGroups = useMemo(
    () =>
      searching
        ? searchSettings(
            pages.map((item) => ({ id: item.id, name: item.name, keywords: item.keywords })),
            searchIndex ?? [],
            query
          )
        : [],
    [pages, searchIndex, query, searching]
  );
  const iconFor = (page: T) => pages.find((item) => item.id === page)?.icon ?? Icons.Setting;
  const showSearch =
    pages.length >= SEARCH_MIN_NAV_ITEMS || (searchIndex?.length ?? 0) >= SEARCH_MIN_INDEX_ENTRIES;

  const chooseResult = (result: SettingsSearchResult<T>) => {
    onSelect(result.page);
    if (result.title) revealSetting(result.title);
  };

  const handleSearchKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Escape' && query) {
      // Clear the search first; a second Escape closes the dialog.
      event.preventDefault();
      event.stopPropagation();
      setQuery('');
      return;
    }
    if (!searching) return;
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      resultsRef.current
        ?.querySelector<HTMLButtonElement>('button[data-settings-nav-item]')
        ?.focus();
      return;
    }
    if (event.key === 'Enter') {
      const first = resultGroups[0]?.results[0];
      if (first) {
        event.preventDefault();
        chooseResult(first);
      }
    }
  };

  return (
    <PageNav size="300">
      <PageNavHeader outlined={false}>
        <Box grow="Yes" gap="200" alignItems="Center">
          {header}
        </Box>
        {headerAfter && <Box shrink="No">{headerAfter}</Box>}
      </PageNavHeader>
      {showSearch && (
        <div className={css.NavFilter}>
          <Input
            id={filterId}
            size="300"
            radii="300"
            variant="Background"
            outlined
            value={query}
            onChange={(event) => setQuery(event.currentTarget.value)}
            onKeyDown={handleSearchKeyDown}
            placeholder="Search settings"
            aria-label="Search settings"
            before={<Icon src={Icons.Search} size="50" />}
          />
        </div>
      )}
      <Box grow="Yes" direction="Column">
        <PageNavContent>
          {searching ? (
            <div
              ref={resultsRef}
              role="region"
              aria-label="Settings search results"
              data-settings-nav
              data-settings-search-results
            >
              {resultGroups.map((group) => (
                <div
                  key={String(group.page)}
                  className={css.NavGroup}
                  role="group"
                  aria-label={group.pageName}
                >
                  <Text as="span" className={css.NavGroupLabel} aria-hidden>
                    {group.pageName}
                  </Text>
                  {group.results.map((result) => (
                    <NavItem
                      key={result.title ?? group.pageName}
                      variant="Background"
                      radii="400"
                      aria-selected={false}
                    >
                      <button
                        type="button"
                        className={css.SearchResultButton}
                        data-settings-nav-item
                        data-settings-search-result={result.title ?? group.pageName}
                        onKeyDown={handleNavKeyDown}
                        onClick={() => chooseResult(result)}
                      >
                        <Icon src={iconFor(group.page)} size="100" />
                        <span className={css.SearchResultCopy}>
                          <Text as="span" size="T300" truncate>
                            <Highlighted text={result.title ?? group.pageName} words={words} />
                          </Text>
                          {result.description && (
                            <Text as="span" size="T200" priority="300" truncate>
                              <Highlighted
                                text={matchSnippet(result.description, words)}
                                words={words}
                              />
                            </Text>
                          )}
                        </span>
                      </button>
                    </NavItem>
                  ))}
                </div>
              ))}
              {resultGroups.length === 0 && (
                <Text className={css.NavEmpty} size="T200" priority="300">
                  No settings match “{query.trim()}”.
                </Text>
              )}
            </div>
          ) : (
            <nav aria-label="Settings sections">
              <div data-settings-nav>
                {groups.map((group) => (
                  <div
                    key={group.label}
                    className={css.NavGroup}
                    role="group"
                    aria-label={group.label}
                  >
                    <Text as="span" className={css.NavGroupLabel} aria-hidden>
                      {group.label}
                    </Text>
                    {group.items.map((item) => {
                      const selected = active === item.id;
                      return (
                        <NavItem
                          key={item.name}
                          variant="Background"
                          radii="400"
                          aria-selected={selected}
                        >
                          <button
                            type="button"
                            className={css.NavItemButton}
                            data-settings-nav-item
                            onKeyDown={handleNavKeyDown}
                            aria-current={selected ? 'page' : undefined}
                            onClick={() => onSelect(item.id)}
                          >
                            <Icon src={item.icon} size="100" filled={selected} />
                            <Text as="span" size="T300" truncate>
                              {item.name}
                            </Text>
                          </button>
                        </NavItem>
                      );
                    })}
                  </div>
                ))}
              </div>
            </nav>
          )}
        </PageNavContent>
        {footer && <div className={css.NavFooter}>{footer}</div>}
      </Box>
    </PageNav>
  );
}

type SettingsPageProps = {
  title: ReactNode;
  description?: ReactNode;
  requestClose: () => void;
  /** Extra header controls placed before the close button. */
  headerAfter?: ReactNode;
  /** For pages that virtualize against their own scroll container. */
  scrollRef?: MutableRefObject<HTMLDivElement | null>;
  children: ReactNode;
};

/**
 * One settings page: a header bar with the title and close (Back on mobile),
 * then a centred readable column with the page description and its sections.
 */
export function SettingsPage({
  title,
  description,
  requestClose,
  headerAfter,
  scrollRef,
  children,
}: SettingsPageProps) {
  const mobile = useScreenSizeContext() === ScreenSize.Mobile;
  return (
    <Box grow="Yes" direction="Column" className={ContainerColor({ variant: 'Surface' })}>
      <PageHeader outlined={false}>
        <Box grow="Yes" alignItems="Center" gap="200">
          {mobile && (
            <IconButton
              className={depthCss.quietInteractiveSurface}
              onClick={requestClose}
              variant="Surface"
              fill="None"
              aria-label="Back"
            >
              <Icon src={Icons.ArrowLeft} />
            </IconButton>
          )}
          <Box grow="Yes">
            <Text as="h2" size="H3" truncate>
              {title}
            </Text>
          </Box>
          {headerAfter}
          {!mobile && (
            <IconButton
              className={depthCss.quietInteractiveSurface}
              onClick={requestClose}
              variant="Surface"
              fill="None"
              aria-label="Close"
            >
              <Icon src={Icons.Cross} />
            </IconButton>
          )}
        </Box>
      </PageHeader>
      <Box grow="Yes" className={css.PageScroll}>
        <Scroll ref={scrollRef} hideTrack visibility="Hover">
          <div className={css.PageColumn} data-settings-page>
            {description && (
              <div className={css.PageIntro}>
                <Text size="T300" priority="300">
                  {description}
                </Text>
              </div>
            )}
            {children}
          </div>
        </Scroll>
      </Box>
    </Box>
  );
}

type SettingsSectionProps = {
  title?: ReactNode;
  description?: ReactNode;
  /** `critical` marks a danger zone: destructive actions only. */
  tone?: 'default' | 'critical';
  /** Controls placed beside the section title (e.g. a section-wide action). */
  after?: ReactNode;
  className?: string;
  children: ReactNode;
};

/** A titled group of settings rendered as a single card with divided rows. */
export function SettingsSection({
  title,
  description,
  tone = 'default',
  after,
  className,
  children,
}: SettingsSectionProps) {
  const titleId = useId();
  const labelled = title !== undefined;
  return (
    <section
      className={classNames(css.Section, className)}
      aria-labelledby={labelled ? titleId : undefined}
      data-settings-section
      data-setting-title={typeof title === 'string' ? title : undefined}
    >
      {(title || description || after) && (
        <Box className={css.SectionHeader} alignItems="End" gap="200">
          <Box grow="Yes" direction="Column" gap="100">
            {title && (
              <Text
                as="h3"
                id={titleId}
                size="T300"
                className={classNames(
                  css.SectionTitle,
                  tone === 'critical' && css.SectionTitleCritical
                )}
              >
                {title}
              </Text>
            )}
            {description && (
              <Text size="T200" priority="300">
                {description}
              </Text>
            )}
          </Box>
          {after && <Box shrink="No">{after}</Box>}
        </Box>
      )}
      <div
        className={classNames(css.SectionCard({ tone }), css.SectionCardRows)}
        data-settings-section-card
      >
        {children}
      </div>
    </section>
  );
}
