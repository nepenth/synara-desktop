import React, { ReactNode, useMemo, useRef, useState } from 'react';
import { Box, Icon, Icons, PopOut, RectCords } from 'folds';
import { useAtomValue } from 'jotai';
import FocusTrap from '../../components/FocusTrap';
import { PageNavHeader } from '../../components/page';
import { NavPaneTitle } from '../../components/nav';
import { UnreadBadge } from '../../components/unread-badge';
import { roomToUnreadAtom } from '../../state/room/roomToUnread';
import { useSelectedSpace } from '../../hooks/router/useSelectedSpace';
import { useHomeSelected } from '../../hooks/router/useHomeSelected';
import { useDirectSelected } from '../../hooks/router/useDirectSelected';
import { useSpaceOrder } from './useSpaceOrder';
import { useNavPaneNavigation } from './useNavPaneNavigation';
import { SpaceSwitcherMenu } from './SpaceSwitcher';
import { ManageSpacesDialog } from './ManageSpacesDialog';
import { otherSpacesUnread } from './switcherUnread';
import * as css from './NavPane.css';

type NavPaneHeaderProps = {
  /** The pane's title: the homeserver, a space, "Activity" or "Explore". */
  title: string;
  /** Leading mark: the Synara logo on Home, the space avatar in a space. */
  mark?: ReactNode;
  /** After the title, e.g. a lock on a private space. */
  titleAfter?: ReactNode;
  /** The pane's own buttons and their menus: "+" on Home, ⋮ on Home and spaces. */
  actions?: ReactNode;
};

/**
 * The list column's header (N1): one frame for Home, spaces, Activity and
 * Explore. The title opens the space switcher (N3); a dot on its chevron says
 * another space has unread. The header drags the window where nothing is
 * clickable.
 */
export function NavPaneHeader({ title, mark, titleAfter, actions }: NavPaneHeaderProps) {
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [anchor, setAnchor] = useState<RectCords>();
  const [manage, setManage] = useState(false);
  const order = useSpaceOrder();
  const selectedSpaceId = useSelectedSpace();
  const homeSelected = useHomeSelected();
  const directSelected = useDirectSelected();
  const { openHome, openSpace } = useNavPaneNavigation();
  const roomToUnread = useAtomValue(roomToUnreadAtom);

  const elsewhere = useMemo(
    () => otherSpacesUnread(order.items, selectedSpaceId, roomToUnread),
    [order.items, selectedSpaceId, roomToUnread]
  );
  const elsewhereText = elsewhere.unread
    ? elsewhere.attention > 0
      ? ', mentions in other spaces'
      : ', unread in other spaces'
    : '';

  const toggle = () => {
    const rect = triggerRef.current?.getBoundingClientRect();
    setAnchor((current) => (current || !rect ? undefined : rect));
  };

  return (
    <>
      <PageNavHeader data-tauri-drag-region>
        <Box alignItems="Center" grow="Yes" gap="200" data-tauri-drag-region>
          <button
            ref={triggerRef}
            type="button"
            className={css.SwitcherTrigger}
            aria-haspopup="dialog"
            aria-expanded={!!anchor}
            aria-label={`${title}, switch space${elsewhereText}`}
            data-testid="space-switcher-trigger"
            onClick={toggle}
          >
            {mark && <span className={css.TriggerMark}>{mark}</span>}
            <NavPaneTitle text={title} />
            {titleAfter}
            <span className={css.TriggerChevron}>
              <Icon src={Icons.ChevronBottom} size="100" />
              {elsewhere.unread && (
                <span className={css.TriggerDot} data-testid="space-switcher-dot">
                  <UnreadBadge highlight={elsewhere.attention > 0} count={0} />
                </span>
              )}
            </span>
          </button>
          <div className={css.HeaderDragSpace} data-tauri-drag-region />
          {actions && (
            <Box shrink="No" gap="100">
              {actions}
            </Box>
          )}
        </Box>
      </PageNavHeader>
      <PopOut
        anchor={anchor}
        position="Bottom"
        align="Start"
        offset={6}
        content={
          <FocusTrap
            focusTrapOptions={{
              // Also used when a space's menu closes and this trap resumes:
              // keep focus on the row it returned to.
              initialFocus: () =>
                (menuRef.current?.contains(document.activeElement)
                  ? (document.activeElement as HTMLElement)
                  : undefined) ??
                menuRef.current?.querySelector<HTMLElement>('[data-switcher-filter]') ??
                menuRef.current?.querySelector<HTMLElement>('[data-switcher-row][tabindex="0"]') ??
                false,
              onDeactivate: () => setAnchor(undefined),
              clickOutsideDeactivates: true,
              // Esc in a filter with text clears it first (the switcher stops
              // that Esc); otherwise it closes, even from the filter.
              escapeDeactivates: (evt: KeyboardEvent) => {
                evt.stopPropagation();
                return true;
              },
            }}
          >
            <SpaceSwitcherMenu
              ref={menuRef}
              order={order}
              selectedSpaceId={selectedSpaceId}
              homeSelected={homeSelected || directSelected}
              onOpenHome={openHome}
              onOpenSpace={openSpace}
              onManage={() => setManage(true)}
              requestClose={() => setAnchor(undefined)}
            />
          </FocusTrap>
        }
      />
      {manage && <ManageSpacesDialog order={order} requestClose={() => setManage(false)} />}
    </>
  );
}
