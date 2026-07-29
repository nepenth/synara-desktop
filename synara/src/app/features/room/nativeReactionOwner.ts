import { invokeDesktopWithAvailability } from '../../utils/desktop';

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

type ReactionInput = {
  roomId: string;
  eventId: string;
  key: string;
};

async function invokeNativeReaction(
  command: string,
  args: Record<string, string>
): Promise<NativeReactionMutationResult> {
  const result = await invokeDesktopWithAvailability<NativeReactionMutationResult>(command, args);
  if (!result.available || !result.value) {
    throw new Error('Native Matrix reactions are unavailable.');
  }
  return result.value;
}

/** Native timeline-owned self reaction add/remove. There is no JS SDK fallback. */
export function toggleReactionWithNativeOwner(input: ReactionInput) {
  return invokeNativeReaction('matrix_timeline_reaction_toggle', input);
}

/** Native idempotent add used by approval controls; never implemented as a toggle. */
export function ensureReactionWithNativeOwner(input: ReactionInput) {
  return invokeNativeReaction('matrix_reaction_ensure', input);
}

/** Native arbitrary annotation redaction used by the reaction viewer. */
export function redactReactionWithNativeOwner(input: ReactionInput & { reactionEventId: string }) {
  const { eventId, reactionEventId, ...rest } = input;
  return invokeNativeReaction('matrix_reaction_redact', {
    ...rest,
    targetEventId: eventId,
    reactionEventId,
  });
}
