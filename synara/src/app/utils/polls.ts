export const POLL_START_EVENT_TYPE = 'org.matrix.msc3381.poll.start';
export const POLL_RESPONSE_EVENT_TYPE = 'org.matrix.msc3381.poll.response';
export const POLL_START_KEY = 'm.poll.start';
export const POLL_START_UNSTABLE_KEY = 'org.matrix.msc3381.poll.start';
export const POLL_RESPONSE_KEY = 'm.poll.response';
export const POLL_RESPONSE_UNSTABLE_KEY = 'org.matrix.msc3381.poll.response';
export const POLL_DISCLOSED_KIND = 'm.poll.disclosed';
export const POLL_DISCLOSED_UNSTABLE_KIND = 'org.matrix.msc3381.poll.disclosed';
export const M_TEXT_KEY = 'm.text';

const MAX_POLL_ANSWERS = 20;
const MAX_POLL_TEXT_LENGTH = 240;
export const MAX_POLL_SELECTIONS = 10;
export const DEFAULT_POLL_SELECTIONS = 1;

export type PollAnswer = {
  id: string;
  text: string;
};

export type ParsedPoll = {
  question: string;
  answers: PollAnswer[];
  maxSelections: number;
};

const trimPollText = (value: string): string => value.trim().slice(0, MAX_POLL_TEXT_LENGTH);

const pollAnswerId = (index: number, answer: string): string => {
  let hash = 0;
  for (let i = 0; i < answer.length; i += 1) {
    hash = (hash * 31 + answer.charCodeAt(i)) % 2147483647;
  }
  return `a${index + 1}_${hash.toString(36)}`;
};

const parseMaxSelectionText = (value: string): number | undefined => {
  const match = value.match(/^max(?:_?selections)?=(\d+)$/i);
  if (!match) return undefined;
  const parsed = Number.parseInt(match[1], 10);
  if (!Number.isFinite(parsed)) return undefined;
  return parsed;
};

export const normalizePollParts = (
  question: string,
  answers: string[],
  maxSelections = DEFAULT_POLL_SELECTIONS
): ParsedPoll | undefined => {
  const cleanQuestion = trimPollText(question);
  const cleanAnswers = answers.map(trimPollText).filter(Boolean).slice(0, MAX_POLL_ANSWERS);
  const normalizedMaxSelections = Math.max(
    DEFAULT_POLL_SELECTIONS,
    Math.min(maxSelections, cleanAnswers.length, MAX_POLL_SELECTIONS)
  );

  if (!cleanQuestion || cleanAnswers.length < 2) return undefined;

  return {
    question: cleanQuestion,
    maxSelections: normalizedMaxSelections,
    answers: cleanAnswers.map((answer, index) => ({
      id: pollAnswerId(index, answer),
      text: answer,
    })),
  };
};

export const parsePollCommand = (payload: string): ParsedPoll | undefined => {
  const [question = '', ...answers] = payload.split('|');
  const normalizedAnswers = answers.map(trimPollText).filter(Boolean);
  let maxSelections = DEFAULT_POLL_SELECTIONS;

  const filteredAnswers: string[] = [];
  normalizedAnswers.forEach((answer) => {
    const maybeMax = parseMaxSelectionText(answer);
    if (typeof maybeMax === 'number') {
      maxSelections = maybeMax;
      return;
    }
    filteredAnswers.push(answer);
  });

  return normalizePollParts(question, filteredAnswers, maxSelections);
};

export const makePollStartContent = (
  question: string,
  answers: string[],
  maxSelections = DEFAULT_POLL_SELECTIONS
) => {
  const poll = normalizePollParts(question, answers, maxSelections);
  if (!poll) return undefined;

  const subtype = {
    question: { [M_TEXT_KEY]: poll.question },
    kind: POLL_DISCLOSED_KIND,
    max_selections: poll.maxSelections,
    answers: poll.answers.map((answer) => ({
      id: answer.id,
      [M_TEXT_KEY]: answer.text,
    })),
  };

  return {
    body: `${poll.question}\n${poll.answers
      .map((answer, index) => `${index + 1}. ${answer.text}`)
      .join('\n')}`,
    [M_TEXT_KEY]: poll.question,
    [POLL_START_KEY]: subtype,
    [POLL_START_UNSTABLE_KEY]: {
      ...subtype,
      kind: POLL_DISCLOSED_UNSTABLE_KIND,
    },
  };
};

export const makePollStartContentFromCommand = (payload: string) => {
  const poll = parsePollCommand(payload);
  if (!poll) return undefined;
  return makePollStartContent(
    poll.question,
    poll.answers.map((answer) => answer.text),
    poll.maxSelections
  );
};

type PollStartPayload = {
  question?: Record<string, unknown>;
  answers?: Array<Record<string, unknown>>;
  max_selections?: unknown;
  maxSelections?: unknown;
};

export const parsePollStartContent = (content: Record<string, unknown>): ParsedPoll | undefined => {
  const poll = (content[POLL_START_KEY] ?? content[POLL_START_UNSTABLE_KEY]) as
    | PollStartPayload
    | undefined;

  if (!poll) return undefined;
  const question = poll.question?.[M_TEXT_KEY];
  if (typeof question !== 'string' || !Array.isArray(poll.answers)) return undefined;
  const stableMaxSelectionsRaw = Number.isFinite(Number(poll.max_selections))
    ? Number(poll.max_selections)
    : Number((poll as { maxSelections?: unknown }).maxSelections);
  const maxSelections = Number.isFinite(stableMaxSelectionsRaw)
    ? stableMaxSelectionsRaw
    : DEFAULT_POLL_SELECTIONS;
  const normalizedMaxSelections = Math.max(
    DEFAULT_POLL_SELECTIONS,
    Math.min(Math.floor(maxSelections), poll.answers.length, MAX_POLL_SELECTIONS)
  );

  const answers = poll.answers
    .map((answer): PollAnswer | undefined => {
      const { id } = answer;
      const text = answer[M_TEXT_KEY];
      if (typeof id !== 'string' || typeof text !== 'string') return undefined;
      return { id, text };
    })
    .filter((answer): answer is PollAnswer => !!answer);

  if (answers.length < 2) return undefined;
  return {
    question,
    maxSelections: normalizedMaxSelections,
    answers,
  };
};

export const makePollResponseContent = (pollEventId: string, answerIds: string[]) => {
  const safeAnswers = Array.from(
    new Set(answerIds.filter((answerId) => typeof answerId === 'string' && answerId.trim()))
  );

  return {
    'm.relates_to': {
      rel_type: 'm.reference',
      event_id: pollEventId,
    },
    [POLL_RESPONSE_KEY]: {
      answers: safeAnswers,
    },
    [POLL_RESPONSE_UNSTABLE_KEY]: {
      answers: safeAnswers,
    },
  };
};

export const parsePollResponseContent = (
  content: Record<string, unknown>
): string[] | undefined => {
  const response = (content[POLL_RESPONSE_KEY] ?? content[POLL_RESPONSE_UNSTABLE_KEY]) as
    | { answers?: unknown }
    | undefined;
  if (!response) return undefined;
  if (!Array.isArray(response?.answers)) return undefined;
  const answers = response.answers.filter((answer): answer is string => typeof answer === 'string');
  return answers.length > 0 ? answers : undefined;
};

export const summarizePollResponses = (
  responses: Array<{ sender?: string; ts?: number; answers: string[] }>,
  answerIds: string[]
): Record<string, number> => {
  const latestBySender = new Map<string, { ts: number; answers: string[] }>();
  responses.forEach((response, index) => {
    const sender = response.sender ?? `event-${index}`;
    const ts = response.ts ?? index;
    const current = latestBySender.get(sender);
    if (!current || current.ts <= ts) {
      latestBySender.set(sender, { ts, answers: response.answers });
    }
  });

  const counts = Object.fromEntries(answerIds.map((answerId) => [answerId, 0]));
  latestBySender.forEach((response) => {
    const unique = new Set(response.answers.filter((answerId) => answerIds.includes(answerId)));
    unique.forEach((answerId) => {
      counts[answerId] += 1;
    });
  });
  return counts;
};
