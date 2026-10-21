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

/**
 * The sound a native banner asks the OS for. The OS plays it, so macOS Focus
 * and the desktop's Do Not Disturb govern it, and the app plays nothing of
 * its own for a native banner.
 */
export const nativeNotificationSound = ({
  soundsEnabled,
  requested = true,
}: {
  soundsEnabled: boolean;
  /** Core's per-event sound decision (push rule tweak) where one exists. */
  requested?: boolean;
}): 'default' | 'silent' => (soundsEnabled && requested ? 'default' : 'silent');

/**
 * The app's own sound is only for the browser fallback; a native banner
 * carries its sound in the notification itself.
 */
export const shouldPlayAppNotificationSound = ({
  nativeDelivery,
  ...rest
}: Parameters<typeof shouldPlayNotificationSound>[0] & { nativeDelivery: boolean }): boolean =>
  !nativeDelivery && shouldPlayNotificationSound(rest);

/**
 * Ask the OS for permission once, only while notifications are on and the
 * OS has never been asked (`prompt`, which macOS reports as `default`). A
 * denied permission is the user's answer and is never asked again here.
 */
export const shouldRequestNotificationPermission = ({
  showNotifications,
  permission,
  alreadyAsked,
}: {
  showNotifications: boolean;
  permission: string | undefined;
  alreadyAsked: boolean;
}): boolean =>
  showNotifications &&
  !alreadyAsked &&
  (permission === 'prompt' || permission === 'prompt-with-rationale');

type ApprovalDecisionName = 'approve_once' | 'approve_always' | 'deny';

const APPROVAL_DECISION_LABELS: Record<ApprovalDecisionName, string> = {
  approve_once: 'approved',
  approve_always: 'always approved',
  deny: 'denied',
};

/** What an approval action from a banner ended as. */
export type ApprovalBannerActionResult =
  | { status: 'applied' }
  | {
      status: 'already_decided';
      clicked?: ApprovalDecisionName;
      recorded?: ApprovalDecisionName;
    }
  | { status: 'failed'; error: unknown };

const nativeErrorCode = (error: unknown): string | undefined => {
  if (!error || typeof error !== 'object') return undefined;
  const { code } = error as { code?: unknown };
  return typeof code === 'string' ? code : undefined;
};

/**
 * The local notice an approval action from a banner leaves behind, so a
 * click from the tray is never silent: a failure says the decision was not
 * sent (or may still be sent, after a timeout), and a prompt decided on
 * another device says so when that differs from the click. `undefined`
 * when the click simply worked.
 */
export const approvalBannerActionNotice = (
  result: ApprovalBannerActionResult
): { title: string; body: string } | undefined => {
  if (result.status === 'applied') return undefined;
  if (result.status === 'already_decided') {
    if (result.recorded && result.recorded === result.clicked) return undefined;
    return {
      title: 'Already decided',
      body: result.recorded
        ? `This request was already ${APPROVAL_DECISION_LABELS[result.recorded]} on another device.`
        : 'This request was already decided on another device.',
    };
  }
  if (nativeErrorCode(result.error) === 'ApprovalDecisionPending') {
    return {
      title: 'Approval may still be sent',
      body: 'Synara could not confirm your decision in time. Check the request before deciding again.',
    };
  }
  return {
    title: 'Approval not sent',
    body: 'Open Synara to review the command. It may have expired.',
  };
};

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

const MAX_INVITE_BANNER_KEYS = 8;

/**
 * Tracks which invites joined or left the list between snapshots. A banner
 * for new invites is tagged `invite:<roomId>` per new invite (at most 8, the
 * shell's limit), and withdrawn once one of them is accepted, declined or
 * gone.
 */
export const createInviteBannerTracker = () => {
  let known = new Set<string>();
  return (roomIds: readonly string[]) => {
    const next = new Set(roomIds);
    const added = roomIds.filter((roomId) => !known.has(roomId));
    const removed = [...known].filter((roomId) => !next.has(roomId));
    known = next;
    return {
      bannerKeys: added.slice(0, MAX_INVITE_BANNER_KEYS).map((roomId) => `invite:${roomId}`),
      withdrawKeys: removed.map((roomId) => `invite:${roomId}`),
    };
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
