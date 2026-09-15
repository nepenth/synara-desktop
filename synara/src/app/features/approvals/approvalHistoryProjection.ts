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

export const EMPTY_HISTORY_SUMMARY = 'Command not recorded';

export function historyItemToInboxItem(item: SynaraAgentApprovalHistoryItem): ApprovalInboxItem {
  const summary = item.summary.trim() ? item.summary : EMPTY_HISTORY_SUMMARY;
  return {
    roomId: item.roomId,
    eventId: item.eventId,
    sender: item.sender,
    body: summary,
    canSendReaction: false,
    bodyTruncated: false,
    originServerTs: item.originServerTs,
    expiresAt: item.expiresAt,
    status: 'decided',
    decision: item.decision,
    decidedAt: item.decidedAt,
    summary,
  };
}

export function compareRecentApprovals(a: ApprovalInboxItem, b: ApprovalInboxItem): number {
  const left = a.decidedAt ?? a.originServerTs;
  const right = b.decidedAt ?? b.originServerTs;
  if (right !== left) return right - left;
  if (a.roomId !== b.roomId) return a.roomId < b.roomId ? -1 : 1;
  if (a.eventId !== b.eventId) return a.eventId < b.eventId ? -1 : 1;
  return 0;
}

function historyPreview(item: ApprovalInboxItem): string | undefined {
  const summary = item.summary?.trim();
  if (summary && summary !== EMPTY_HISTORY_SUMMARY) return item.summary;
  return undefined;
}

/** Inbox proof wins a conflict. Account data is not a reaction. */
function mergeInboxOverHistory(
  inbox: ApprovalInboxItem,
  history: ApprovalInboxItem
): ApprovalInboxItem {
  const inboxNamedDecision = inbox.decision !== undefined;
  const hideUnprovenHistoryDecision = inbox.status === 'decided' && !inboxNamedDecision;
  const decision = inboxNamedDecision
    ? inbox.decision
    : hideUnprovenHistoryDecision
    ? undefined
    : history.decision;
  const status = decision !== undefined || hideUnprovenHistoryDecision ? 'decided' : inbox.status;
  const summary = historyPreview(history) ?? inbox.summary ?? inbox.body;
  return {
    ...history,
    ...inbox,
    status,
    decision,
    summary,
    body: summary,
    decidedAt: inbox.decidedAt ?? history.decidedAt,
  };
}

/**
 * Recent = account-data history ∪ inbox items whose status ≠ pending.
 * History-only rows stay visible (cross-device). Inbox `decision` wins a
 * conflict. A decided inbox row without `decision` does not inherit the
 * history decision. An expired inbox row may still show the history decision.
 */
export function unionRecentApprovals(
  inboxItems: ApprovalInboxItem[],
  historyItems: SynaraAgentApprovalHistoryItem[]
): ApprovalInboxItem[] {
  const merged = new Map<string, ApprovalInboxItem>();
  for (const item of historyItems) {
    merged.set(approvalIdentity(item), historyItemToInboxItem(item));
  }
  for (const item of inboxItems) {
    if (item.status === 'pending') continue;
    const identity = approvalIdentity(item);
    const history = merged.get(identity);
    merged.set(identity, history ? mergeInboxOverHistory(item, history) : item);
  }
  return [...merged.values()].sort(compareRecentApprovals);
}
