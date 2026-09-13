/** A renderer session token prevents late native replies affecting a replacement session. */
export type ApprovalDecisionNotice = {
  scope: object | undefined;
  roomId: string;
  eventId: string;
};
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
