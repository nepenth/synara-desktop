/**
 * Desktop client for Core's encrypted `in.synara.draft` sync.
 *
 * The local `in.synara.room_draft.*` copy stays the composer's source of
 * truth; this only mirrors it to the account's other Synara devices. Design:
 * docs/design/cross-device-drafts.md.
 */

import {
  invokeDesktopWithAvailability,
  listen,
  type DesktopInvokeResult,
} from '../../utils/desktop';
import type {
  NativeDraftDecision,
  NativeDraftKeyState,
  NativeDraftResolution,
  NativeDraftSyncStatus,
  NativeDraftWriteResult,
  NativeLocalDraftState,
  NativeRemoteDraft,
} from '../matrix-dto/generated';

export const DRAFTS_UPDATED_EVENT = 'matrix-drafts-updated';
export const MAIN_DRAFT_SLOT = 'main';

export type DraftKeyState = NativeDraftKeyState;
export type DraftDecision = NativeDraftDecision;
export type DraftSyncStatus = NativeDraftSyncStatus;
export type RemoteDraft = NativeRemoteDraft;
export type DraftResolution = NativeDraftResolution;
export type LocalDraftState = NativeLocalDraftState;

export type DraftSaveInput = {
  roomId: string;
  slot: string;
  markdown?: string;
  desktopSlate?: unknown;
  replyTo?: string;
  editOf?: string;
  flush?: boolean;
};

type DraftWriteResult = NativeDraftWriteResult;
type DraftUpdateSignal = { sessionGeneration: number; roomId?: string };

export type DraftSyncInvoke = <T>(
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<T>>;

const defaultInvoke: DraftSyncInvoke = (command, args) =>
  invokeDesktopWithAvailability(command, args);

/** `main`, or `thread:<root>` for a thread pane's composer. */
export const draftSlotFor = (threadRootEventId?: string): string =>
  threadRootEventId ? `thread:${threadRootEventId}` : MAIN_DRAFT_SLOT;

/** A peer's Slate value is used only if it has the editor's basic shape. */
export const isSlateValue = (value: unknown): boolean =>
  Array.isArray(value) &&
  value.length > 0 &&
  value.every(
    (node) =>
      !!node &&
      typeof node === 'object' &&
      typeof (node as { type?: unknown }).type === 'string' &&
      Array.isArray((node as { children?: unknown }).children)
  );

const KEY_STATES: readonly DraftKeyState[] = ['ready', 'requesting', 'needsRecovery', 'disabled'];
const DECISIONS: readonly DraftDecision[] = ['keep', 'apply', 'prompt'];

export const acceptsDraftSyncStatus = (value: unknown): value is DraftSyncStatus => {
  if (!value || typeof value !== 'object') return false;
  const status = value as Partial<DraftSyncStatus>;
  return (
    typeof status.enabled === 'boolean' &&
    typeof status.sessionGeneration === 'number' &&
    KEY_STATES.includes(status.keyState as DraftKeyState)
  );
};

export const acceptsDraftResolution = (value: unknown): value is DraftResolution => {
  if (!value || typeof value !== 'object') return false;
  const resolution = value as Partial<DraftResolution>;
  if (!DECISIONS.includes(resolution.decision as DraftDecision)) return false;
  if (resolution.decision === 'keep') return true;
  const { draft } = resolution;
  return (
    !!draft &&
    typeof draft.updatedAt === 'number' &&
    typeof draft.cleared === 'boolean' &&
    (draft.markdown === undefined || typeof draft.markdown === 'string')
  );
};

export async function draftSyncStatus(
  invoke: DraftSyncInvoke = defaultInvoke
): Promise<DraftSyncStatus | undefined> {
  const result = await invoke<DraftSyncStatus>('matrix_drafts_status').catch(() => undefined);
  return result?.available && acceptsDraftSyncStatus(result.value) ? result.value : undefined;
}

export async function setDraftSyncEnabled(
  enabled: boolean,
  invoke: DraftSyncInvoke = defaultInvoke
): Promise<DraftSyncStatus | undefined> {
  const result = await invoke<DraftSyncStatus>('matrix_drafts_set_enabled', { enabled }).catch(
    () => undefined
  );
  return result?.available && acceptsDraftSyncStatus(result.value) ? result.value : undefined;
}

/** Queue a draft; returns the stamp the write carries, or undefined. */
export async function saveSyncedDraft(
  input: DraftSaveInput,
  invoke: DraftSyncInvoke = defaultInvoke
): Promise<number | undefined> {
  const result = await invoke<DraftWriteResult>('matrix_drafts_save', input).catch(() => undefined);
  return result?.available ? result.value?.updatedAt : undefined;
}

export async function clearSyncedDraft(
  roomId: string,
  slot: string,
  invoke: DraftSyncInvoke = defaultInvoke
): Promise<number | undefined> {
  const result = await invoke<DraftWriteResult>('matrix_drafts_clear', { roomId, slot }).catch(
    () => undefined
  );
  return result?.available ? result.value?.updatedAt : undefined;
}

export async function resolveSyncedDraft(
  roomId: string,
  slot: string,
  local: LocalDraftState,
  fetchFresh: boolean,
  invoke: DraftSyncInvoke = defaultInvoke
): Promise<DraftResolution | undefined> {
  const result = await invoke<DraftResolution>('matrix_drafts_resolve', {
    roomId,
    slot,
    local,
    fetchFresh,
  }).catch(() => undefined);
  return result?.available && acceptsDraftResolution(result.value) ? result.value : undefined;
}

export async function deleteAllSyncedDrafts(
  invoke: DraftSyncInvoke = defaultInvoke
): Promise<number | undefined> {
  const result = await invoke<{ cleared: number }>('matrix_drafts_delete_all').catch(
    () => undefined
  );
  return result?.available ? result.value?.cleared : undefined;
}

export async function flushSyncedDrafts(invoke: DraftSyncInvoke = defaultInvoke): Promise<void> {
  await invoke('matrix_drafts_flush').catch(() => undefined);
}

/** Calls `onUpdate(roomId)` for one room, or `onUpdate(undefined)` for all. */
export async function subscribeSyncedDrafts(
  onUpdate: (roomId: string | undefined) => void
): Promise<() => void> {
  const unlisten = await listen<DraftUpdateSignal>(DRAFTS_UPDATED_EVENT, (event) => {
    const roomId = event.payload?.roomId;
    onUpdate(typeof roomId === 'string' ? roomId : undefined);
  });
  return () => {
    void unlisten?.();
  };
}
