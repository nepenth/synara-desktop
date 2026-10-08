import React, { KeyboardEvent, MutableRefObject, ReactNode, useId, useMemo, useState } from 'react';
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

const matchesFilter = <T,>(item: SettingsNavItem<T>, query: string): boolean => {
  if (!query) return true;
  const haystack = [item.name, ...(item.keywords ?? [])].join(' ').toLowerCase();
  return query
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean)
    .every((word) => haystack.includes(word));
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
};

/** Grouped, filterable settings navigation shared by App, Room and Space settings. */
export function SettingsNav<T>({
  header,
  headerAfter,
  groups,
  active,
  onSelect,
  footer,
}: SettingsNavProps<T>) {
  const [query, setQuery] = useState('');
  const filterId = useId();
  const visibleGroups = useMemo(
    () =>
      groups
        .map((group) => ({
          ...group,
          items: group.items.filter((item) => matchesFilter(item, query.trim())),
        }))
        .filter((group) => group.items.length > 0),
    [groups, query]
  );
  const total = groups.reduce((count, group) => count + group.items.length, 0);

  return (
    <PageNav size="300">
      <PageNavHeader outlined={false}>
        <Box grow="Yes" gap="200" alignItems="Center">
          {header}
        </Box>
        {headerAfter && <Box shrink="No">{headerAfter}</Box>}
      </PageNavHeader>
      {total > 5 && (
        <div className={css.NavFilter}>
          <Input
            id={filterId}
            size="300"
            radii="300"
            variant="Background"
            outlined
            value={query}
            onChange={(event) => setQuery(event.currentTarget.value)}
            placeholder="Search settings"
            aria-label="Search settings"
            before={<Icon src={Icons.Search} size="50" />}
          />
        </div>
      )}
      <Box grow="Yes" direction="Column">
        <PageNavContent>
          <nav aria-label="Settings sections">
            <div data-settings-nav>
              {visibleGroups.map((group) => (
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
              {visibleGroups.length === 0 && (
                <Text className={css.NavEmpty} size="T200" priority="300">
                  No settings match “{query.trim()}”.
                </Text>
              )}
            </div>
          </nav>
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
