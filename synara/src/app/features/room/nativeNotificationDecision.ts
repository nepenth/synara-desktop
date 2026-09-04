import { NotificationType } from '../../../types/matrix/room';
import { invokeDesktopWithAvailability, type DesktopInvokeResult } from '../../utils/desktop';

/**
 * A9 desktop decision stream — Core owns the suppress/show policy.
 *
 * The renderer only observes facts (focused room, event identity, room mode
 * resolved through the existing push-rule snapshots, explicit user mentions,
 * sender-is-self) and delivers shown candidates through the platform facade.
 * It never branches on mute/mention policy itself: `matrix_notification_decide`
 * is the single decider. There is no room-list polling and no TS mute matcher
 * on this path.
 */

export type NativeNotificationDecisionKind =
  | 'message'
  | 'invite'
  | 'agent_approval'
  | 'later_reminder';

export type NativeNotificationRoomMode = 'all' | 'mentions' | 'mute' | 'default';

export type NativeNotificationDecision = 'show' | 'suppress';

export type NativeNotificationCandidate = {
  candidateId: string;
  roomId: string;
  eventId?: string;
  kind: NativeNotificationDecisionKind;
  title: string;
  body: string;
  route?: string;
  suppressIfFocusedRoom: boolean;
  isEncrypted: boolean;
};

export type NativeNotificationDecisionReadback = {
  decision: NativeNotificationDecision;
  reason?: string;
  candidate?: NativeNotificationCandidate;
};

export type NativeNotificationDecideInput = {
  roomId: string;
  eventId?: string;
  kind: NativeNotificationDecisionKind;
  title: string;
  body: string;
  route?: string;
  suppressIfFocusedRoom?: boolean;
  isEncrypted?: boolean;
  roomMode: NativeNotificationRoomMode;
  highlight?: boolean;
  isOwnEvent?: boolean;
};

export type NativeNotificationInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<NativeNotificationDecisionReadback>>;

/** Map the existing renderer push-rule reading to the closed Core mode vocabulary. */
export function notificationRoomModeForType(type: NotificationType): NativeNotificationRoomMode {
  switch (type) {
    case NotificationType.AllMessages:
      return 'all';
    case NotificationType.MentionsAndKeywords:
      return 'mentions';
    case NotificationType.Mute:
      return 'mute';
    default:
      return 'default';
  }
}

/**
 * Explicit user-mention observation from Matrix `m.mentions` content.
 * `@room` and keyword highlights are push-rule evaluations the renderer cannot
 * observe without a JS highlight engine; they resolve in the Core-side
 * sync-subscription follow-up. `default`/`all` modes do not need highlight.
 */
export function eventMentionsUser(
  content: Record<string, unknown> | null | undefined,
  userId: string | null | undefined
): boolean {
  if (!content || typeof content !== 'object' || !userId) return false;
  const mentions = content['m.mentions'];
  if (!mentions || typeof mentions !== 'object') return false;
  const userIds = (mentions as Record<string, unknown>).user_ids;
  return Array.isArray(userIds) && userIds.includes(userId);
}

function acceptsNativeNotificationDecisionReadback(
  value: NativeNotificationDecisionReadback
): boolean {
  if (value.decision !== 'show' && value.decision !== 'suppress') return false;
  if (value.decision === 'show') {
    const candidate = value.candidate;
    if (!candidate || typeof candidate.candidateId !== 'string' || !candidate.candidateId) {
      return false;
    }
    if (typeof candidate.roomId !== 'string' || !candidate.roomId) return false;
  }
  if (value.decision === 'suppress' && value.candidate !== undefined) return false;
  return true;
}

const defaultInvoke: NativeNotificationInvoke = (command, args) =>
  invokeDesktopWithAvailability<NativeNotificationDecisionReadback>(command, args);

/** Core suppress/show decision for one observed event. There is no TS fallback. */
export async function decideNotificationWithNativeOwner(
  input: NativeNotificationDecideInput,
  invoke: NativeNotificationInvoke = defaultInvoke
): Promise<NativeNotificationDecisionReadback> {
  const result = await invoke('matrix_notification_decide', { request: input });
  if (!result.available || !result.value) {
    throw new Error('Native notification decisions are unavailable.');
  }
  if (!acceptsNativeNotificationDecisionReadback(result.value)) {
    throw new Error('Native notification decision did not match the closed vocabulary.');
  }
  return result.value;
}

/** Record the platform-observed focused room in Core (null clears focus). */
export async function setNotificationFocusWithNativeOwner(
  roomId: string | null,
  invoke: (
    command: string,
    args?: Record<string, unknown>
  ) => Promise<DesktopInvokeResult<unknown>> = invokeDesktopWithAvailability
): Promise<void> {
  const result = await invoke('matrix_notification_focus_set', { roomId });
  if (!result.available) {
    throw new Error('Native notification focus is unavailable.');
  }
}

/** Acknowledge a delivered or dismissed candidate. Dedup memory is retained. */
export async function dismissNotificationWithNativeOwner(
  candidateId: string,
  invoke: (
    command: string,
    args?: Record<string, unknown>
  ) => Promise<DesktopInvokeResult<boolean>> = invokeDesktopWithAvailability
): Promise<boolean> {
  const result = await invoke('matrix_notification_dismiss', { candidateId });
  if (!result.available || typeof result.value !== 'boolean') {
    throw new Error('Native notification dismiss is unavailable.');
  }
  return result.value;
}
