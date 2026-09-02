import { invokeDesktopWithAvailability, type DesktopInvokeResult } from '../../utils/desktop';

export type NativeReactionMutation = 'added' | 'removed' | 'already_present' | 'redacted';

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

const defaultInvoke: NativeReactionInvoke = (command, args) =>
  invokeDesktopWithAvailability<NativeReactionMutationResult>(command, args);
const defaultAgentApprovalInvoke: NativeAgentApprovalInvoke = (command, args) =>
  invokeDesktopWithAvailability<NativeAgentApprovalDecisionResult>(command, args);

async function invokeNativeReaction(
  command: string,
  args: Record<string, string>,
  invoke: NativeReactionInvoke
): Promise<NativeReactionMutationResult> {
  const result = await invoke(command, args);
  if (!result.available || !result.value) {
    throw new Error('Native Matrix reactions are unavailable.');
  }
  return result.value;
}

/** Native timeline-owned self reaction add/remove. There is no JS SDK fallback. */
export function toggleReactionWithNativeOwner(
  input: ReactionInput,
  invoke: NativeReactionInvoke = defaultInvoke
) {
  return invokeNativeReaction('matrix_timeline_reaction_toggle', input, invoke);
}

/** Native idempotent add used by approval controls; never implemented as a toggle. */
export function ensureReactionWithNativeOwner(
  input: ReactionInput,
  invoke: NativeReactionInvoke = defaultInvoke
) {
  return invokeNativeReaction('matrix_reaction_ensure', input, invoke);
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
  },
  invoke: NativeAgentApprovalInvoke = defaultAgentApprovalInvoke
): Promise<NativeAgentApprovalDecisionResult> {
  const result = await invoke('matrix_agent_approval_decide', input);
  if (!result.available || !result.value) {
    throw new Error('Native agent approval decisions are unavailable.');
  }
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
    invoke
  );
}
