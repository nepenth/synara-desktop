/** One item a bulk action could not complete, with the error it raised. */
export type BulkActionFailure<T> = {
  item: T;
  error: unknown;
};

/**
 * Run `action` for every item in order. A failure does not stop the rest;
 * every failure is returned so the caller can say which items need a retry.
 */
export const runBulkAction = async <T>(
  items: readonly T[],
  action: (item: T, index: number) => Promise<unknown>
): Promise<BulkActionFailure<T>[]> => {
  const failures: BulkActionFailure<T>[] = [];
  for (let index = 0; index < items.length; index += 1) {
    const item = items[index];
    try {
      // Sequential on purpose: Core serializes these writes and the server
      // rate-limits bursts.
      // eslint-disable-next-line no-await-in-loop
      await action(item, index);
    } catch (error) {
      failures.push({ item, error });
    }
  }
  return failures;
};

const MAX_LISTED_FAILURES = 3;

/** "A", "A and B", "A, B and C", "A, B, C and 2 more". */
export const listBulkFailureLabels = (labels: readonly string[]): string => {
  if (labels.length <= 1) return labels[0] ?? '';
  if (labels.length <= MAX_LISTED_FAILURES) {
    return `${labels.slice(0, -1).join(', ')} and ${labels[labels.length - 1]}`;
  }
  const rest = labels.length - MAX_LISTED_FAILURES;
  return `${labels.slice(0, MAX_LISTED_FAILURES).join(', ')} and ${rest} more`;
};

export type BulkFailureCopy = {
  /** Past tense for the success count, e.g. "Declined". */
  done: string;
  /** Base verb for the failure list, e.g. "decline". */
  verb: string;
  /** Plural noun, e.g. "invites". */
  noun: string;
};

/**
 * Copy for a bulk action where some items failed, e.g.
 * "Declined 2 of 3 invites. Couldn't decline Spam Room. Try again."
 */
export const bulkFailureMessage = (
  total: number,
  failedLabels: readonly string[],
  { done, verb, noun }: BulkFailureCopy
): string => {
  const succeeded = Math.max(0, total - failedLabels.length);
  const failedList = listBulkFailureLabels(failedLabels);
  if (succeeded === 0) {
    return `Couldn't ${verb} ${failedList}. Try again.`;
  }
  return `${done} ${succeeded} of ${total} ${noun}. Couldn't ${verb} ${failedList}. Try again.`;
};

/** Thrown when a bulk action finished with failures; the message is user copy. */
export class BulkActionError<T = unknown> extends Error {
  readonly failures: BulkActionFailure<T>[];

  constructor(message: string, failures: BulkActionFailure<T>[]) {
    super(message);
    this.name = 'BulkActionError';
    this.failures = failures;
  }
}
