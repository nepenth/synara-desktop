export const NATIVE_TIMELINE_GROUP_WINDOW_MS = 2 * 60 * 60 * 1000;

export type NativeTimelineGroupingRow = {
  senderId?: string;
  originServerTs?: number;
};

/**
 * Consecutive messages from one sender share presentation metadata for up to
 * two hours. Non-message rows are represented without sender metadata, so a
 * date divider or system event always terminates the visual group.
 */
export const shouldGroupNativeTimelineRows = (
  previous: NativeTimelineGroupingRow | undefined,
  current: NativeTimelineGroupingRow | undefined
): boolean => {
  if (!previous || !current) return false;
  if (!previous.senderId || previous.senderId !== current.senderId) return false;
  if (!previous.originServerTs || !current.originServerTs) return false;

  const elapsed = current.originServerTs - previous.originServerTs;
  return elapsed >= 0 && elapsed < NATIVE_TIMELINE_GROUP_WINDOW_MS;
};
