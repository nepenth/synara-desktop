/**
 * App and inbox badge numbers.
 *
 * Core decides which rooms count and how (a room with mentions adds its
 * mention count, any other room its unread total; space rollups never count)
 * and ships the totals on the room-list presentation. The renderer only adds
 * the counts it owns: active Later items, invites and pending approvals.
 */
export type NotificationSummaryInput = {
  /** `RoomListPresentation.highlightTotal` from Core. */
  highlightTotal?: number;
  /** `RoomListPresentation.unreadTotal` from Core. */
  unreadTotal?: number;
  laterActiveCount?: number;
  inviteCount?: number;
  agentApprovalCount?: number;
};

export type NotificationSummary = {
  appBadgeCount: number;
  inboxBadgeCount: number;
  laterActiveCount: number;
  inviteCount: number;
  agentApprovalCount: number;
  highlightCount: number;
  unreadCount: number;
};

const clampCount = (value: number | null | undefined): number => {
  if (typeof value !== 'number' || !Number.isFinite(value)) return 0;
  return Math.max(0, Math.floor(value));
};

export const summarizeNotifications = ({
  highlightTotal,
  unreadTotal,
  laterActiveCount,
  inviteCount,
  agentApprovalCount,
}: NotificationSummaryInput): NotificationSummary => {
  const laterCount = clampCount(laterActiveCount);
  const invites = clampCount(inviteCount);
  const agentApprovals = clampCount(agentApprovalCount);
  const highlightCount = clampCount(highlightTotal);
  const unreadCount = clampCount(unreadTotal);

  return {
    appBadgeCount: laterCount + highlightCount + unreadCount,
    inboxBadgeCount: laterCount + invites + agentApprovals,
    laterActiveCount: laterCount,
    inviteCount: invites,
    agentApprovalCount: agentApprovals,
    highlightCount,
    unreadCount,
  };
};

/** The app-icon number alone. */
export const getBadgeCount = (input: NotificationSummaryInput): number =>
  summarizeNotifications(input).appBadgeCount;
