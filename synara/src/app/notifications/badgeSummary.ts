export type BadgeUnreadSource = {
  total?: number;
  highlight?: number;
};

export type NotificationSummaryInput = {
  unreadCounts: Iterable<BadgeUnreadSource>;
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

export type BadgeSummary = {
  count: number;
  laterActiveCount: number;
  highlightCount: number;
  unreadCount: number;
};

const clampCount = (value: number | undefined): number => {
  if (!Number.isFinite(value)) return 0;
  return Math.max(0, Math.floor(value ?? 0));
};

export const summarizeNotifications = ({
  unreadCounts,
  laterActiveCount,
  inviteCount,
  agentApprovalCount,
}: NotificationSummaryInput): NotificationSummary => {
  const laterCount = clampCount(laterActiveCount);
  const invites = clampCount(inviteCount);
  const agentApprovals = clampCount(agentApprovalCount);
  let highlightCount = 0;
  let unreadCount = 0;

  Array.from(unreadCounts).forEach((unread) => {
    if (unread.highlight !== undefined) {
      highlightCount += clampCount(unread.highlight);
      return;
    }
    unreadCount += clampCount(unread.total);
  });

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

export const summarizeBadgeCount = (
  unreadCounts: Iterable<BadgeUnreadSource>,
  laterActiveCount: number
): BadgeSummary => {
  const summary = summarizeNotifications({ unreadCounts, laterActiveCount });

  return {
    count: summary.appBadgeCount,
    laterActiveCount: summary.laterActiveCount,
    highlightCount: summary.highlightCount,
    unreadCount: summary.unreadCount,
  };
};

export const getBadgeCount = (
  unreadCounts: Iterable<BadgeUnreadSource>,
  laterActiveCount: number
): number => summarizeBadgeCount(unreadCounts, laterActiveCount).count;
