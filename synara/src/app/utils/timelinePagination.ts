export type TimelinePaginationDirection = 'backward' | 'forward';

export type TimelinePaginationErrors = Partial<Record<TimelinePaginationDirection, string>>;

export const createTimelinePaginationErrorMessage = (err: unknown): string => {
  if (err instanceof Error && err.message.trim().length > 0) {
    return err.message;
  }
  return 'Failed to load messages.';
};

export const setTimelinePaginationError = (
  errors: TimelinePaginationErrors,
  direction: TimelinePaginationDirection,
  err: unknown
): TimelinePaginationErrors => ({
  ...errors,
  [direction]: createTimelinePaginationErrorMessage(err),
});

export const clearTimelinePaginationError = (
  errors: TimelinePaginationErrors,
  direction: TimelinePaginationDirection
): TimelinePaginationErrors => {
  if (!errors[direction]) return errors;
  const next = { ...errors };
  delete next[direction];
  return next;
};

export const shouldShowTimelinePaginationLoader = (
  canPaginate: boolean,
  errors: TimelinePaginationErrors,
  direction: TimelinePaginationDirection
): boolean => canPaginate && !errors[direction];
