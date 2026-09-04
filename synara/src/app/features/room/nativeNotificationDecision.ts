import { NotificationType } from '../../../types/matrix/room';
import { invokeDesktopWithAvailability, type DesktopInvokeResult } from '../../utils/desktop';

/**
 * A9 desktop decision stream — Core owns the suppress/show policy.
 *
 * The renderer only observes facts (focused room, event identity, room mode
 * resolved through the Core room-notification / push-rules snapshots, highlight
 * signals from `m.mentions` plus unencrypted keyword/@room/display-name
 * observation, sender-is-self) and delivers shown candidates through the
 * platform facade. It never branches on mute/mention policy itself:
 * `matrix_notification_decide` is the single decider. There is no JS
 * `getNotificationType` / push-rule stub on this path, no room-list mute
 * matcher, and no message body sent to Core.
 */

export type NativeNotificationDecisionKind =
  | 'message'
  | 'invite'
  | 'agent_approval'
  | 'later_reminder';

export type NativeNotificationRoomMode = 'all' | 'mentions' | 'mute' | 'default';

/** Resolved mode the live native path sends to Core. Never `'default'`. */
export type NativeNotificationResolvedRoomMode = 'all' | 'mentions' | 'mute';

/** Account-default modes from `matrix_push_rules_snapshot`. */
export type NativeAccountNotificationDefaults = {
  dm: NativeNotificationResolvedRoomMode;
  dmEncrypted: NativeNotificationResolvedRoomMode;
  group: NativeNotificationResolvedRoomMode;
  groupEncrypted: NativeNotificationResolvedRoomMode;
};

export type NativeHighlightFlags = {
  userMention: boolean;
  displayName: boolean;
  userName: boolean;
  roomMention: boolean;
  atRoom: boolean;
};

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

function mentionsRoom(content: Record<string, unknown> | null | undefined): boolean {
  if (!content || typeof content !== 'object') return false;
  const mentions = content['m.mentions'];
  if (!mentions || typeof mentions !== 'object') return false;
  return (mentions as Record<string, unknown>).room === true;
}

function isWordChar(ch: string | undefined): boolean {
  return typeof ch === 'string' && ch.length === 1 && /[\p{L}\p{N}_]/u.test(ch);
}

/** Case-insensitive token match that does not fire inside another word. */
export function notificationBodyContainsToken(body: string, token: string): boolean {
  const needle = token.trim();
  if (!needle) return false;
  const hay = body.toLocaleLowerCase();
  const find = needle.toLocaleLowerCase();
  let from = 0;
  while (from <= hay.length - find.length) {
    const idx = hay.indexOf(find, from);
    if (idx === -1) return false;
    const before = idx === 0 ? undefined : hay[idx - 1];
    const after = hay[idx + find.length];
    if (!isWordChar(before) && !isWordChar(after)) return true;
    from = idx + 1;
  }
  return false;
}

function closedResolvedMode(
  mode: string | null | undefined
): NativeNotificationResolvedRoomMode | undefined {
  if (mode === 'all' || mode === 'mentions' || mode === 'mute') return mode;
  return undefined;
}

/**
 * Inherit the account default for a room without a user-defined override.
 * Missing defaults fail closed to mentions (never notify-all).
 */
export function effectiveNotificationRoomMode(input: {
  userDefined?: NativeNotificationRoomMode | null;
  isEncrypted: boolean;
  isDirect: boolean;
  defaults?: NativeAccountNotificationDefaults | null;
}): NativeNotificationResolvedRoomMode {
  const override = closedResolvedMode(input.userDefined);
  if (override) return override;
  const defaults = input.defaults;
  if (!defaults) return 'mentions';
  if (input.isDirect) return input.isEncrypted ? defaults.dmEncrypted : defaults.dm;
  return input.isEncrypted ? defaults.groupEncrypted : defaults.group;
}

/**
 * Live native mode resolution: per-room Core override, else account default,
 * else the room-list effective mode, else mentions. Never returns `'default'`.
 */
export function resolveObservedNotificationRoomMode(input: {
  userDefined?: NativeNotificationRoomMode | null;
  listMode?: NativeNotificationRoomMode | null;
  isEncrypted: boolean;
  isDirect: boolean;
  defaults?: NativeAccountNotificationDefaults | null;
}): NativeNotificationResolvedRoomMode {
  const override = closedResolvedMode(input.userDefined);
  if (override) return override;
  if (input.defaults) {
    return effectiveNotificationRoomMode({
      userDefined: 'default',
      isEncrypted: input.isEncrypted,
      isDirect: input.isDirect,
      defaults: input.defaults,
    });
  }
  return closedResolvedMode(input.listMode) ?? 'mentions';
}

export function roomOverrideMapFromSnapshots(
  rooms: ReadonlyArray<{ roomId: string; mode: string }>
): Map<string, NativeNotificationResolvedRoomMode> {
  const map = new Map<string, NativeNotificationResolvedRoomMode>();
  for (const room of rooms) {
    const mode = closedResolvedMode(room.mode);
    if (mode) map.set(room.roomId, mode);
  }
  return map;
}

/**
 * Shell highlight observation. Ciphertext events (`isEncrypted`) only honor
 * explicit `m.mentions`; body/keyword/display-name matching never runs on
 * ciphertext and the body is never sent to Core.
 */
export function eventIsHighlightObservation(input: {
  content: Record<string, unknown> | null | undefined;
  userId: string | null | undefined;
  isEncrypted: boolean;
  body?: string | null;
  keywords?: readonly string[] | null;
  displayName?: string | null;
  localpart?: string | null;
  flags?: NativeHighlightFlags | null;
}): boolean {
  const flags = input.flags ?? {
    userMention: true,
    displayName: false,
    userName: false,
    roomMention: true,
    atRoom: false,
  };
  if (flags.userMention && eventMentionsUser(input.content, input.userId)) return true;
  if ((flags.roomMention || flags.atRoom) && mentionsRoom(input.content)) return true;
  if (input.isEncrypted) return false;

  const body = typeof input.body === 'string' ? input.body : '';
  if (!body) return false;
  if ((flags.roomMention || flags.atRoom) && notificationBodyContainsToken(body, '@room')) {
    return true;
  }
  if (
    flags.displayName &&
    input.displayName &&
    notificationBodyContainsToken(body, input.displayName)
  ) {
    return true;
  }
  if (flags.userName && input.localpart) {
    if (
      notificationBodyContainsToken(body, `@${input.localpart}`) ||
      notificationBodyContainsToken(body, input.localpart)
    ) {
      return true;
    }
  }
  return Boolean(input.keywords?.some((keyword) => notificationBodyContainsToken(body, keyword)));
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
