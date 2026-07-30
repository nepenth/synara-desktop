/**
 * SDK-neutral owner for native composer reply-draft state.
 *
 * Message body drafts remain local (Slate / localStorage). Reply transport
 * remains `matrix_send_text` with `replyTo`. This owner does not select
 * NativeTimelinePresenter or delete RoomTimeline.
 */

import type { DesktopInvokeResult } from '../../utils/desktop';
import type { IReplyDraft } from '../../state/room/roomInputDrafts';

export const NATIVE_COMPOSER_REPLY_DRAFT_SCHEMA_VERSION = 1;

export type NativeComposerReplyDraft = {
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

export type NativeComposerSetReplyDraftInput = {
  roomId: string;
  eventId: string;
  startThread?: boolean;
};

export type NativeComposerReplyDraftRoomInput = {
  roomId: string;
};

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

const DRAFT_STATUSES = new Set<NativeComposerReplyDraftStatus>(['set', 'cleared', 'empty']);

const acceptReplyDraftReadback = (value: unknown): NativeComposerReplyDraftReadback | undefined => {
  if (!value || typeof value !== 'object') return undefined;
  const readback = value as NativeComposerReplyDraftReadback;
  if (
    readback.schemaVersion !== NATIVE_COMPOSER_REPLY_DRAFT_SCHEMA_VERSION ||
    typeof readback.roomId !== 'string' ||
    !DRAFT_STATUSES.has(readback.status)
  ) {
    return undefined;
  }
  if (readback.draft) {
    const draft = readback.draft;
    if (
      typeof draft.eventId !== 'string' ||
      typeof draft.senderId !== 'string' ||
      typeof draft.body !== 'string'
    ) {
      return undefined;
    }
  } else if (readback.status === 'set') {
    return undefined;
  }
  return readback;
};

export const mapNativeReplyDraftToJs = (draft: NativeComposerReplyDraft): IReplyDraft => ({
  userId: draft.senderId,
  eventId: draft.eventId,
  body: draft.body,
  formattedBody: draft.formattedBody,
  relation: draft.threadRootEventId
    ? { rel_type: 'm.thread', event_id: draft.threadRootEventId }
    : undefined,
});

export async function setReplyDraftWithNativeComposerOwner(
  input: NativeComposerSetReplyDraftInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeComposerReplyDraftReadback | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_composer_set_reply_draft', { request: input });
  if (!result.available) return 'unavailable';
  return acceptReplyDraftReadback(result.value) ?? 'unavailable';
}

export async function clearReplyDraftWithNativeComposerOwner(
  input: NativeComposerReplyDraftRoomInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeComposerReplyDraftReadback | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_composer_clear_reply_draft', { request: input });
  if (!result.available) return 'unavailable';
  return acceptReplyDraftReadback(result.value) ?? 'unavailable';
}

export async function getReplyDraftWithNativeComposerOwner(
  input: NativeComposerReplyDraftRoomInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeComposerReplyDraftReadback | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_composer_get_reply_draft', { request: input });
  if (!result.available) return 'unavailable';
  return acceptReplyDraftReadback(result.value) ?? 'unavailable';
}
