/** A renderer session token prevents late native replies affecting a replacement session. */
export type ApprovalDecision = 'approve_once' | 'approve_always' | 'deny';
export type ApprovalDecisionNotice = {
  scope: object | undefined;
  roomId: string;
  eventId: string;
  /** Optional so the optimistic Recent row can label Denied/Approved before account data syncs. */
  decision?: ApprovalDecision;
};

const APPROVAL_DECISION_BY_ACTION_ID: Record<string, ApprovalDecision> = {
  'agent-approval.approve-once': 'approve_once',
  'agent-approval.approve-always': 'approve_always',
  'agent-approval.deny': 'deny',
};
export const approvalDecisionFromActionId = (actionId: string): ApprovalDecision | undefined =>
  APPROVAL_DECISION_BY_ACTION_ID[actionId];
let activeScope: object | undefined;
const listeners = new Set<(notice: ApprovalDecisionNotice) => void>();
export const captureApprovalDecisionScope = () => activeScope;
export function activateApprovalDecisionScope(scope: object) {
  activeScope = scope;
  return () => {
    if (activeScope === scope) activeScope = undefined;
  };
}
export function subscribeApprovalDecisions(listener: (notice: ApprovalDecisionNotice) => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}
export function publishApprovalDecision(notice: ApprovalDecisionNotice) {
  listeners.forEach((listener) => listener(notice));
}
