/**
 * SDK-neutral owners for retained timeline actions.
 *
 * Reply/rich send use `matrix_send_text` (optional `formattedBody` + `replyTo`).
 * Reactions use the merged V-SEND.2 owner (`nativeReactionOwner`). These helpers
 * are consumed by NativeTimelinePresenter after V-TIMELINE.C1/C2 cutover
 * (JS RoomTimeline deleted).
 */

import type { DesktopInvokeResult } from '../../utils/desktop';

/**
 * Choose the native forward owner for a product timeline row.
 * Core explicitly projects `media` or `text`; missing and unknown values fail closed.
 */
export function isNativeTimelineForwardTransport(
  transport?: string
): transport is 'text' | 'media' {
  return transport === 'text' || transport === 'media';
}

export function isNativeTimelineForwardMedia(transport?: string): boolean {
  return transport === 'media';
}

/**
 * Choose pin vs unpin from room pin-list readback.
 * Capability `pin` only means the action is allowed; state comes from `pinnedEventIds`.
 */
export function selectNativeTimelinePinAction(pinned: boolean): 'pin' | 'unpin' {
  return pinned ? 'unpin' : 'pin';
}

export const NATIVE_TIMELINE_ACTION_SCHEMA_VERSION = 1;

export type NativeTimelineActionKind =
  | 'edit_text'
  | 'redact'
  | 'forward_text'
  | 'forward_media'
  | 'report'
  | 'pin'
  | 'unpin'
  | 'poll_vote'
  | 'call_decline';

export type NativeTimelineActionStatus =
  | 'sent'
  | 'redacted'
  | 'reported'
  | 'pinned'
  | 'unpinned'
  | 'already_pinned'
  | 'already_unpinned'
  | 'voted'
  | 'declined';

export type NativeTimelineActionReadback = {
  schemaVersion: number;
  action: NativeTimelineActionKind;
  roomId: string;
  eventId: string;
  status: NativeTimelineActionStatus;
};

export type NativeTimelineEditTextInput = {
  roomId: string;
  eventId: string;
  body: string;
  formattedBody?: string;
};

export type NativeTimelineRedactInput = {
  roomId: string;
  eventId: string;
  reason?: string;
};

export type NativeTimelineForwardTextInput = {
  sourceRoomId: string;
  eventId: string;
  targetRoomId: string;
  asQuote?: boolean;
  confirmedEncryptionDowngrade: boolean;
};

export type NativeTimelineForwardMediaInput = {
  sourceRoomId: string;
  eventId: string;
  targetRoomId: string;
  confirmedEncryptionDowngrade: boolean;
};

export type NativeTimelineReportInput = {
  roomId: string;
  eventId: string;
  reason?: string;
};

export type NativeTimelinePinInput = {
  roomId: string;
  eventId: string;
};

export type NativeTimelinePollVoteInput = {
  roomId: string;
  eventId: string;
  answerIds?: string[];
};

export type NativeTimelineCallDeclineInput = {
  roomId: string;
  eventId: string;
};

export function toggleNativePollSelection(
  current: ReadonlySet<string>,
  answerId: string,
  availableAnswerIds: ReadonlySet<string>,
  maximumSelections: number
): Set<string> {
  const next = new Set([...current].filter((id) => availableAnswerIds.has(id)));
  if (!availableAnswerIds.has(answerId) || maximumSelections <= 0) return next;
  if (maximumSelections === 1) return next.has(answerId) ? new Set() : new Set([answerId]);
  if (next.has(answerId)) next.delete(answerId);
  else if (next.size < maximumSelections) next.add(answerId);
  return next;
}

export function nativePollSubmission(
  selection: ReadonlySet<string>,
  original: ReadonlySet<string>,
  availableAnswerIds: ReadonlySet<string>,
  maximumSelections: number,
  canVote: boolean,
  closed: boolean
): string[] | undefined {
  if (!canVote || closed || maximumSelections <= 0) return undefined;
  const bounded = [...selection].filter((id) => availableAnswerIds.has(id)).sort();
  if (bounded.length > maximumSelections) return undefined;
  const prior = [...original].filter((id) => availableAnswerIds.has(id)).sort();
  if (bounded.length === prior.length && bounded.every((id, index) => id === prior[index])) {
    return undefined;
  }
  return bounded;
}

type NativePollFlight = {
  expectedSelection: string;
  dispatchSettled: boolean;
  projectionObserved: boolean;
};

/**
 * Coordinates poll writes whose typed command readback and authoritative
 * timeline projection can arrive in either order. The owner keeps the action
 * locked until both phases complete and clears all pending state when Core's
 * session generation changes.
 */
export class NativePollFlightCoordinator {
  private sessionGeneration?: number;
  private readonly flights = new Map<string, NativePollFlight>();

  bindSession(sessionGeneration: number): void {
    if (this.sessionGeneration === sessionGeneration) return;
    this.sessionGeneration = sessionGeneration;
    this.flights.clear();
  }

  prepare(key: string, answerIds: readonly string[]): void {
    this.flights.set(key, {
      expectedSelection: [...answerIds].sort().join('\u0000'),
      dispatchSettled: false,
      projectionObserved: false,
    });
  }

  observeProjection(key: string, answerIds: readonly string[]): boolean {
    const flight = this.flights.get(key);
    if (!flight || flight.expectedSelection !== [...answerIds].sort().join('\u0000')) return false;
    flight.projectionObserved = true;
    return this.finishIfComplete(key, flight);
  }

  settleDispatch(key: string, succeeded: boolean): boolean {
    const flight = this.flights.get(key);
    if (!flight) return false;
    if (!succeeded) {
      this.flights.delete(key);
      return true;
    }
    flight.dispatchSettled = true;
    return this.finishIfComplete(key, flight);
  }

  has(key: string): boolean {
    return this.flights.has(key);
  }

  private finishIfComplete(key: string, flight: NativePollFlight): boolean {
    if (!flight.dispatchSettled || !flight.projectionObserved) return false;
    this.flights.delete(key);
    return true;
  }
}

type NativeReactionFlight = {
  reactionKey: string;
  expectedOwn: boolean;
  dispatchSettled: boolean;
  projectionObserved: boolean;
};

/**
 * Holds a reaction toggle across the command acknowledgement / timeline
 * projection race. A successful SDK send may legitimately have no immediate
 * readback, so only the later authoritative row projection completes it.
 */
export class NativeReactionFlightCoordinator {
  private sessionGeneration?: number;
  private readonly flights = new Map<string, NativeReactionFlight>();

  bindSession(sessionGeneration: number): void {
    if (this.sessionGeneration === sessionGeneration) return;
    this.sessionGeneration = sessionGeneration;
    this.flights.clear();
  }

  prepare(key: string, reactionKey: string, expectedOwn: boolean): void {
    this.flights.set(key, {
      reactionKey,
      expectedOwn,
      dispatchSettled: false,
      projectionObserved: false,
    });
  }

  observeEventProjection(
    actionPrefix: string,
    reactions: readonly { key: string; own?: boolean }[]
  ): string[] {
    const completed: string[] = [];
    for (const [key, flight] of this.flights) {
      if (!key.startsWith(actionPrefix)) continue;
      const projected = reactions.find((reaction) => reaction.key === flight.reactionKey);
      if (projected !== undefined && projected.own === undefined) continue;
      const owns = projected?.own ?? false;
      if (owns !== flight.expectedOwn) continue;
      flight.projectionObserved = true;
      if (this.finishIfComplete(key, flight)) completed.push(key);
    }
    return completed;
  }

  settleDispatch(key: string, succeeded: boolean): boolean {
    const flight = this.flights.get(key);
    if (!flight) return false;
    if (!succeeded) {
      this.flights.delete(key);
      return true;
    }
    flight.dispatchSettled = true;
    return this.finishIfComplete(key, flight);
  }

  has(key: string): boolean {
    return this.flights.has(key);
  }

  private finishIfComplete(key: string, flight: NativeReactionFlight): boolean {
    if (!flight.dispatchSettled || !flight.projectionObserved) return false;
    this.flights.delete(key);
    return true;
  }
}

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

const ACTION_STATUSES = new Set<NativeTimelineActionStatus>([
  'sent',
  'redacted',
  'reported',
  'pinned',
  'unpinned',
  'already_pinned',
  'already_unpinned',
  'voted',
  'declined',
]);

const acceptActionReadback = (
  value: unknown,
  expected: {
    action: NativeTimelineActionKind;
    roomId: string;
    statuses: ReadonlySet<NativeTimelineActionStatus>;
    eventId?: string;
  }
): NativeTimelineActionReadback | undefined => {
  if (!value || typeof value !== 'object') return undefined;
  const readback = value as NativeTimelineActionReadback;
  if (
    readback.schemaVersion !== NATIVE_TIMELINE_ACTION_SCHEMA_VERSION ||
    readback.action !== expected.action ||
    readback.roomId !== expected.roomId ||
    typeof readback.eventId !== 'string' ||
    readback.eventId.length === 0 ||
    !ACTION_STATUSES.has(readback.status) ||
    !expected.statuses.has(readback.status) ||
    (expected.eventId !== undefined && readback.eventId !== expected.eventId)
  ) {
    return undefined;
  }
  return readback;
};

export async function editTextWithNativeTimelineOwner(
  input: NativeTimelineEditTextInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeTimelineActionReadback | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_timeline_edit_text', { request: input });
  if (!result.available) return 'unavailable';
  return (
    acceptActionReadback(result.value, {
      action: 'edit_text',
      roomId: input.roomId,
      statuses: new Set(['sent']),
    }) ?? 'unavailable'
  );
}

export async function redactWithNativeTimelineOwner(
  input: NativeTimelineRedactInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeTimelineActionReadback | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_timeline_redact', { request: input });
  if (!result.available) return 'unavailable';
  return (
    acceptActionReadback(result.value, {
      action: 'redact',
      roomId: input.roomId,
      statuses: new Set(['redacted']),
      eventId: input.eventId,
    }) ?? 'unavailable'
  );
}

export async function forwardTextWithNativeTimelineOwner(
  input: NativeTimelineForwardTextInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeTimelineActionReadback | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_timeline_forward_text', { request: input });
  if (!result.available) return 'unavailable';
  return (
    acceptActionReadback(result.value, {
      action: 'forward_text',
      roomId: input.targetRoomId,
      statuses: new Set(['sent']),
    }) ?? 'unavailable'
  );
}

export async function forwardMediaWithNativeTimelineOwner(
  input: NativeTimelineForwardMediaInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeTimelineActionReadback | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_timeline_forward_media', { request: input });
  if (!result.available) return 'unavailable';
  return (
    acceptActionReadback(result.value, {
      action: 'forward_media',
      roomId: input.targetRoomId,
      statuses: new Set(['sent']),
    }) ?? 'unavailable'
  );
}

export async function reportWithNativeTimelineOwner(
  input: NativeTimelineReportInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeTimelineActionReadback | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_timeline_report', { request: input });
  if (!result.available) return 'unavailable';
  return (
    acceptActionReadback(result.value, {
      action: 'report',
      roomId: input.roomId,
      statuses: new Set(['reported']),
      eventId: input.eventId,
    }) ?? 'unavailable'
  );
}

export async function pinWithNativeTimelineOwner(
  input: NativeTimelinePinInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeTimelineActionReadback | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_timeline_pin', { request: input });
  if (!result.available) return 'unavailable';
  return (
    acceptActionReadback(result.value, {
      action: 'pin',
      roomId: input.roomId,
      statuses: new Set(['pinned', 'already_pinned']),
      eventId: input.eventId,
    }) ?? 'unavailable'
  );
}

export async function unpinWithNativeTimelineOwner(
  input: NativeTimelinePinInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeTimelineActionReadback | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_timeline_unpin', { request: input });
  if (!result.available) return 'unavailable';
  return (
    acceptActionReadback(result.value, {
      action: 'unpin',
      roomId: input.roomId,
      statuses: new Set(['unpinned', 'already_unpinned']),
      eventId: input.eventId,
    }) ?? 'unavailable'
  );
}

export async function pollVoteWithNativeTimelineOwner(
  input: NativeTimelinePollVoteInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeTimelineActionReadback | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_timeline_poll_vote', { request: input });
  if (!result.available) return 'unavailable';
  return (
    acceptActionReadback(result.value, {
      action: 'poll_vote',
      roomId: input.roomId,
      statuses: new Set(['voted']),
    }) ?? 'unavailable'
  );
}

export async function callDeclineWithNativeTimelineOwner(
  input: NativeTimelineCallDeclineInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeTimelineActionReadback | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_timeline_call_decline', { request: input });
  if (!result.available) return 'unavailable';
  return (
    acceptActionReadback(result.value, {
      action: 'call_decline',
      roomId: input.roomId,
      statuses: new Set(['declined']),
    }) ?? 'unavailable'
  );
}
