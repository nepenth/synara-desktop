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
    invoke,
    input,
    new Set(['redacted'])
  );
}
