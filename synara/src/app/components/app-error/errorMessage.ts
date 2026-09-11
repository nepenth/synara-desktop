export const formatUnknownError = (error: unknown): string => {
  if (typeof error === 'string' && error.trim()) return error;
  if (error instanceof Error && error.message.trim()) return error.message;
  if (error && typeof error === 'object') {
    const record = error as { message?: unknown; statusText?: unknown; status?: unknown };
    if (typeof record.message === 'string' && record.message.trim()) return record.message;
    if (typeof record.statusText === 'string' && record.statusText.trim()) {
      return typeof record.status === 'number'
        ? `${record.status} ${record.statusText}`
        : record.statusText;
    }
  }
  return 'An unexpected error occurred.';
};
