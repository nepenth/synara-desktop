import {
  captureApprovalDecisionScope,
  publishApprovalDecision,
  approvalDecisionFromActionId,
  type ApprovalDecision,
} from '../approvals/approvalDecisionEvents';
import { invokeDesktopWithAvailability, type DesktopInvokeResult } from '../../utils/desktop';
import type { NativeReactionMutation } from '../matrix-dto/generated';
export type { NativeReactionMutation } from '../matrix-dto/generated';

export type NativeReactionReadback = {
  key: string;
  count: number;
  me: boolean;
  senders: Array<{ userId: string; reactionEventId?: string }>;
};

export type NativeReactionMutationResult = {
  roomId: string;
  targetEventId: string;
  key: string;
  mutation: NativeReactionMutation;
  readback?: NativeReactionReadback;
};

export type NativeAgentApprovalDecisionResult = {
  roomId: string;
  eventId: string;
  status: 'applied' | 'already_decided';
  reaction?: NativeReactionMutationResult;
  /**
   * The decision already on record when `status` is `already_decided`, and
   * when it was made. Absent when Core cannot tell; the caller then shows no
   * decision.
   */
  decision?: ApprovalDecision;
  decidedAt?: number;
};

const APPROVAL_DECISIONS: ReadonlySet<string> = new Set(['approve_once', 'approve_always', 'deny']);

/**
 * The decision to record for a successful call. `applied` is the clicked
 * choice. `already_decided` means another click (often on another device)
 * won; the clicked choice was not sent, so only Core's recorded decision may
 * be shown, and nothing when Core does not report one. Callers then read the
 * authoritative state from Core (approval inbox, timeline row).
 */
export const recordedApprovalDecision = (
  result: Pick<NativeAgentApprovalDecisionResult, 'status' | 'decision'>,
  actionId: string
): ApprovalDecision | undefined => {
  if (result.status === 'applied') return approvalDecisionFromActionId(actionId);
  const reported = result.decision;
  return typeof reported === 'string' && APPROVAL_DECISIONS.has(reported) ? reported : undefined;
};

export type NativeReactionInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<NativeReactionMutationResult>>;

export type NativeAgentApprovalInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<NativeAgentApprovalDecisionResult>>;

type ReactionInput = {
  roomId: string;
  eventId: string;
  key: string;
};

type ToggleReactionInput = ReactionInput & {
  /** Exact ownership expected after applying the toggle to the projected row. */
  expectedOwn: boolean;
};

function acceptsNativeReactionReadback(
  value: NativeReactionMutationResult,
  input: ReactionInput,
  allowedMutations: ReadonlySet<NativeReactionMutation>,
  expectedOwn?: boolean
): boolean {
  if (
    value.roomId !== input.roomId ||
    value.targetEventId !== input.eventId ||
    value.key !== input.key ||
    !allowedMutations.has(value.mutation)
  ) {
    return false;
  }
  const projected = value.readback;
  if (projected && projected.key !== input.key) return false;
  switch (value.mutation) {
    case 'added':
      if (expectedOwn === false) return false;
      return true;
    case 'already_present':
      return expectedOwn !== false;
    case 'removed':
      return expectedOwn !== true;
    case 'redacted':
      return true;
  }
}

const defaultInvoke: NativeReactionInvoke = (command, args) =>
  invokeDesktopWithAvailability<NativeReactionMutationResult>(command, args);
const defaultAgentApprovalInvoke: NativeAgentApprovalInvoke = (command, args) =>
  invokeDesktopWithAvailability<NativeAgentApprovalDecisionResult>(command, args);

async function invokeNativeReaction(
  command: string,
  args: Record<string, string>,
  invoke: NativeReactionInvoke,
  expected: ReactionInput,
  allowedMutations: ReadonlySet<NativeReactionMutation>,
  expectedOwn?: boolean
): Promise<NativeReactionMutationResult> {
  const result = await invoke(command, args);
  if (!result.available || !result.value) {
    throw new Error('Native Matrix reactions are unavailable.');
  }
  if (!acceptsNativeReactionReadback(result.value, expected, allowedMutations, expectedOwn)) {
    throw new Error('Native Matrix reaction readback did not match the requested action.');
  }
  return result.value;
}

/** Native timeline-owned self reaction add/remove. There is no JS SDK fallback. */
export function toggleReactionWithNativeOwner(
  input: ToggleReactionInput,
  invoke: NativeReactionInvoke = defaultInvoke
) {
  const { expectedOwn, ...request } = input;
  return invokeNativeReaction(
    'matrix_timeline_reaction_toggle',
    request,
    invoke,
    request,
    new Set(['added', 'removed']),
    expectedOwn
  );
}

/** Native idempotent add used by approval controls; never implemented as a toggle. */
export function ensureReactionWithNativeOwner(
  input: ReactionInput,
  invoke: NativeReactionInvoke = defaultInvoke
) {
  return invokeNativeReaction(
    'matrix_reaction_ensure',
    input,
    invoke,
    input,
    new Set(['added', 'already_present'])
  );
}

/**
 * Shared-core approval authority. It resolves the exact event, enforces the
 * five-minute policy and terminal-decision state, then applies at most one
 * notification reaction under the native timeline lock.
 */
export async function decideAgentApprovalWithNativeOwner(
  input: {
    roomId: string;
    eventId: string;
    actionId: string;
    notificationSessionGeneration?: number;
  },
  invoke: NativeAgentApprovalInvoke = defaultAgentApprovalInvoke
): Promise<NativeAgentApprovalDecisionResult> {
  const scope = captureApprovalDecisionScope();
  const result = await invoke('matrix_agent_approval_decide', input);
  if (!result.available || !result.value) {
    throw new Error('Native agent approval decisions are unavailable.');
  }
  if (
    result.value.roomId !== input.roomId ||
    result.value.eventId !== input.eventId ||
    !['applied', 'already_decided'].includes(result.value.status)
  ) {
    throw new Error('Native agent approval readback did not match the requested decision.');
  }
  const decision = recordedApprovalDecision(result.value, input.actionId);
  const decidedAt =
    result.value.status === 'already_decided' &&
    typeof result.value.decidedAt === 'number' &&
    Number.isFinite(result.value.decidedAt) &&
    result.value.decidedAt > 0
      ? result.value.decidedAt
      : undefined;
  publishApprovalDecision({
    scope,
    roomId: input.roomId,
    eventId: input.eventId,
    ...(decision ? { decision } : {}),
    ...(decision && decidedAt !== undefined ? { decidedAt } : {}),
  });
  return result.value;
}

/** Native arbitrary annotation redaction used by the reaction viewer. */
export function redactReactionWithNativeOwner(
  input: ReactionInput & { reactionEventId: string },
  invoke: NativeReactionInvoke = defaultInvoke
) {
  const { eventId, reactionEventId, ...rest } = input;
  return invokeNativeReaction(
    'matrix_reaction_redact',
    {
      ...rest,
      targetEventId: eventId,
      reactionEventId,
    },
    invoke,
    input,
    new Set(['redacted'])
  );
}

export function nativeReactionsForViewer(
  reactions:
    | ReadonlyArray<{
        key: string;
        count: number;
        own?: boolean;
        me?: boolean;
        senders?: NativeReactionReadback['senders'];
      }>
    | undefined
): NativeReactionReadback[] {
  return (reactions ?? []).map((reaction) => ({
    key: reaction.key,
    count: reaction.count,
    me: reaction.me ?? Boolean(reaction.own),
    senders: reaction.senders ?? [],
  }));
}

type NativeEventReadbackValue = {
  roomId?: string;
  eventId?: string;
  item?: { reactions?: NativeReactionReadback[] };
};

/** Viewer-open recovery: event readback fills missing annotation ids via /relations. */
export async function nativeReactionViewFromEventReadback(input: {
  roomId: string;
  eventId: string;
}): Promise<NativeReactionReadback[] | undefined> {
  const result = await invokeDesktopWithAvailability<NativeEventReadbackValue>(
    'matrix_timeline_event_readback',
    { roomId: input.roomId, eventId: input.eventId }
  );
  if (!result.available || !result.value || result.value.roomId !== input.roomId) {
    return undefined;
  }
  const reactions = result.value.item?.reactions;
  if (!Array.isArray(reactions)) return undefined;
  return nativeReactionsForViewer(reactions);
}
