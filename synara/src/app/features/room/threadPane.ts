import type { ThreadDisplayMode } from '../../utils/threadDisplay';

export type ThreadPaneTarget = { rootEventId: string; latestEventId?: string };

/**
 * The thread shown in the room's side pane, if any.
 *
 * Full view never uses the pane. Side panel shows the opened thread, or a
 * thread route (a notification or link) in the pane instead of replacing the
 * room. Inline only uses the pane for "Reply in thread"; a thread route still
 * opens full view there. The pane needs the desktop layout.
 */
export const resolveThreadPane = ({
  mode,
  opened,
  routeThreadRootId,
  desktop,
}: {
  mode: ThreadDisplayMode;
  opened?: ThreadPaneTarget;
  routeThreadRootId?: string;
  desktop: boolean;
}): ThreadPaneTarget | undefined => {
  if (!desktop || mode === 'full') return undefined;
  if (opened) return opened;
  if (mode === 'side' && routeThreadRootId) return { rootEventId: routeThreadRootId };
  return undefined;
};
