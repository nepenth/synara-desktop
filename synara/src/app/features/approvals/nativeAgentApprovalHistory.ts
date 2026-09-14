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

const MAX_HISTORY_ITEMS = 200;
const MAX_SUMMARY_CHARS = 240;
const MAX_ID_BYTES = 255;
const MAX_SENDER_CHARS = 256;

const hasDisallowedIdChar = (value: string): boolean =>
  [...value].some((ch) => {
    const code = ch.codePointAt(0) ?? 0;
    return (
      ch.trim() === '' ||
      code <= 0x1f ||
      (code >= 0x7f && code <= 0x9f) ||
      (code >= 0x200b && code <= 0x200f) ||
      (code >= 0x202a && code <= 0x202e) ||
      (code >= 0x2066 && code <= 0x2069) ||
      code === 0xfeff
    );
  });

const isMatrixRoomId = (value: string): boolean => {
  if (!value.startsWith('!') || value.length <= 1 || value.length > MAX_ID_BYTES) return false;
  if (hasDisallowedIdChar(value)) return false;
  const separator = value.indexOf(':');
  return separator > 1 && separator < value.length - 1;
};

const isMatrixEventId = (value: string): boolean =>
  value.startsWith('$') &&
  value.length > 1 &&
  value.length <= MAX_ID_BYTES &&
  !hasDisallowedIdChar(value);

const isMatrixUserId = (value: string): boolean => {
  if ([...value].length > MAX_SENDER_CHARS || hasDisallowedIdChar(value)) return false;
  const separator = value.indexOf(':');
  return value.startsWith('@') && separator > 1 && separator < value.length - 1;
};

const isSafeSummary = (value: string): boolean => {
  if ([...value].length > MAX_SUMMARY_CHARS) return false;
  return ![...value].some((ch) => {
    const code = ch.codePointAt(0) ?? 0;
    return (
      code <= 0x1f ||
      (code >= 0x7f && code <= 0x9f) ||
      (code >= 0x200b && code <= 0x200f) ||
      (code >= 0x202a && code <= 0x202e) ||
      (code >= 0x2066 && code <= 0x2069) ||
      code === 0xfeff
    );
  });
};

export function acceptsAgentApprovalHistory(
  value: unknown
): value is NativeAgentApprovalHistorySnapshot {
  if (!value || typeof value !== 'object') return false;
  const snapshot = value as NativeAgentApprovalHistorySnapshot;
  if (!Array.isArray(snapshot.items) || snapshot.items.length > MAX_HISTORY_ITEMS) return false;
  const identities = new Set<string>();
  return snapshot.items.every((item) => {
    if (
      !item ||
      typeof item !== 'object' ||
      !isMatrixRoomId(item.roomId) ||
      !isMatrixEventId(item.eventId) ||
      !isMatrixUserId(item.sender) ||
      !['approve_once', 'approve_always', 'deny'].includes(item.decision) ||
      typeof item.summary !== 'string' ||
      !isSafeSummary(item.summary) ||
      !Number.isFinite(item.decidedAt) ||
      item.decidedAt <= 0 ||
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
