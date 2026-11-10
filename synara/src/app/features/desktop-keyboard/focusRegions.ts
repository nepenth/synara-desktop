import { ROOM_LIST_PANE_ID, roomListFocusTarget } from '../room-list-pane/roomListPane';

/**
 * F6 / Shift+F6 move focus between the window's areas, in this order and
 * wrapping: room list, timeline, composer, then the thread pane when it is
 * open. Areas that are not on screen (a hidden list, a room you cannot write
 * in, no thread) are skipped.
 */
export const FOCUS_REGIONS = ['room-list', 'timeline', 'composer', 'thread'] as const;
export type FocusRegion = (typeof FOCUS_REGIONS)[number];

/**
 * The area after (or before) `current` among the `available` ones. From
 * outside every area, F6 starts at the first and Shift+F6 at the last.
 */
export const cycleFocusRegion = (
  available: readonly FocusRegion[],
  current: FocusRegion | undefined,
  direction: 'next' | 'previous'
): FocusRegion | undefined => {
  const present = FOCUS_REGIONS.filter((region) => available.includes(region));
  if (present.length === 0) return undefined;
  const step = direction === 'next' ? 1 : -1;
  const from = current
    ? FOCUS_REGIONS.indexOf(current)
    : direction === 'next'
      ? -1
      : FOCUS_REGIONS.length;
  for (let offset = 1; offset <= FOCUS_REGIONS.length; offset += 1) {
    const index =
      (((from + step * offset) % FOCUS_REGIONS.length) + FOCUS_REGIONS.length) %
      FOCUS_REGIONS.length;
    const region = FOCUS_REGIONS[index];
    if (present.includes(region)) return region;
  }
  return undefined;
};

type RegionElements = {
  /** What contains focus while the area is current. */
  container: HTMLElement;
  /** What takes focus when F6 moves to the area. */
  target: HTMLElement;
};

const COMPOSER = '[data-slate-editor="true"]';
const TIMELINE = '#native-timeline-history';

const regionElements = (doc: Document): Partial<Record<FocusRegion, RegionElements>> => {
  const elements: Partial<Record<FocusRegion, RegionElements>> = {};

  const pane = doc.getElementById(ROOM_LIST_PANE_ID);
  const listTarget = pane && !pane.hasAttribute('inert') ? roomListFocusTarget(pane) : null;
  if (pane && listTarget) elements['room-list'] = { container: pane, target: listTarget };

  const room = doc.querySelector<HTMLElement>('[data-file-drop-region="room"]');
  const timeline = room?.querySelector<HTMLElement>(TIMELINE);
  if (timeline) elements.timeline = { container: timeline, target: timeline };
  const composer = room?.querySelector<HTMLElement>(COMPOSER);
  if (composer) elements.composer = { container: composer, target: composer };

  const thread = doc.querySelector<HTMLElement>('[data-thread-side-panel]');
  if (thread) {
    const target =
      thread.querySelector<HTMLElement>(COMPOSER) ??
      thread.querySelector<HTMLElement>(TIMELINE) ??
      thread;
    elements.thread = { container: thread, target };
  }
  return elements;
};

/** Move focus to the next or previous area. Returns false when there is none. */
export const moveFocusRegion = (doc: Document, direction: 'next' | 'previous'): boolean => {
  const elements = regionElements(doc);
  const available = FOCUS_REGIONS.filter((region) => elements[region]);
  const active = doc.activeElement;
  const current = available.find((region) => elements[region]?.container.contains(active));
  const next = cycleFocusRegion(available, current, direction);
  const target = next && elements[next]?.target;
  if (!target) return false;
  // The timeline scroller and the thread pane are not in the tab order; make
  // them focusable from script only, so arrow and page keys then scroll them.
  if (!target.hasAttribute('tabindex') && target.tabIndex < 0) target.tabIndex = -1;
  target.focus({ preventScroll: next !== 'room-list' });
  return true;
};
