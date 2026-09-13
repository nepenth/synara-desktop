import {
  approvalIdentity,
  approvalStatus,
  type ApprovalInboxSnapshot,
} from './nativeApprovalInbox';

/** Readback + optimistic decisions for exactly one client/generation, independently testable. */
export function createApprovalInboxProjection() {
  let client: object | undefined;
  let scope: object = {};
  let snapshot: ApprovalInboxSnapshot | undefined;
  const completed = new Set<string>();
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
      item: { roomId: string; eventId: string }
    ): boolean {
      if (!snapshot || expectedScope !== scope) return false;
      completed.add(approvalIdentity(item));
      return true;
    },
    read(now: number): ApprovalInboxSnapshot | undefined {
      return (
        snapshot && {
          ...snapshot,
          items: snapshot.items.map((item) => ({
            ...item,
            status: completed.has(approvalIdentity(item)) ? 'decided' : approvalStatus(item, now),
          })),
        }
      );
    },
  };
}
