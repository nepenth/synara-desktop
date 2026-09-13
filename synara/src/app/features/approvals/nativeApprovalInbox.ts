import { invokeDesktopWithAvailability, type DesktopInvokeResult } from '../../utils/desktop';

export type ApprovalInboxItem = {
  roomId: string;
  eventId: string;
  sender: string;
  body: string;
  originServerTs: number;
  expiresAt: number;
  status: 'pending' | 'decided' | 'expired';
};

export type ApprovalInboxSnapshot = {
  sessionGeneration: number;
  items: ApprovalInboxItem[];
  loading: boolean;
  incomplete: boolean;
};

export const approvalIdentity = (item: Pick<ApprovalInboxItem, 'roomId' | 'eventId'>): string =>
  `${item.roomId}\u0000${item.eventId}`;

export const approvalStatus = (item: ApprovalInboxItem, now: number): ApprovalInboxItem['status'] =>
  item.status === 'pending' && now >= item.expiresAt ? 'expired' : item.status;

export function acceptsApprovalInbox(value: unknown): value is ApprovalInboxSnapshot {
  if (!value || typeof value !== 'object') return false;
  const snapshot = value as ApprovalInboxSnapshot;
  if (
    !Number.isSafeInteger(snapshot.sessionGeneration) ||
    snapshot.sessionGeneration < 0 ||
    typeof snapshot.loading !== 'boolean' ||
    typeof snapshot.incomplete !== 'boolean' ||
    !Array.isArray(snapshot.items)
  )
    return false;
  const identities = new Set<string>();
  return snapshot.items.every((item) => {
    if (
      !item ||
      typeof item !== 'object' ||
      ![item.roomId, item.eventId, item.sender].every(
        (id) => typeof id === 'string' && id.length > 0
      ) ||
      typeof item.body !== 'string' ||
      !Number.isSafeInteger(item.originServerTs) ||
      item.originServerTs <= 0 ||
      !Number.isSafeInteger(item.expiresAt) ||
      item.expiresAt <= item.originServerTs ||
      !['pending', 'decided', 'expired'].includes(item.status)
    )
      return false;
    const identity = approvalIdentity(item);
    if (identities.has(identity)) return false;
    identities.add(identity);
    return true;
  });
}

export async function loadApprovalInbox(
  invoke: (command: string) => Promise<DesktopInvokeResult<unknown>> = invokeDesktopWithAvailability
): Promise<ApprovalInboxSnapshot> {
  const result = await invoke('matrix_agent_approvals_list');
  if (!result.available || !acceptsApprovalInbox(result.value)) {
    throw new Error('Approval requests could not be loaded.');
  }
  return result.value;
}
