import type {
  AgentQuestionAnswer,
  AgentQuestionChoice,
  AgentQuestionOwnAnswer,
  AgentQuestionSource,
  AgentQuestionState,
} from '../features/matrix-dto/generated';

export type {
  AgentQuestionAnswer,
  AgentQuestionChoice,
  AgentQuestionOwnAnswer,
} from '../features/matrix-dto/generated';

/**
 * Core-recognised Hermes clarifying question on a timeline row
 * (`org.hermes.clarify`, or the plain-text `❓` fallback). Core owns
 * recognition and the answer route; presenters only render.
 */
export type NativeTimelineAgentQuestion = {
  source: AgentQuestionSource;
  clarifyId?: string;
  question: string;
  choices?: AgentQuestionChoice[];
  multiSelect: boolean;
  allowOther: boolean;
  allowSkip: boolean;
  requesterUserId?: string;
  expiresAtMs?: number;
  /** Hermes-settled state from the latest edit; fallback cards stay `pending`. */
  state: AgentQuestionState;
  answer?: string;
  /** False for another requester's card: render it read-only. */
  canAnswer: boolean;
  /** This account's own keycap or skip reaction (structured cards). */
  ownAnswer?: AgentQuestionOwnAnswer;
};

export type AgentQuestionSettlement =
  | { kind: 'pending' }
  | { kind: 'answered'; text?: string }
  | { kind: 'skipped' }
  | { kind: 'expired' }
  | { kind: 'cancelled' };

const NUMBER_LIST = /^\d+(\s*,\s*\d+)*$/;

const choiceLabel = (question: NativeTimelineAgentQuestion, index: number): string =>
  question.choices?.find((choice) => choice.index === index)?.label ?? String(index);

/**
 * Read this account's reply to a card the way Hermes does: "skip", a list
 * of choice numbers ("1, 3"), or free text.
 */
export function describeAgentQuestionReply(
  question: NativeTimelineAgentQuestion,
  body: string
): AgentQuestionSettlement {
  const text = body.trim();
  if (!text) return { kind: 'pending' };
  if (text.toLowerCase() === 'skip' && question.allowSkip) return { kind: 'skipped' };
  if (NUMBER_LIST.test(text) && (question.choices?.length ?? 0) > 0) {
    const labels = text
      .split(',')
      .map((part) => Number.parseInt(part.trim(), 10))
      .map((index) => choiceLabel(question, index));
    return { kind: 'answered', text: labels.join(', ') };
  }
  return { kind: 'answered', text };
}

/** What this device sent; shown until Core or Hermes reports the answer. */
export function describeAgentQuestionAnswer(
  question: NativeTimelineAgentQuestion,
  answer: AgentQuestionAnswer
): AgentQuestionSettlement {
  switch (answer.kind) {
    case 'choice':
      return { kind: 'answered', text: choiceLabel(question, answer.index) };
    case 'choices':
      return {
        kind: 'answered',
        text: [...answer.indices]
          .sort((a, b) => a - b)
          .map((index) => choiceLabel(question, index))
          .join(', '),
      };
    case 'other':
      return { kind: 'answered', text: answer.text.trim() };
    case 'skip':
      return { kind: 'skipped' };
  }
}

/**
 * The card's display state. Hermes' settled state wins; then local expiry;
 * then this account's own answer (reaction, a reply on the timeline, or a
 * send from this device).
 */
export function settleAgentQuestion(
  question: NativeTimelineAgentQuestion,
  context: { now: number; ownReplyBody?: string; localAnswer?: AgentQuestionAnswer }
): AgentQuestionSettlement {
  switch (question.state) {
    case 'answered':
      return { kind: 'answered', text: question.answer };
    case 'skipped':
      return { kind: 'skipped' };
    case 'expired':
      return { kind: 'expired' };
    case 'cancelled':
      return { kind: 'cancelled' };
    case 'pending':
      break;
  }
  const own = question.ownAnswer;
  if (own?.kind === 'skipped') return { kind: 'skipped' };
  if (own?.kind === 'choice') return { kind: 'answered', text: choiceLabel(question, own.index) };
  if (context.localAnswer) return describeAgentQuestionAnswer(question, context.localAnswer);
  if (context.ownReplyBody !== undefined) {
    const reply = describeAgentQuestionReply(question, context.ownReplyBody);
    if (reply.kind !== 'pending') return reply;
  }
  if (question.expiresAtMs !== undefined && question.expiresAtMs <= context.now) {
    return { kind: 'expired' };
  }
  return { kind: 'pending' };
}

/** Toggle a choice index in a multi-select selection, kept sorted. */
export function toggleAgentQuestionChoice(selection: readonly number[], index: number): number[] {
  return selection.includes(index)
    ? selection.filter((value) => value !== index)
    : [...selection, index].sort((a, b) => a - b);
}

/**
 * This account's latest reply to each card, keyed by card event id. Hermes
 * reads the first answer, but the latest one is what the user last saw sent.
 */
export function ownAgentQuestionReplies(
  rows: ReadonlyArray<{
    kind: string;
    senderId?: string;
    body?: string;
    reply?: { eventId: string };
  }>,
  ownUserId: string | undefined
): Map<string, string> {
  const replies = new Map<string, string>();
  if (!ownUserId) return replies;
  for (const row of rows) {
    if (row.kind !== 'message' || row.senderId !== ownUserId || !row.reply) continue;
    if (typeof row.body === 'string') replies.set(row.reply.eventId, row.body);
  }
  return replies;
}
