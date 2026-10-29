export const NATIVE_TIMELINE_GROUP_WINDOW_MS = 2 * 60 * 60 * 1000;

export type NativeTimelineGroupingRow = {
  /** Row kind from the view snapshot; only sender-attributed kinds group. */
  kind?: string;
  senderId?: string;
  originServerTs?: number;
};

/**
 * Row kinds that show who sent them. Membership, state, call, poll and
 * "other" rows also carry the sender's id, but render no sender header, so a
 * message after one of them must start a new group with its own header.
 */
export const nativeTimelineRowShowsSender = (kind: string | undefined): boolean =>
  kind === 'message' || kind === 'sticker';

/**
 * Consecutive messages from one sender share presentation metadata for up to
 * two hours. Any row that shows no sender (a date divider, a system event, a
 * hidden or unsupported event) terminates the visual group.
 */
export const shouldGroupNativeTimelineRows = (
  previous: NativeTimelineGroupingRow | undefined,
  current: NativeTimelineGroupingRow | undefined
): boolean => {
  if (!previous || !current) return false;
  if (!nativeTimelineRowShowsSender(previous.kind) || !nativeTimelineRowShowsSender(current.kind)) {
    return false;
  }
  if (!previous.senderId || previous.senderId !== current.senderId) return false;
  if (!previous.originServerTs || !current.originServerTs) return false;

  const elapsed = current.originServerTs - previous.originServerTs;
  return elapsed >= 0 && elapsed < NATIVE_TIMELINE_GROUP_WINDOW_MS;
};
