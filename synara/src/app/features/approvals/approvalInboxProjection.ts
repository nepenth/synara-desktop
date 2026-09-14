import {
  approvalIdentity,
  approvalStatus,
  type ApprovalInboxSnapshot,
} from './nativeApprovalInbox';
import type { ApprovalDecision } from './approvalDecisionEvents';

/** Readback + optimistic decisions for exactly one client/generation, independently testable. */
export function createApprovalInboxProjection() {
  let client: object | undefined;
  let scope: object = {};
  let snapshot: ApprovalInboxSnapshot | undefined;
  const completed = new Map<string, ApprovalDecision | undefined>();
  return {
    get scope() {
      return scope;
    },
    reset() {
      client = undefined;
      snapshot = undefined;
      scope = {};
      completed.clear();
    },
    receive(next: ApprovalInboxSnapshot, nextClient: object) {
      if (nextClient !== client || next.sessionGeneration !== snapshot?.sessionGeneration) {
        scope = {};
        completed.clear();
      }
      client = nextClient;
      snapshot = next;
    },
    complete(
      expectedScope: object | undefined,
      item: { roomId: string; eventId: string; decision?: ApprovalDecision }
    ): boolean {
      if (!snapshot || expectedScope !== scope) return false;
      completed.set(approvalIdentity(item), item.decision);
      return true;
    },
    read(now: number): ApprovalInboxSnapshot | undefined {
      return (
        snapshot && {
          ...snapshot,
          items: snapshot.items.map((item) => {
            const identity = approvalIdentity(item);
            if (!completed.has(identity)) return { ...item, status: approvalStatus(item, now) };
            const decision = completed.get(identity) ?? item.decision;
            return {
              ...item,
              status: 'decided' as const,
              ...(decision ? { decision, decidedAt: item.decidedAt ?? now } : {}),
            };
          }),
        }
      );
    },
  };
}
