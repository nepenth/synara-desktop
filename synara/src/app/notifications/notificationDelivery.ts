/**
 * Renderer-side delivery rules for desktop notifications. Core decides whether
 * an event notifies; these rules decide what the renderer does with that
 * decision: when to hold a banner back, when a sound may play, and which
 * delivered banners to withdraw later.
 */

export type NotificationDeliveryOutcome = 'delivered' | 'failed' | undefined;

/**
 * An ordinary message banner is held back only while the user is looking at
 * the Inbox notification list in a focused, visible window. With the window
 * hidden or in the background (the tray and global shortcut both open that
 * list), messages still notify. A room missing from the room-list snapshot is
 * never a reason to drop one; only a known space room is.
 */
export const shouldPresentOrdinaryMessage = ({
  notificationListSelected,
  windowActive,
  isSpaceRoom,
}: {
  notificationListSelected: boolean;
  windowActive: boolean;
  isSpaceRoom: boolean;
}): boolean => !isSpaceRoom && !(notificationListSelected && windowActive);

/** The window is in front of the user: focused and not hidden or minimized. */
export const isDocumentActive = (doc: Pick<Document, 'hasFocus' | 'visibilityState'>): boolean =>
  doc.visibilityState === 'visible' && doc.hasFocus();

/**
 * A notification sound follows its banner. Nothing plays when notifications
 * are off (the tray's Do Not Disturb item turns them off), when permission is
 * missing, or when the OS did not accept the banner.
 *
 * The Desktop Notifications switch, and so tray Do Not Disturb, governs every
 * kind, approvals included: there is no separate desktop switch for approval
 * alerts, and an unanswered approval expires without running the command.
 */
export const shouldPlayNotificationSound = ({
  soundsEnabled,
  outcome,
  requested = true,
}: {
  soundsEnabled: boolean;
  outcome: NotificationDeliveryOutcome;
  /** Core's per-event sound decision (push rule tweak) where one exists. */
  requested?: boolean;
}): boolean => soundsEnabled && requested && outcome === 'delivered';

export const inviteNotificationText = (count: number): { title: string; body: string } =>
  count === 1
    ? { title: 'New invitation', body: 'You have a new invitation.' }
    : { title: 'New invitations', body: `You have ${count} new invitations.` };

/**
 * Counts invites that arrived since the previous snapshot. The first loaded
 * snapshot of a session is the baseline, so invites that already existed at
 * launch or sign-in do not notify; nor do invites that arrive while sync is
 * not running.
 */
export const createInviteArrivalTracker = () => {
  let generation = 0;
  let known = new Set<string>();
  return (snapshot: { sessionGeneration: number; roomIds: readonly string[] }, live: boolean) => {
    if (snapshot.sessionGeneration === 0) {
      generation = 0;
      known = new Set();
      return 0;
    }
    const baseline = snapshot.sessionGeneration !== generation;
    const arrived = snapshot.roomIds.filter((roomId) => !known.has(roomId)).length;
    generation = snapshot.sessionGeneration;
    known = new Set(snapshot.roomIds);
    return baseline || !live ? 0 : arrived;
  };
};

type RoomUnreadSummary = {
  roomId: string;
  unreadCount: number;
  highlightCount: number;
  markedUnread: boolean;
};

const hasUnread = (room: RoomUnreadSummary): boolean =>
  room.unreadCount > 0 || room.highlightCount > 0 || room.markedUnread;

/**
 * Rooms that were unread in the previous snapshot and no longer are: read
 * here or on another device, or left. Their message banners are stale.
 */
export const createRoomReadTracker = () => {
  let unread = new Set<string>();
  return (rooms: readonly RoomUnreadSummary[]): string[] => {
    const next = new Set(rooms.filter(hasUnread).map((room) => room.roomId));
    const read = [...unread].filter((roomId) => !next.has(roomId));
    unread = next;
    return read;
  };
};

type TimerHandle = ReturnType<typeof setTimeout>;

const MAX_TRACKED_APPROVAL_ALERTS = 256;

/**
 * Withdraws a delivered approval alert when its prompt expires or is decided
 * elsewhere, so a stale banner never keeps its Approve/Deny buttons.
 */
export const createApprovalAlertWithdrawal = ({
  withdraw,
  now = Date.now,
  schedule = (callback, delayMs) => setTimeout(callback, delayMs),
  cancel = (handle) => clearTimeout(handle),
}: {
  withdraw: (eventId: string) => void;
  now?: () => number;
  schedule?: (callback: () => void, delayMs: number) => TimerHandle;
  cancel?: (handle: TimerHandle) => void;
}) => {
  const timers = new Map<string, TimerHandle>();
  const forget = (eventId: string): boolean => {
    const handle = timers.get(eventId);
    if (handle === undefined) return false;
    cancel(handle);
    timers.delete(eventId);
    return true;
  };
  return {
    /** Arms the expiry for a delivered alert. */
    track(eventId: string, expiresAt: number) {
      forget(eventId);
      const delay = Math.max(0, expiresAt - now());
      timers.set(
        eventId,
        schedule(() => {
          timers.delete(eventId);
          withdraw(eventId);
        }, delay)
      );
      if (timers.size > MAX_TRACKED_APPROVAL_ALERTS) {
        const oldest = timers.keys().next().value;
        if (oldest !== undefined) forget(oldest);
      }
    },
    /** The prompt is no longer pending (decided or expired per Core). */
    settle(eventId: string) {
      if (forget(eventId)) withdraw(eventId);
    },
    has: (eventId: string) => timers.has(eventId),
    clear() {
      timers.forEach((handle) => cancel(handle));
      timers.clear();
    },
  };
};
