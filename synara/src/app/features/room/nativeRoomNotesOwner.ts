/**
 * SDK-neutral owner for `in.synara.room_notes` account data.
 *
 * Desktop mutations go through native Tauri commands. There is no
 * matrix-js-sdk setAccountData fallback. Consumed by NativeTimelinePresenter
 * after V-TIMELINE.C1/C2 cutover.
 */

import { invokeDesktopWithAvailability, type DesktopInvokeResult } from '../../utils/desktop';
import type {
  SynaraRoomNoteItem,
  SynaraRoomNoteItemKind,
  SynaraRoomNotesContent,
} from '../../../types/matrix/accountData';

export type NativeRoomNotesSnapshot = {
  sessionGeneration: number;
  content: SynaraRoomNotesContent;
};

export type NativeRoomNotesInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<NativeRoomNotesSnapshot>>;

const MAX_NOTE_BODY_LENGTH = 4000;
const MAX_MESSAGE_BODY_LENGTH = 1000;

const defaultInvoke: NativeRoomNotesInvoke = (command, args) =>
  invokeDesktopWithAvailability<NativeRoomNotesSnapshot>(command, args);

async function invokeRoomNotes(
  command: string,
  args: Record<string, unknown> | undefined,
  invoke: NativeRoomNotesInvoke
): Promise<NativeRoomNotesSnapshot> {
  const result = await invoke(command, args);
  if (!result.available || !result.value) {
    throw new Error('Native Matrix room notes account data is unavailable.');
  }
  return result.value;
}

const limitText = (value: string, maxLength: number): string => value.trim().slice(0, maxLength);

const createRoomNoteId = (kind: SynaraRoomNoteItemKind, now = Date.now()): string =>
  `${kind}:${now.toString(36)}:${Math.random().toString(36).slice(2, 10)}`;

export function createManualRoomNoteItem(
  roomId: string,
  kind: Extract<SynaraRoomNoteItemKind, 'note' | 'todo'>,
  body: string,
  now = Date.now()
): SynaraRoomNoteItem | undefined {
  const normalizedBody = limitText(body, MAX_NOTE_BODY_LENGTH);
  if (!normalizedBody) return undefined;
  return {
    id: createRoomNoteId(kind, now),
    kind,
    roomId,
    body: normalizedBody,
    createdAt: now,
    updatedAt: now,
    order: kind === 'todo' ? now : undefined,
  };
}

export function createMessageRoomNoteItemFromIds(input: {
  roomId: string;
  eventId: string;
  body?: string;
  eventTs?: number;
  sender?: string;
  now?: number;
}): SynaraRoomNoteItem {
  const now = input.now ?? Date.now();
  const body =
    typeof input.body === 'string' ? limitText(input.body, MAX_MESSAGE_BODY_LENGTH) : undefined;
  return {
    id: `${input.roomId}\n${input.eventId}`,
    kind: 'message',
    roomId: input.roomId,
    eventId: input.eventId,
    eventTs: input.eventTs,
    sender: input.sender,
    body: body || undefined,
    createdAt: now,
    updatedAt: now,
  };
}

export function snapshotRoomNotesWithNativeOwner(invoke: NativeRoomNotesInvoke = defaultInvoke) {
  return invokeRoomNotes('matrix_room_notes_snapshot', undefined, invoke);
}

export function upsertRoomNoteWithNativeOwner(
  item: SynaraRoomNoteItem,
  invoke: NativeRoomNotesInvoke = defaultInvoke
) {
  return invokeRoomNotes('matrix_room_notes_upsert', { item }, invoke);
}

export function deleteRoomNoteWithNativeOwner(
  roomId: string,
  itemId: string,
  invoke: NativeRoomNotesInvoke = defaultInvoke
) {
  return invokeRoomNotes('matrix_room_notes_delete', { roomId, itemId }, invoke);
}

export function completeRoomTodoWithNativeOwner(
  roomId: string,
  itemId: string,
  completed: boolean,
  invoke: NativeRoomNotesInvoke = defaultInvoke
) {
  return invokeRoomNotes('matrix_room_notes_complete_todo', { roomId, itemId, completed }, invoke);
}

export function moveRoomTodoWithNativeOwner(
  roomId: string,
  itemId: string,
  direction: 'up' | 'down',
  invoke: NativeRoomNotesInvoke = defaultInvoke
) {
  return invokeRoomNotes('matrix_room_notes_move_todo', { roomId, itemId, direction }, invoke);
}
