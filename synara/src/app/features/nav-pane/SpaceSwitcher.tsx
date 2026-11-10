import React, {
  KeyboardEvent as ReactKeyboardEvent,
  MouseEvent as ReactMouseEvent,
  RefObject,
  forwardRef,
  useCallback,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
} from 'react';
import { Avatar, Icon, IconButton, Icons, Input, Menu, PopOut, RectCords, Text } from 'folds';
import { useAtom } from 'jotai';
import {
  draggable,
  dropTargetForElements,
  monitorForElements,
} from '@atlaskit/pragmatic-drag-and-drop/element/adapter';
import {
  attachInstruction,
  extractInstruction,
  Instruction,
} from '@atlaskit/pragmatic-drag-and-drop-hitbox/tree-item';
import { autoScrollForElements } from '@atlaskit/pragmatic-drag-and-drop-auto-scroll/element';
import { combine } from '@atlaskit/pragmatic-drag-and-drop/combine';
import FocusTrap from '../../components/FocusTrap';
import { RoomAvatar } from '../../components/room-avatar';
import { UnreadBadge } from '../../components/unread-badge';
import { nameInitials, randomStr } from '../../utils/common';
import { getRoomAvatarUrl } from '../../utils/room';
import { useRoomUnread, useRoomsUnread } from '../../state/hooks/unread';
import { roomToUnreadAtom } from '../../state/room/roomToUnread';
import { useNativeNavigationScope } from '../../state/hooks/navigationUnread';
import { useOpenedSidebarFolderAtom } from '../../state/hooks/openedSidebarFolder';
import { listNavigationIndex } from '../../utils/keyboard';
import { getNativeRoom } from '../../native/nativeSession';
import type { ISidebarFolder } from '../../hooks/useSidebarItems';
import SynaraPNG from '../../../../public/res/png/synara.png';
import { SpaceRowMenu } from './SpaceRowMenu';
import type { SpaceOrder } from './useSpaceOrder';
import {
  ReorderInstruction,
  SWITCHER_FILTER_THRESHOLD,
  SpaceDraggable,
  SwitcherRow,
  folderLabel,
  reorderSpaceItems,
  spaceCount,
  switcherRows,
} from './spaceOrder';
import * as css from './NavPane.css';

const DRAG_TYPE = 'synara-space-switcher';
const nameOf = (spaceId: string): string => getNativeRoom(spaceId)?.name ?? '';

type DragData = { type: typeof DRAG_TYPE; item: SpaceDraggable };
const isSwitcherDrag = (data: Record<string | symbol, unknown>): boolean => data.type === DRAG_TYPE;

/**
 * One row's drag source and drop target. Drops only reorder (above/below):
 * dropping onto a space no longer makes a folder (D1, Manage spaces does).
 */
const useSwitcherDnd = (
  elementRef: RefObject<HTMLElement | null>,
  item: SpaceDraggable,
  { canDrag, enabled, aboveOnly }: { canDrag: boolean; enabled: boolean; aboveOnly?: boolean }
) => {
  const [dropType, setDropType] = useState<Instruction['type']>();
  const [dragging, setDragging] = useState(false);

  useEffect(() => {
    const element = elementRef.current;
    if (!element || !enabled) return undefined;
    const cleanups = [
      dropTargetForElements({
        element,
        canDrop: ({ source }) =>
          isSwitcherDrag(source.data) && (source.data as DragData).item !== item,
        getData: ({ input, element: target }) => {
          const block: Instruction['type'][] = ['reparent', 'make-child'];
          if (aboveOnly) block.push('reorder-below');
          const instruction = extractInstruction(
            attachInstruction(
              {},
              {
                input,
                element: target,
                currentLevel: 0,
                indentPerLevel: 0,
                mode: 'standard',
                block,
              }
            )
          );
          setDropType(instruction?.type);
          return { type: DRAG_TYPE, item, instructionType: instruction?.type };
        },
        onDragLeave: () => setDropType(undefined),
        onDrop: () => setDropType(undefined),
      }),
    ];
    if (canDrag) {
      cleanups.push(
        draggable({
          element,
          getInitialData: (): DragData => ({ type: DRAG_TYPE, item }),
          onDragStart: () => setDragging(true),
          onDrop: () => setDragging(false),
        })
      );
    }
    return combine(...cleanups);
  }, [elementRef, item, canDrag, enabled, aboveOnly]);

  return { dropType, dragging };
};

type RowFocusProps = {
  rowKey: string;
  activeKey: string | undefined;
  onRowFocus: (key: string) => void;
  onRowKeyDown: (evt: ReactKeyboardEvent<HTMLButtonElement>) => void;
};

type SpaceRowProps = RowFocusProps & {
  spaceId: string;
  depth: 0 | 1;
  folder?: ISidebarFolder;
  selected: boolean;
  dndEnabled: boolean;
  onOpen: (spaceId: string) => void;
  onOptions: (spaceId: string, anchor: RectCords) => void;
  optionsOpen: boolean;
};

function SpaceRow({
  spaceId,
  depth,
  folder,
  selected,
  dndEnabled,
  onOpen,
  onOptions,
  optionsOpen,
  rowKey,
  activeKey,
  onRowFocus,
  onRowKeyDown,
}: SpaceRowProps) {
  const itemRef = useRef<HTMLLIElement>(null);
  const space = getNativeRoom(spaceId);
  const unread = useRoomUnread(spaceId, roomToUnreadAtom);
  const draggableItem: SpaceDraggable = useMemo(
    () => (folder ? { folder, spaceId } : spaceId),
    [folder, spaceId]
  );
  const { dropType, dragging } = useSwitcherDnd(itemRef, draggableItem, {
    canDrag: true,
    enabled: dndEnabled,
  });
  if (!space) return null;
  const name = space.name;
  const highlight = unread?.highlight ?? 0;
  const unreadText = unread
    ? highlight > 0
      ? `, ${highlight} ${highlight === 1 ? 'mention' : 'mentions'}`
      : ', unread messages'
    : '';

  const openOptions = (target: HTMLElement) => onOptions(spaceId, target.getBoundingClientRect());

  return (
    <li
      ref={itemRef}
      className={css.SwitcherItem}
      data-drop-above={dropType === 'reorder-above' || undefined}
      data-drop-below={dropType === 'reorder-below' || undefined}
      data-dragging={dragging || undefined}
    >
      <button
        type="button"
        className={`${css.SwitcherRow} ${css.SwitcherRowWithOptions}`}
        data-switcher-row={rowKey}
        data-space-id={spaceId}
        data-depth={depth}
        tabIndex={activeKey === rowKey ? 0 : -1}
        aria-current={selected ? 'page' : undefined}
        aria-label={`${name}${unreadText}`}
        aria-keyshortcuts="Shift+F10"
        onFocus={() => onRowFocus(rowKey)}
        onKeyDown={onRowKeyDown}
        onClick={() => onOpen(spaceId)}
        onContextMenu={(evt) => {
          evt.preventDefault();
          openOptions(evt.currentTarget);
        }}
      >
        <Avatar as="span" size="200" radii="300" className={css.SwitcherAvatar}>
          <RoomAvatar
            roomId={space.roomId}
            src={getRoomAvatarUrl(space, 96) ?? undefined}
            alt=""
            renderFallback={() => (
              <Text as="span" size="H6">
                {nameInitials(name, 2)}
              </Text>
            )}
          />
        </Avatar>
        <span className={`${css.SwitcherRowName} ${unread ? css.SwitcherRowUnread : ''}`}>
          {name}
        </span>
        {unread && (
          <span aria-hidden="true">
            <UnreadBadge highlight={highlight > 0} count={highlight} />
          </span>
        )}
      </button>
      <IconButton
        className={css.SwitcherOptions}
        size="300"
        radii="300"
        variant="Surface"
        fill="None"
        tabIndex={-1}
        aria-label={`${name} options`}
        aria-haspopup="menu"
        aria-expanded={optionsOpen}
        onClick={(evt: ReactMouseEvent<HTMLButtonElement>) => openOptions(evt.currentTarget)}
      >
        <Icon size="100" src={Icons.VerticalDots} />
      </IconButton>
    </li>
  );
}

type FolderRowProps = RowFocusProps & {
  folder: ISidebarFolder;
  open: boolean;
  containsSelected: boolean;
  dndEnabled: boolean;
  onToggle: (folderId: string) => void;
};

function FolderRow({
  folder,
  open,
  containsSelected,
  dndEnabled,
  onToggle,
  rowKey,
  activeKey,
  onRowFocus,
  onRowKeyDown,
}: FolderRowProps) {
  const itemRef = useRef<HTMLLIElement>(null);
  const unread = useRoomsUnread(folder.content, roomToUnreadAtom);
  const draggableItem: SpaceDraggable = useMemo(() => ({ folder }), [folder]);
  // An open folder's spaces follow it, so it only takes drops above it, and
  // (as on the rail) only a closed folder moves as a whole.
  const { dropType, dragging } = useSwitcherDnd(itemRef, draggableItem, {
    canDrag: !open,
    enabled: dndEnabled,
    aboveOnly: open,
  });
  const label = folderLabel(folder, nameOf);
  const highlight = unread?.highlight ?? 0;

  return (
    <li
      ref={itemRef}
      className={css.SwitcherItem}
      data-drop-above={dropType === 'reorder-above' || undefined}
      data-drop-below={dropType === 'reorder-below' || undefined}
      data-dragging={dragging || undefined}
    >
      <button
        type="button"
        className={css.SwitcherRow}
        data-switcher-row={rowKey}
        data-folder-id={folder.id}
        tabIndex={activeKey === rowKey ? 0 : -1}
        aria-expanded={open}
        aria-label={`${label}, folder${containsSelected && !open ? ', contains the current space' : ''}${
          unread ? (highlight > 0 ? `, ${highlight} mentions` : ', unread messages') : ''
        }`}
        onFocus={() => onRowFocus(rowKey)}
        onKeyDown={onRowKeyDown}
        onClick={() => onToggle(folder.id)}
      >
        <span className={css.SwitcherFolderChevron}>
          <Icon size="100" src={Icons.ChevronBottom} />
        </span>
        <span className={`${css.SwitcherRowName} ${unread && !open ? css.SwitcherRowUnread : ''}`}>
          {label}
        </span>
        {unread && !open && (
          <span aria-hidden="true">
            <UnreadBadge highlight={highlight > 0} count={highlight} />
          </span>
        )}
      </button>
    </li>
  );
}

function HomeRow({
  selected,
  onOpen,
  rowKey,
  activeKey,
  onRowFocus,
  onRowKeyDown,
}: RowFocusProps & { selected: boolean; onOpen: () => void }) {
  const { unread, attention } = useNativeNavigationScope('merged');
  return (
    <li className={css.SwitcherItem}>
      <button
        type="button"
        className={css.SwitcherRow}
        data-switcher-row={rowKey}
        tabIndex={activeKey === rowKey ? 0 : -1}
        aria-current={selected ? 'page' : undefined}
        aria-label={`Home${
          unread
            ? attention > 0
              ? `, ${attention} mentions and direct messages`
              : ', unread messages'
            : ''
        }`}
        onFocus={() => onRowFocus(rowKey)}
        onKeyDown={onRowKeyDown}
        onClick={onOpen}
      >
        <span className={css.SwitcherAvatar}>
          <img src={SynaraPNG} alt="" width={24} height={24} style={{ borderRadius: 6 }} />
        </span>
        <span className={`${css.SwitcherRowName} ${unread ? css.SwitcherRowUnread : ''}`}>
          Home
        </span>
        {unread && (
          <span aria-hidden="true">
            <UnreadBadge highlight={attention > 0} count={attention} />
          </span>
        )}
      </button>
    </li>
  );
}

const HOME_KEY = 'home';
const rowKeyOf = (row: SwitcherRow): string =>
  row.kind === 'folder' ? `folder:${row.folder.id}` : `space:${row.spaceId}`;

export type SpaceSwitcherMenuProps = {
  order: SpaceOrder;
  /** The space whose list is showing, if any. */
  selectedSpaceId?: string;
  /** Whether Home's list is showing (Home or a DM route). */
  homeSelected: boolean;
  onOpenHome: () => void;
  onOpenSpace: (spaceId: string) => void;
  onManage: () => void;
  requestClose: () => void;
};

/**
 * The list title's menu (N3): Home and every space with its unread, folders
 * as collapsible groups, drag to reorder, a per-space context menu and a
 * type-ahead filter once there are more than eight spaces.
 *
 * Keys: ↑/↓, Home/End move; Enter opens; →/← open and close a folder (← on a
 * space in a folder goes to the folder); Shift+F10 or the Menu key opens a
 * space's options; typing in the filter narrows the list; Esc clears the
 * filter, then closes.
 */
export const SpaceSwitcherMenu = forwardRef<HTMLDivElement, SpaceSwitcherMenuProps>(
  (
    { order, selectedSpaceId, homeSelected, onOpenHome, onOpenSpace, onManage, requestClose },
    ref
  ) => {
    const listRef = useRef<HTMLUListElement>(null);
    const filterRef = useRef<HTMLInputElement>(null);
    const listId = useId();
    const [openedFolder, setOpenedFolder] = useAtom(useOpenedSidebarFolderAtom());
    const [filter, setFilter] = useState('');
    const [rowMenu, setRowMenu] = useState<{ spaceId: string; anchor: RectCords }>();
    const { items, save, unpin, orphanSpaces, saveError, clearSaveError } = order;

    const showFilter = spaceCount(items) > SWITCHER_FILTER_THRESHOLD;
    const query = filter.trim();
    const rows = useMemo(
      () => switcherRows(items, openedFolder, query, nameOf),
      [items, openedFolder, query]
    );
    const showHome = !query || 'home'.includes(query.toLowerCase());
    const rowKeys = useMemo(
      () => [...(showHome ? [HOME_KEY] : []), ...rows.map(rowKeyOf)],
      [rows, showHome]
    );
    const selectedKey = selectedSpaceId
      ? rows.find((row) => row.kind === 'space' && row.spaceId === selectedSpaceId) &&
        `space:${selectedSpaceId}`
      : homeSelected
        ? HOME_KEY
        : undefined;
    const [activeKey, setActiveKey] = useState<string | undefined>(() => selectedKey || rowKeys[0]);
    const rovingKey = activeKey && rowKeys.includes(activeKey) ? activeKey : rowKeys[0];

    const focusRow = useCallback((key: string | undefined) => {
      if (!key) return;
      listRef.current
        ?.querySelector<HTMLElement>(`[data-switcher-row="${CSS.escape(key)}"]`)
        ?.focus();
    }, []);

    const toggleFolder = useCallback(
      (folderId: string) =>
        setOpenedFolder({ type: openedFolder.has(folderId) ? 'DELETE' : 'PUT', id: folderId }),
      [openedFolder, setOpenedFolder]
    );

    // Drag to reorder, saved with the rail's rollback-on-failure.
    const dndEnabled = !query;
    useEffect(() => {
      const list = listRef.current;
      if (!list || !dndEnabled) return undefined;
      return combine(
        monitorForElements({
          canMonitor: ({ source }) => isSwitcherDrag(source.data),
          onDrop: ({ source, location }) => {
            const target = location.current.dropTargets[0];
            if (!target) return;
            const instructionType = target.data.instructionType as ReorderInstruction | undefined;
            if (!instructionType || instructionType === 'make-child') return;
            const edit = reorderSpaceItems(
              items,
              (source.data as DragData).item,
              target.data.item as SpaceDraggable,
              instructionType,
              randomStr
            );
            edit.removedFolderIds.forEach((id) => setOpenedFolder({ type: 'DELETE', id }));
            save(items, edit.items, "Couldn't save the new order of your spaces. Try again.");
          },
        }),
        autoScrollForElements({ element: list })
      );
    }, [dndEnabled, items, save, setOpenedFolder]);

    const openSpace = (spaceId: string) => {
      requestClose();
      onOpenSpace(spaceId);
    };
    const openHome = () => {
      requestClose();
      onOpenHome();
    };

    const handleListKeyDown = (evt: ReactKeyboardEvent<HTMLButtonElement>) => {
      const target = evt.currentTarget;
      const key = target.getAttribute('data-switcher-row');
      if (!key) return;
      const index = rowKeys.indexOf(key);
      const next = listNavigationIndex(evt.key, index, rowKeys.length);
      if (next !== undefined) {
        evt.preventDefault();
        if (evt.key === 'ArrowUp' && index === 0 && showFilter) {
          filterRef.current?.focus();
          return;
        }
        focusRow(rowKeys[next]);
        return;
      }
      const folderId = target.getAttribute('data-folder-id');
      const spaceId = target.getAttribute('data-space-id');
      if (folderId && (evt.key === 'ArrowRight' || evt.key === 'ArrowLeft')) {
        evt.preventDefault();
        const open = openedFolder.has(folderId);
        if ((evt.key === 'ArrowRight') !== open) toggleFolder(folderId);
        return;
      }
      if (spaceId && evt.key === 'ArrowLeft') {
        const row = rows.find((r) => r.kind === 'space' && r.spaceId === spaceId);
        if (row?.kind === 'space' && row.folder) {
          evt.preventDefault();
          focusRow(`folder:${row.folder.id}`);
        }
        return;
      }
      if (spaceId && (evt.key === 'ContextMenu' || (evt.key === 'F10' && evt.shiftKey))) {
        evt.preventDefault();
        setRowMenu({ spaceId, anchor: target.getBoundingClientRect() });
      }
    };

    const handleFilterKeyDown = (evt: ReactKeyboardEvent<HTMLInputElement>) => {
      if (evt.key === 'Escape' && filter) {
        // Esc clears the filter first; stopping it here keeps the focus trap
        // (listening on the document) from also closing the switcher.
        evt.preventDefault();
        evt.stopPropagation();
        setFilter('');
      } else if (evt.key === 'ArrowDown') {
        evt.preventDefault();
        focusRow(rowKeys[0]);
      } else if (evt.key === 'Enter') {
        evt.preventDefault();
        const first = rows.find((row) => row.kind === 'space');
        if (first?.kind === 'space') openSpace(first.spaceId);
        else if (showHome) openHome();
      }
    };

    const rowMenuRoom = rowMenu ? getNativeRoom(rowMenu.spaceId) : undefined;

    return (
      <Menu
        ref={ref}
        className={css.SwitcherMenu}
        role="dialog"
        aria-label="Switch space"
        data-testid="space-switcher"
      >
        {showFilter && (
          <div className={css.SwitcherFilter}>
            <Input
              ref={filterRef}
              size="300"
              radii="300"
              variant="Background"
              type="search"
              value={filter}
              placeholder="Find a space"
              aria-label="Find a space"
              aria-controls={listId}
              autoComplete="off"
              data-switcher-filter
              onChange={(evt) => setFilter(evt.currentTarget.value)}
              onKeyDown={handleFilterKeyDown}
              before={<Icon size="50" src={Icons.Search} />}
            />
          </div>
        )}
        <ul ref={listRef} id={listId} className={css.SwitcherList} aria-label="Home and spaces">
          {showHome && (
            <HomeRow
              rowKey={HOME_KEY}
              activeKey={rovingKey}
              onRowFocus={setActiveKey}
              onRowKeyDown={handleListKeyDown}
              selected={homeSelected}
              onOpen={openHome}
            />
          )}
          {showHome && rows.length > 0 && (
            <li role="presentation" aria-hidden="true" className={css.SwitcherSeparator} />
          )}
          {rows.map((row) =>
            row.kind === 'folder' ? (
              <FolderRow
                key={rowKeyOf(row)}
                rowKey={rowKeyOf(row)}
                activeKey={rovingKey}
                onRowFocus={setActiveKey}
                onRowKeyDown={handleListKeyDown}
                folder={row.folder}
                open={row.open}
                containsSelected={!!selectedSpaceId && row.folder.content.includes(selectedSpaceId)}
                dndEnabled={dndEnabled}
                onToggle={toggleFolder}
              />
            ) : (
              <SpaceRow
                key={`${row.folder?.id ?? ''}:${row.spaceId}`}
                rowKey={rowKeyOf(row)}
                activeKey={rovingKey}
                onRowFocus={setActiveKey}
                onRowKeyDown={handleListKeyDown}
                spaceId={row.spaceId}
                depth={row.depth}
                folder={row.folder}
                selected={row.spaceId === selectedSpaceId}
                dndEnabled={dndEnabled}
                onOpen={openSpace}
                onOptions={(spaceId, anchor) => setRowMenu({ spaceId, anchor })}
                optionsOpen={rowMenu?.spaceId === row.spaceId}
              />
            )
          )}
        </ul>
        {query && rows.length === 0 && !showHome && (
          <p className={css.SwitcherEmpty} role="status">
            No spaces match “{query}”.
          </p>
        )}
        {saveError && (
          <div className={css.SwitcherError} role="alert">
            <Icon size="100" src={Icons.Warning} />
            <span style={{ flexGrow: 1 }}>{saveError}</span>
            <IconButton
              size="300"
              radii="300"
              variant="Critical"
              fill="None"
              aria-label="Dismiss"
              onClick={clearSaveError}
            >
              <Icon size="50" src={Icons.Cross} />
            </IconButton>
          </div>
        )}
        <div className={css.SwitcherFooter}>
          <button
            type="button"
            className={css.SwitcherRow}
            onClick={() => {
              requestClose();
              onManage();
            }}
          >
            <span className={css.SwitcherAvatar} style={{ display: 'inline-flex' }}>
              <Icon size="200" src={Icons.Space} />
            </span>
            <span className={css.SwitcherRowName}>Manage spaces</span>
          </button>
        </div>
        {rowMenu && rowMenuRoom && (
          <PopOut
            anchor={rowMenu.anchor}
            position="Right"
            align="Start"
            content={
              <FocusTrap
                focusTrapOptions={{
                  initialFocus: false,
                  onDeactivate: () => setRowMenu(undefined),
                  clickOutsideDeactivates: true,
                  isKeyForward: (evt: KeyboardEvent) => evt.key === 'ArrowDown',
                  isKeyBackward: (evt: KeyboardEvent) => evt.key === 'ArrowUp',
                  escapeDeactivates: (evt: KeyboardEvent) => {
                    evt.stopPropagation();
                    return true;
                  },
                }}
              >
                <SpaceRowMenu
                  room={rowMenuRoom}
                  requestClose={() => setRowMenu(undefined)}
                  onUnpin={orphanSpaces.includes(rowMenu.spaceId) ? undefined : unpin}
                />
              </FocusTrap>
            }
          />
        )}
      </Menu>
    );
  }
);
