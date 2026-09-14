import {
  invokeDesktopWithAvailability,
  listen,
  type DesktopInvokeResult,
} from '../../utils/desktop';
import type { SynaraAgentApprovalHistoryItem } from '../../../types/matrix/accountData';

export const AGENT_APPROVAL_HISTORY_UPDATED_EVENT = 'matrix-agent-approval-history-updated';

export type NativeAgentApprovalHistorySnapshot = {
  items: SynaraAgentApprovalHistoryItem[];
};

export type NativeAgentApprovalHistoryInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<NativeAgentApprovalHistorySnapshot>>;

const defaultInvoke: NativeAgentApprovalHistoryInvoke = (command, args) =>
  invokeDesktopWithAvailability<NativeAgentApprovalHistorySnapshot>(command, args);

export function acceptsAgentApprovalHistory(
  value: unknown
): value is NativeAgentApprovalHistorySnapshot {
  if (!value || typeof value !== 'object') return false;
  const snapshot = value as NativeAgentApprovalHistorySnapshot;
  if (!Array.isArray(snapshot.items)) return false;
  const identities = new Set<string>();
  return snapshot.items.every((item) => {
    if (
      !item ||
      typeof item !== 'object' ||
      ![item.roomId, item.eventId, item.sender].every(
        (id) => typeof id === 'string' && id.length > 0
      ) ||
      !['approve_once', 'approve_always', 'deny'].includes(item.decision) ||
      typeof item.summary !== 'string' ||
      !Number.isFinite(item.decidedAt) ||
      !Number.isFinite(item.originServerTs) ||
      item.originServerTs <= 0 ||
      !Number.isFinite(item.expiresAt) ||
      item.expiresAt <= item.originServerTs
    ) {
      return false;
    }
    const identity = `${item.roomId}\u0000${item.eventId}`;
    if (identities.has(identity)) return false;
    identities.add(identity);
    return true;
  });
}

export async function loadAgentApprovalHistory(
  invoke: NativeAgentApprovalHistoryInvoke = defaultInvoke
): Promise<NativeAgentApprovalHistorySnapshot> {
  const result = await invoke('matrix_agent_approval_history_snapshot');
  if (!result.available || !acceptsAgentApprovalHistory(result.value)) {
    throw new Error('Approval history could not be loaded.');
  }
  return result.value;
}

export async function subscribeAgentApprovalHistory(onUpdate: () => void): Promise<() => void> {
  const unlisten = await listen(AGENT_APPROVAL_HISTORY_UPDATED_EVENT, () => {
    onUpdate();
  });
  return () => {
    void unlisten?.();
  };
}
