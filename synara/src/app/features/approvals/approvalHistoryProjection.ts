import { approvalIdentity, type ApprovalInboxItem } from './nativeApprovalInbox';
import type { SynaraAgentApprovalHistoryItem } from '../../../types/matrix/accountData';

export const historyDecisionLabel = (
  decision: SynaraAgentApprovalHistoryItem['decision'] | undefined,
  status: ApprovalInboxItem['status']
): string => {
  if (status === 'expired') return 'Expired';
  switch (decision) {
    case 'approve_once':
      return 'Approved once';
    case 'approve_always':
      return 'Approved always';
    case 'deny':
      return 'Denied';
    default:
      return status === 'decided' ? 'Decided' : 'Expired';
  }
};

export function historyItemToInboxItem(item: SynaraAgentApprovalHistoryItem): ApprovalInboxItem {
  return {
    roomId: item.roomId,
    eventId: item.eventId,
    sender: item.sender,
    body: item.summary,
    canSendReaction: false,
    bodyTruncated: false,
    originServerTs: item.originServerTs,
    expiresAt: item.expiresAt,
    status: 'decided',
    decision: item.decision,
    decidedAt: item.decidedAt,
    summary: item.summary,
  };
}

/** Recent = account-data history ∪ inbox items whose status ≠ pending. Account data wins. */
export function unionRecentApprovals(
  inboxItems: ApprovalInboxItem[],
  historyItems: SynaraAgentApprovalHistoryItem[]
): ApprovalInboxItem[] {
  const merged = new Map<string, ApprovalInboxItem>();
  for (const item of inboxItems) {
    if (item.status === 'pending') continue;
    merged.set(approvalIdentity(item), item);
  }
  for (const item of historyItems) {
    merged.set(approvalIdentity(item), historyItemToInboxItem(item));
  }
  return [...merged.values()].sort((a, b) => {
    const left = a.decidedAt ?? a.originServerTs;
    const right = b.decidedAt ?? b.originServerTs;
    return right - left;
  });
}
