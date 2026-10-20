import type { NativeTimelineEventRoute } from './nativeTimelineView';

export const FOCUS_TARGET_MISSING_MESSAGE =
  "That message isn't available. It may have been deleted, or you may not have access to it.";
export const FOCUS_TARGET_UNREACHABLE_MESSAGE =
  "That message couldn't be loaded right now. Showing the latest messages instead.";

export type NativeFocusRouteAction =
  /** The room stream shows the event: the presenter places it as usual. */
  | { kind: 'show_room' }
  /** Open the thread scrolled to the reply; the room stays behind it. */
  | { kind: 'open_thread'; rootEventId: string; replyEventId: string }
  /** The room opened at its live bottom; say why the message is not shown. */
  | { kind: 'notice'; message: string };

/**
 * What the room presenter does once Core has resolved a `focused` open of a
 * notification, link, search hit or reply jump. Core never returns a
 * thread-only stream as the room, so a thread reply always needs the thread UI.
 */
export const resolveNativeFocusRouteAction = (
  eventId: string,
  route: NativeTimelineEventRoute
): NativeFocusRouteAction => {
  switch (route.kind) {
    case 'thread':
      return route.root_event_id && route.root_event_id !== eventId
        ? { kind: 'open_thread', rootEventId: route.root_event_id, replyEventId: eventId }
        : { kind: 'show_room' };
    case 'missing':
      return { kind: 'notice', message: FOCUS_TARGET_MISSING_MESSAGE };
    case 'unreachable':
      return { kind: 'notice', message: FOCUS_TARGET_UNREACHABLE_MESSAGE };
    default:
      return { kind: 'show_room' };
  }
};
