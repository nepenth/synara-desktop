import type { VimListKeysSetting } from '../../state/settings';
import { currentRoomListNavSource, roomListNavTargets } from './roomListNavigation';

/**
 * Optional list keys for a focused room row (K4): j and k step down and up,
 * g g goes to the first room, G to the last, and Enter opens the row (the row
 * is a link, so Enter is the browser's). Not app shortcuts: they act only
 * while a row has focus, never while typing.
 */

export type VimListMove = 'down' | 'up' | 'top' | 'bottom';

/** How long the first `g` of `g g` waits for the second. */
export const VIM_G_TIMEOUT_MS = 1000;

/** On while chosen; on Auto (not chosen yet) it follows tiling mode. */
export const resolveVimListKeys = (setting: VimListKeysSetting, tilingMode: boolean): boolean =>
  setting === 'auto' ? tilingMode : setting === 'on';

type VimKeyEvent = Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'metaKey' | 'altKey' | 'isComposing'>;

/**
 * The move a key asks for. `pendingG` is when an unanswered `g` was pressed;
 * the result carries the new value (a `g` that starts `g g`, or none).
 */
export const vimListKey = (
  evt: VimKeyEvent,
  pendingG: number | undefined,
  now: number
): { move?: VimListMove; pendingG?: number } => {
  if (evt.ctrlKey || evt.metaKey || evt.altKey || evt.isComposing) return {};
  switch (evt.key) {
    case 'j':
      return { move: 'down' };
    case 'k':
      return { move: 'up' };
    case 'G':
      return { move: 'bottom' };
    case 'g':
      if (pendingG !== undefined && now - pendingG <= VIM_G_TIMEOUT_MS) return { move: 'top' };
      return { pendingG: now };
    default:
      return {};
  }
};

const ROOM_ROW = 'a[data-room-nav-link]';

const isEditable = (element: EventTarget | null): boolean =>
  element instanceof HTMLElement &&
  (element.isContentEditable || element.tagName === 'INPUT' || element.tagName === 'TEXTAREA');

const findRoomRow = (scope: ParentNode, roomId: string) =>
  scope.querySelector<HTMLElement>(`${ROOM_ROW}[data-room-nav-id="${CSS.escape(roomId)}"]`);

/**
 * Focus a room's row: at once when it is rendered (so a quick second key
 * starts from it), else once the virtual list has rendered it.
 */
const focusRoomRow = (scope: ParentNode, roomId: string, framesLeft = 20) => {
  const row = findRoomRow(scope, roomId);
  if (row) {
    row.focus({ preventScroll: true });
    row.scrollIntoView({ block: 'nearest' });
    return;
  }
  if (framesLeft > 0) requestAnimationFrame(() => focusRoomRow(scope, roomId, framesLeft - 1));
};

let pendingG: number | undefined;

/**
 * Handle a key on a focused room row. Moves walk the list's data (rows the
 * virtual list has not rendered included) and fall back to the rendered rows
 * when no list publishes its data. Returns true when the key was used.
 */
export const handleVimListKey = (evt: KeyboardEvent, row: HTMLElement): boolean => {
  if (isEditable(evt.target)) return false;
  const result = vimListKey(evt, pendingG, Date.now());
  pendingG = result.pendingG;
  if (result.pendingG !== undefined) return true;
  const { move } = result;
  if (!move) return false;

  const scope: ParentNode = row.ownerDocument;
  const roomId = row.getAttribute('data-room-nav-id');
  const source = currentRoomListNavSource();
  const targets = source ? roomListNavTargets(source) : [];
  const index = targets.findIndex((target) => target.roomId === roomId);
  if (targets.length > 0 && index >= 0) {
    const nextIndex =
      move === 'top'
        ? 0
        : move === 'bottom'
          ? targets.length - 1
          : Math.min(Math.max(index + (move === 'down' ? 1 : -1), 0), targets.length - 1);
    const target = targets[nextIndex];
    target.section.reveal?.(target.index);
    focusRoomRow(scope, target.roomId);
    return true;
  }

  const rows = Array.from(document.querySelectorAll<HTMLElement>(ROOM_ROW));
  const current = rows.indexOf(row);
  if (current < 0) return false;
  const next =
    move === 'top'
      ? 0
      : move === 'bottom'
        ? rows.length - 1
        : Math.min(Math.max(current + (move === 'down' ? 1 : -1), 0), rows.length - 1);
  rows[next]?.focus();
  return true;
};
