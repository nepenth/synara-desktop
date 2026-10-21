import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { isSynaraDesktop } from '../../utils/desktop';
import type {
  AgentApprovalNotificationSummary as WireAgentApprovalNotificationSummary,
  NativeAgentApprovalObservation as WireNativeAgentApprovalObservation,
  NativeNotificationObservation as WireNativeNotificationObservation,
} from '../matrix-dto/generated';
import type { NullsToOptional } from '../matrix-dto/wireTypes';

export type AgentApprovalNotificationSummary =
  NullsToOptional<WireAgentApprovalNotificationSummary>;

export type NativeAgentApprovalObservation = Omit<WireNativeAgentApprovalObservation, 'summary'> & {
  summary: AgentApprovalNotificationSummary;
};

/** Parsed form of Core's `NativeNotificationObservation`: absent instead of `null`. */
export type NativeNotificationObservation = Omit<
  NullsToOptional<WireNativeNotificationObservation>,
  'agentApproval'
> & { agentApproval?: NativeAgentApprovalObservation };

/**
 * A9 Core→renderer notification observation stream.
 *
 * Core observes every message-like timeline event the SDK sync delivers for
 * the bound session and pushes one bounded observation per candidate on the
 * `matrix-notification-observed` event. The renderer hands the identity back
 * through `matrix_notification_decide`; the SDK push rules inside Core still
 * decide. There is no renderer timeline scan, no `Room.timeline` listener,
 * and no sync-state gate on this path.
 *
 * An observation carries identity (`roomId`, `eventId`, `sender`), the event
 * type, its origin timestamp, and Core syntax/sender/expiry classification.
 * Classification does not establish terminal reaction state; Core validates
 * current reactions when an action is submitted. Push verdicts remain private: any `highlight`, `sound`, `notify`, or mode key
 * is rejected here so the wire cannot grow one silently.
 */

export const NOTIFICATION_OBSERVED_EVENT = 'matrix-notification-observed';

export type NativeNotificationObservationEventType =
  'm.room.message' | 'm.room.encrypted' | 'm.sticker';

const OBSERVATION_KEYS = new Set([
  'sessionGeneration',
  'roomId',
  'eventId',
  'sender',
  'eventType',
  'originServerTs',
  'agentApproval',
]);

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

const isSafeGeneration = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;

const APPROVAL_KEYS = new Set(['expiresAt', 'expired', 'summary']);
const SUMMARY_KEYS = new Set(['reason', 'command']);
// Core bounds these far lower; this only rejects a malformed wire.
const MAX_SUMMARY_FIELD_CHARS = 512;

const parseSummaryField = (value: unknown): string | undefined | false => {
  if (value === undefined || value === null) return undefined;
  if (typeof value !== 'string' || value.length > MAX_SUMMARY_FIELD_CHARS) return false;
  return value;
};

const parseAgentApproval = (value: unknown): NativeAgentApprovalObservation | undefined => {
  if (!isRecord(value) || !Object.keys(value).every((key) => APPROVAL_KEYS.has(key)))
    return undefined;
  const { expiresAt, expired, summary } = value;
  if (!isSafeGeneration(expiresAt) || typeof expired !== 'boolean') return undefined;
  if (!isRecord(summary) || !Object.keys(summary).every((key) => SUMMARY_KEYS.has(key)))
    return undefined;
  const reason = parseSummaryField(summary.reason);
  const command = parseSummaryField(summary.command);
  if (reason === false || command === false) return undefined;
  return {
    expiresAt,
    expired,
    summary: {
      ...(reason !== undefined ? { reason } : {}),
      ...(command !== undefined ? { command } : {}),
    },
  };
};

const isEventType = (value: unknown): value is NativeNotificationObservationEventType =>
  value === 'm.room.message' || value === 'm.room.encrypted' || value === 'm.sticker';

/** Fail-closed parser: unknown keys, missing identity, or a verdict reject. */
export const parseNativeNotificationObservation = (
  value: unknown
): NativeNotificationObservation | undefined => {
  if (!isRecord(value)) return undefined;
  if (!Object.keys(value).every((key) => OBSERVATION_KEYS.has(key))) return undefined;
  const { sessionGeneration, roomId, eventId, sender, eventType, originServerTs, agentApproval } =
    value;
  if (!isSafeGeneration(sessionGeneration)) return undefined;
  if (typeof roomId !== 'string' || !roomId.startsWith('!')) return undefined;
  if (typeof eventId !== 'string' || !eventId.startsWith('$')) return undefined;
  if (typeof sender !== 'string' || !sender.startsWith('@')) return undefined;
  if (!isEventType(eventType)) return undefined;
  if (!isSafeGeneration(originServerTs)) return undefined;
  let parsedApproval: NativeAgentApprovalObservation | undefined;
  if (agentApproval !== undefined && agentApproval !== null) {
    parsedApproval = parseAgentApproval(agentApproval);
    if (!parsedApproval) return undefined;
  }
  return {
    sessionGeneration,
    roomId,
    eventId,
    sender,
    eventType,
    originServerTs,
    ...(parsedApproval ? { agentApproval: parsedApproval } : {}),
  };
};

export type NativeNotificationObservationListen = <T>(
  event: string,
  handler: (event: { payload: T }) => void
) => Promise<UnlistenFn>;

export type NativeNotificationObservationDependencies = {
  desktopAvailable: boolean;
  listen: NativeNotificationObservationListen;
};

const defaultDependencies = (): NativeNotificationObservationDependencies => ({
  desktopAvailable: isSynaraDesktop(),
  listen: (event, handler) => listen(event, handler),
});

/**
 * Subscribe to the Core observation stream for the current session.
 *
 * `expectedGeneration` is read per observation; an observation from another
 * session generation (account switch, re-login) or from before the renderer
 * knows its generation is dropped, never decided. Returns a disposer.
 */
export const subscribeNativeNotificationObservations = (
  expectedGeneration: () => number | undefined,
  onObservation: (observation: NativeNotificationObservation) => void,
  dependencies: NativeNotificationObservationDependencies = defaultDependencies()
): (() => void) => {
  let disposed = false;
  let unlisten: UnlistenFn | undefined;

  if (!dependencies.desktopAvailable) {
    return () => {
      disposed = true;
    };
  }

  dependencies
    .listen<unknown>(NOTIFICATION_OBSERVED_EVENT, (event) => {
      if (disposed) return;
      const observation = parseNativeNotificationObservation(event.payload);
      if (!observation) return;
      const generation = expectedGeneration();
      if (generation === undefined || observation.sessionGeneration !== generation) return;
      onObservation(observation);
    })
    .then((nextUnlisten) => {
      if (disposed) {
        nextUnlisten();
        return;
      }
      unlisten = nextUnlisten;
    })
    .catch(() => undefined);

  return () => {
    disposed = true;
    if (unlisten) {
      unlisten();
      unlisten = undefined;
    }
  };
};
