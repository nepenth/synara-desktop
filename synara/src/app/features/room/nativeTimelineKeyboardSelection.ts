/**
 * Slack's quick edit: Up in an empty message box selects your last message in
 * that timeline, Up and Down then move between messages, E edits the selected
 * one when it is yours, and Escape goes back to the message box.
 *
 * The message box and the timeline are siblings, so each registers what it
 * can do for its timeline (a room, or one thread in it) and the other asks.
 * A room view and a thread pane of the same room never answer for each other.
 */

type Registry<T> = Map<string, T[]>;

const scopeKey = (roomId: string, threadRootId: string | undefined): string =>
  `${roomId}\u0000${threadRootId || ''}`;

const register = <T>(registry: Registry<T>, key: string, entry: T): (() => void) => {
  const stack = registry.get(key) ?? [];
  stack.push(entry);
  registry.set(key, stack);
  return () => {
    const index = stack.lastIndexOf(entry);
    if (index >= 0) stack.splice(index, 1);
    if (stack.length === 0 && registry.get(key) === stack) registry.delete(key);
  };
};

/** The most recently mounted entry answers, as with the app shortcut handlers. */
const latest = <T>(registry: Registry<T>, key: string): T | undefined => registry.get(key)?.at(-1);

const lastOwnMessageFocusers: Registry<() => boolean> = new Map();
const composerFocusers: Registry<() => void> = new Map();

/** A timeline view offers to move focus to the user's last message in it. */
export const registerTimelineLastOwnMessageFocus = (
  roomId: string,
  threadRootId: string | undefined,
  focus: () => boolean
): (() => void) => register(lastOwnMessageFocusers, scopeKey(roomId, threadRootId), focus);

/**
 * Move focus to the user's last message in this timeline. False when there
 * is no such view or message, so the key that asked keeps its default.
 */
export const focusLastOwnTimelineMessage = (
  roomId: string,
  threadRootId: string | undefined
): boolean => latest(lastOwnMessageFocusers, scopeKey(roomId, threadRootId))?.() ?? false;

/** A message box offers to take focus back from its timeline. */
export const registerTimelineComposerFocus = (
  roomId: string,
  threadRootId: string | undefined,
  focus: () => void
): (() => void) => register(composerFocusers, scopeKey(roomId, threadRootId), focus);

/** Focus this timeline's message box. False when it has none (a read-only room). */
export const focusTimelineComposer = (
  roomId: string,
  threadRootId: string | undefined
): boolean => {
  const focus = latest(composerFocusers, scopeKey(roomId, threadRootId));
  if (!focus) return false;
  focus();
  return true;
};

type SelectionKeyEvent = {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
};

const unmodified = (evt: SelectionKeyEvent): boolean =>
  !evt.ctrlKey && !evt.metaKey && !evt.altKey && !evt.shiftKey;

/**
 * Up leaves the message box only when there is nothing in it and nothing
 * else wants the key: a draft keeps normal caret movement, an open
 * suggestion list keeps its own arrows, and an input method keeps its keys.
 */
export const composerArrowUpLeavesForTimeline = (
  evt: SelectionKeyEvent,
  composer: { empty: boolean; suggestionsOpen: boolean; composing: boolean }
): boolean =>
  evt.key === 'ArrowUp' &&
  unmodified(evt) &&
  composer.empty &&
  !composer.suggestionsOpen &&
  !composer.composing;

export type TimelineSelectionKey = 'previous' | 'next' | 'composer' | 'edit';

/** What a plain key does on a selected message; modified keys keep their meaning. */
export const timelineSelectionKey = (evt: SelectionKeyEvent): TimelineSelectionKey | undefined => {
  if (!unmodified(evt)) return undefined;
  if (evt.key === 'ArrowUp') return 'previous';
  if (evt.key === 'ArrowDown') return 'next';
  if (evt.key === 'Escape') return 'composer';
  if (evt.key === 'e' || evt.key === 'E') return 'edit';
  return undefined;
};

export type TimelineSelectionRow = {
  /** The row can take keyboard focus (it has a server event and actions). */
  selectable: boolean;
  /** A message, sticker or poll this account sent. */
  ownMessage: boolean;
  /** Still on its way to the server: it has no event to act on yet. */
  sending: boolean;
};

/**
 * The row Up selects from the message box: the newest message this account
 * sent. While that message is still being sent the answer is `wait`, not the
 * message before it, so a quick Up then E never opens the wrong message.
 * A message that failed to send is passed over; it has its own Retry.
 */
export const lastOwnMessageTarget = (
  rows: readonly TimelineSelectionRow[]
): { kind: 'row'; index: number } | { kind: 'wait' } | { kind: 'none' } => {
  for (let index = rows.length - 1; index >= 0; index -= 1) {
    const row = rows[index];
    if (!row.ownMessage) continue;
    if (row.selectable) return { kind: 'row', index };
    if (row.sending) return { kind: 'wait' };
  }
  return { kind: 'none' };
};

/** The nearest row above or below `from` that can take focus. */
export const adjacentSelectableRow = (
  rows: readonly Pick<TimelineSelectionRow, 'selectable'>[],
  from: number,
  direction: 'previous' | 'next'
): number | undefined => {
  const step = direction === 'next' ? 1 : -1;
  for (let index = from + step; index >= 0 && index < rows.length; index += step) {
    if (rows[index].selectable) return index;
  }
  return undefined;
};

const LABEL_BODY_LIMIT = 160;

/** What a screen reader says when a message takes focus: who, then what. */
export const timelineMessageLabel = (senderName: string, body: string | undefined): string => {
  const text = (body ?? '').replace(/\s+/g, ' ').trim();
  if (!text) return `Message from ${senderName}`;
  const short =
    text.length > LABEL_BODY_LIMIT ? `${text.slice(0, LABEL_BODY_LIMIT).trimEnd()}…` : text;
  return `${senderName}: ${short}`;
};
