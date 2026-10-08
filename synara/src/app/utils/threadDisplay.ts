/**
 * How the desktop room opens a thread.
 *
 * - `full`: the thread replaces the room timeline, with a back control.
 * - `side`: the thread opens in a resizable pane beside the live room timeline.
 * - `inline`: replies expand beneath their root message in the room timeline;
 *   replying opens the side pane.
 */
export const THREAD_DISPLAY_MODES = ['full', 'side', 'inline'] as const;

export type ThreadDisplayMode = (typeof THREAD_DISPLAY_MODES)[number];

export const DEFAULT_THREAD_DISPLAY: ThreadDisplayMode = 'full';

export const THREAD_DISPLAY_LABELS: Record<ThreadDisplayMode, string> = {
  full: 'Full view',
  side: 'Side panel',
  inline: 'Inline',
};

export const normalizeThreadDisplay = (value: unknown): ThreadDisplayMode =>
  typeof value === 'string' && THREAD_DISPLAY_MODES.includes(value as ThreadDisplayMode)
    ? (value as ThreadDisplayMode)
    : DEFAULT_THREAD_DISPLAY;

export const THREAD_PANE_MIN_WIDTH = 320;
export const THREAD_PANE_MAX_WIDTH = 720;
export const DEFAULT_THREAD_PANE_WIDTH = 420;

/**
 * Clamp a pane width to its bounds, and to at most 60% of the available room
 * width so the room timeline always keeps space.
 */
export const clampThreadPaneWidth = (width: unknown, availableWidth?: number): number => {
  const requested =
    typeof width === 'number' && Number.isFinite(width) ? width : DEFAULT_THREAD_PANE_WIDTH;
  const ceiling =
    typeof availableWidth === 'number' && Number.isFinite(availableWidth) && availableWidth > 0
      ? Math.max(THREAD_PANE_MIN_WIDTH, Math.min(THREAD_PANE_MAX_WIDTH, availableWidth * 0.6))
      : THREAD_PANE_MAX_WIDTH;
  return Math.round(Math.min(ceiling, Math.max(THREAD_PANE_MIN_WIDTH, requested)));
};

/** Inline replies shown before "Show earlier replies", and the step it reveals. */
export const INLINE_THREAD_INITIAL_REPLIES = 5;
export const INLINE_THREAD_REVEAL_STEP = 10;

/** The latest `visible` replies, oldest first, and how many earlier ones are hidden. */
export const visibleInlineReplies = <T>(
  replies: readonly T[],
  visible: number
): { shown: T[]; hidden: number } => {
  const count = Math.max(0, Math.floor(visible));
  const shown = count >= replies.length ? [...replies] : replies.slice(replies.length - count);
  return { shown, hidden: replies.length - shown.length };
};
