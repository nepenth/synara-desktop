/**
 * SDK-neutral owner for native composer reply-draft state.
 *
 * Message body drafts remain local (Slate / localStorage). Reply transport
 * remains `matrix_send_text` with `replyTo`. Used with NativeTimelinePresenter
 * after V-TIMELINE.C1/C2 cutover (JS RoomTimeline deleted).
 */

import type { DesktopInvokeResult } from '../../utils/desktop';
export const NATIVE_COMPOSER_REPLY_DRAFT_SCHEMA_VERSION = 2;

export type NativeComposerReplyDraft = {
  /** Core-issued opaque identity; pass back unchanged when clearing. */
  draftRevision: number;
  eventId: string;
  senderId: string;
  body: string;
  formattedBody?: string;
  threadRootEventId?: string;
};

export type NativeComposerReplyDraftStatus = 'set' | 'cleared' | 'empty';

export type NativeComposerReplyDraftReadback = {
  schemaVersion: number;
  roomId: string;
  status: NativeComposerReplyDraftStatus;
  draft?: NativeComposerReplyDraft;
};

export type NativeComposerSendRelation = {
  draftRevision?: number;
  replyTo?: string;
  threadRoot?: string;
};

export type NativeComposerSetReplyDraftInput = {
  roomId: string;
  eventId: string;
  startThread?: boolean;
};

export type NativeComposerReplyDraftRoomInput = {
  roomId: string;
  /** Live drafts omit this; thread-view drafts key the same room separately. */
  threadRootEventId?: string;
};

export type NativeComposerClearReplyDraftInput = NativeComposerReplyDraftRoomInput & {
  /** Clear only the exact Core draft consumed by the actor. */
  expectedDraftRevision: number;
};

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

const DRAFT_STATUSES = new Set<NativeComposerReplyDraftStatus>(['set', 'cleared', 'empty']);

const acceptReplyDraftReadback = (
  value: unknown,
  expectedRoomId: string
): NativeComposerReplyDraftReadback | undefined => {
  if (!value || typeof value !== 'object') return undefined;
  const readback = value as NativeComposerReplyDraftReadback;
  if (
    readback.schemaVersion !== NATIVE_COMPOSER_REPLY_DRAFT_SCHEMA_VERSION ||
    typeof readback.roomId !== 'string' ||
    readback.roomId !== expectedRoomId ||
    !DRAFT_STATUSES.has(readback.status)
  ) {
    return undefined;
  }
  if (readback.draft) {
    const draft = readback.draft;
    if (
      !Number.isSafeInteger(draft.draftRevision) ||
      draft.draftRevision <= 0 ||
      typeof draft.eventId !== 'string' ||
      typeof draft.senderId !== 'string' ||
      typeof draft.body !== 'string'
    ) {
      return undefined;
    }
    if (readback.status !== 'set') return undefined;
  } else if (readback.status === 'set') {
    return undefined;
  }
  return readback;
};

/**
 * Derive every composer transport relation from the same Core-owned draft.
 *
 * A threaded reply carries both the thread root and the selected event. Core
 * needs both identifiers to emit `m.thread` plus `m.in_reply_to`; dropping the
 * selected event silently degrades a reply into a plain thread message.
 */
export const nativeComposerSendRelation = (
  draft: NativeComposerReplyDraft | undefined,
  viewThreadRoot?: string
): NativeComposerSendRelation => ({
  draftRevision: draft?.draftRevision,
  replyTo: draft?.eventId,
  threadRoot: draft?.threadRootEventId ?? viewThreadRoot,
});

export const nativeComposerDraftSlotKey = (
  roomId: string,
  threadRootEventId?: string
): string => (threadRootEventId ? `${roomId}\0${threadRootEventId}` : roomId);

type ReplyDraftListener = () => void;

/**
 * Readback-only UI projection of the authoritative Core composer registry.
 * It cannot invent or mutate a draft: callers may only apply a typed Core
 * readback produced by set/get/clear.
 */
export class NativeComposerReplyDraftProjection {
  private readonly drafts = new Map<string, NativeComposerReplyDraft>();

  private readonly listeners = new Map<string, Set<ReplyDraftListener>>();

  get(roomId: string, threadRootEventId?: string): NativeComposerReplyDraft | undefined {
    return this.drafts.get(nativeComposerDraftSlotKey(roomId, threadRootEventId));
  }

  apply(readback: NativeComposerReplyDraftReadback, slotThreadRoot?: string): void {
    const threadRoot = readback.draft?.threadRootEventId ?? slotThreadRoot;
    const key = nativeComposerDraftSlotKey(readback.roomId, threadRoot);
    if (readback.status === 'set' && readback.draft) {
      this.drafts.set(key, readback.draft);
    } else {
      this.drafts.delete(key);
    }
    this.listeners.get(key)?.forEach((listener) => listener());
  }

  clearLocal(roomId: string, threadRootEventId?: string): void {
    const key = nativeComposerDraftSlotKey(roomId, threadRootEventId);
    if (!this.drafts.delete(key)) return;
    this.listeners.get(key)?.forEach((listener) => listener());
  }

  subscribe(
    roomId: string,
    listener: ReplyDraftListener,
    threadRootEventId?: string
  ): () => void {
    const key = nativeComposerDraftSlotKey(roomId, threadRootEventId);
    const roomListeners = this.listeners.get(key) ?? new Set<ReplyDraftListener>();
    roomListeners.add(listener);
    this.listeners.set(key, roomListeners);
    return () => {
      roomListeners.delete(listener);
      if (roomListeners.size === 0) this.listeners.delete(key);
    };
  }
}

export async function setReplyDraftWithNativeComposerOwner(
  input: NativeComposerSetReplyDraftInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeComposerReplyDraftReadback | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_composer_set_reply_draft', { request: input });
  if (!result.available) return 'unavailable';
  const readback = acceptReplyDraftReadback(result.value, input.roomId);
  if (readback?.status !== 'set' || readback.draft?.eventId !== input.eventId) {
    return 'unavailable';
  }
  return readback ?? 'unavailable';
}

export async function clearReplyDraftWithNativeComposerOwner(
  input: NativeComposerClearReplyDraftInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeComposerReplyDraftReadback | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_composer_clear_reply_draft', { request: input });
  if (!result.available) return 'unavailable';
  const readback = acceptReplyDraftReadback(result.value, input.roomId);
  if (readback?.status === 'cleared') return readback;
  if (readback?.status === 'set' && readback.draft?.draftRevision !== input.expectedDraftRevision) {
    return readback;
  }
  return 'unavailable';
}

export async function getReplyDraftWithNativeComposerOwner(
  input: NativeComposerReplyDraftRoomInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeComposerReplyDraftReadback | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_composer_get_reply_draft', { request: input });
  if (!result.available) return 'unavailable';
  const readback = acceptReplyDraftReadback(result.value, input.roomId);
  return readback?.status === 'set' || readback?.status === 'empty' ? readback : 'unavailable';
}
