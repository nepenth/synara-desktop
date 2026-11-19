/**
 * Sole write owner for user room folders (`in.synara.room_folders`) on
 * desktop. Every change goes through Core's `matrix_room_folders_*` commands,
 * which do the read-modify-write, validation and sync-echo handling
 * (`docs/design/room-folders.md`). There is no js-sdk or localStorage
 * fallback.
 *
 * Named `roomFolder` so it cannot be confused with the space switcher's
 * *space* folders (`in.synara.spaces`).
 */

import { invokeDesktopWithAvailability, type DesktopInvokeResult } from '../../utils/desktop';
import type {
  NativeRoomFolderCreateResult,
  NativeRoomFoldersSnapshot,
} from '../matrix-dto/generated';

export type NativeRoomFolderInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

const defaultInvoke: NativeRoomFolderInvoke = (command, args) =>
  invokeDesktopWithAvailability<unknown>(command, args);

/** Core's limits (`room_folders.rs`). */
export const ROOM_FOLDER_NAME_MAX = 64;
export const ROOM_FOLDER_MAX = 100;

const UNAVAILABLE = 'Native room folders are unavailable.';

const isSnapshot = (value: unknown): value is NativeRoomFoldersSnapshot => {
  if (!value || typeof value !== 'object') return false;
  const record = value as Record<string, unknown>;
  return (
    typeof record.readOnly === 'boolean' &&
    typeof record.version === 'number' &&
    Array.isArray(record.folders) &&
    record.folders.every(
      (folder) =>
        !!folder &&
        typeof folder === 'object' &&
        typeof (folder as Record<string, unknown>).id === 'string' &&
        typeof (folder as Record<string, unknown>).name === 'string' &&
        Array.isArray((folder as Record<string, unknown>).roomIds)
    )
  );
};

async function invokeValue(
  command: string,
  args: Record<string, unknown> | undefined,
  invoke: NativeRoomFolderInvoke
): Promise<unknown> {
  const result = await invoke(command, args);
  if (!result.available || result.value === undefined) throw new Error(UNAVAILABLE);
  return result.value;
}

async function invokeSnapshot(
  command: string,
  args: Record<string, unknown> | undefined,
  invoke: NativeRoomFolderInvoke
): Promise<NativeRoomFoldersSnapshot> {
  const value = await invokeValue(command, args, invoke);
  if (!isSnapshot(value)) throw new Error(UNAVAILABLE);
  return value;
}

/**
 * Core's name rule, mirrored for inline feedback before the round trip:
 * trimmed, inner whitespace collapsed, 1–64 characters (code points).
 */
export const normalizeRoomFolderName = (name: string): string => name.trim().replace(/\s+/g, ' ');

export const isValidRoomFolderName = (name: string): boolean => {
  const length = [...normalizeRoomFolderName(name)].length;
  return length >= 1 && length <= ROOM_FOLDER_NAME_MAX;
};

export function snapshotRoomFoldersWithNativeOwner(
  invoke: NativeRoomFolderInvoke = defaultInvoke
): Promise<NativeRoomFoldersSnapshot> {
  return invokeSnapshot('matrix_room_folders_snapshot', undefined, invoke);
}

/** Creates a folder at the end; with `roomId`, files that room in the same write. */
export async function createRoomFolderWithNativeOwner(
  name: string,
  roomId?: string,
  invoke: NativeRoomFolderInvoke = defaultInvoke
): Promise<NativeRoomFolderCreateResult> {
  const value = await invokeValue(
    'matrix_room_folders_create',
    roomId
      ? { name: normalizeRoomFolderName(name), roomId }
      : { name: normalizeRoomFolderName(name) },
    invoke
  );
  const record = value as Partial<NativeRoomFolderCreateResult> | null;
  if (!record || typeof record.folderId !== 'string' || !isSnapshot(record.snapshot)) {
    throw new Error(UNAVAILABLE);
  }
  return { folderId: record.folderId, snapshot: record.snapshot };
}

export function renameRoomFolderWithNativeOwner(
  folderId: string,
  name: string,
  invoke: NativeRoomFolderInvoke = defaultInvoke
): Promise<NativeRoomFoldersSnapshot> {
  return invokeSnapshot(
    'matrix_room_folders_rename',
    { folderId, name: normalizeRoomFolderName(name) },
    invoke
  );
}

/** Its rooms become unfiled (they go back to Rooms). */
export function deleteRoomFolderWithNativeOwner(
  folderId: string,
  invoke: NativeRoomFolderInvoke = defaultInvoke
): Promise<NativeRoomFoldersSnapshot> {
  return invokeSnapshot('matrix_room_folders_delete', { folderId }, invoke);
}

export function moveRoomFolderWithNativeOwner(
  folderId: string,
  toIndex: number,
  invoke: NativeRoomFolderInvoke = defaultInvoke
): Promise<NativeRoomFoldersSnapshot> {
  if (!Number.isSafeInteger(toIndex) || toIndex < 0) {
    return Promise.reject(new Error('Invalid folder position.'));
  }
  return invokeSnapshot('matrix_room_folders_move', { folderId, toIndex }, invoke);
}

/** Files `roomId` in `folderId`, or unfiles it with `null`. */
export function setRoomFolderWithNativeOwner(
  roomId: string,
  folderId: string | null,
  invoke: NativeRoomFolderInvoke = defaultInvoke
): Promise<NativeRoomFoldersSnapshot> {
  return invokeSnapshot('matrix_room_folders_set_room', { roomId, folderId }, invoke);
}

/** The `v-room-folders.*` id of a rejected command, if it has one. */
export const roomFolderDiagnosticId = (error: unknown): string | undefined => {
  if (!error || typeof error !== 'object' || Array.isArray(error)) return undefined;
  // Tauri rejects with the serialized `MatrixAuthCommandError` (camelCase).
  const id = (error as Record<string, unknown>).diagnosticId;
  return typeof id === 'string' && id.startsWith('v-room-folders.') ? id : undefined;
};

/** Plain sentence-case copy for a failed folder command. Never the raw error. */
export const roomFolderErrorMessage = (error: unknown): string => {
  switch (roomFolderDiagnosticId(error)) {
    case 'v-room-folders.invalid-name':
      return `Folder names need 1 to ${ROOM_FOLDER_NAME_MAX} characters.`;
    case 'v-room-folders.too-many-folders':
      return `You can have up to ${ROOM_FOLDER_MAX} folders. Delete one to add another.`;
    case 'v-room-folders.content-too-large':
      return 'Your folders are too large to save. Take some rooms out of folders and try again.';
    case 'v-room-folders.unsupported-version':
      return 'Your folders were changed by a newer version of Synara. Update Synara to change them here.';
    case 'v-room-folders.folder-not-found':
      return 'This folder no longer exists.';
    case 'v-room-folders.room-is-direct':
      return "Direct messages can't go in a folder.";
    case 'v-room-folders.room-is-space':
      return "Spaces can't go in a folder.";
    case 'v-room-folders.room-not-joined':
      return 'Join this room before putting it in a folder.';
    default:
      return "Couldn't save your folders. Try again.";
  }
};

/** A newer client owns the format: reload the snapshot so management hides. */
export const isRoomFolderReadOnlyError = (error: unknown): boolean =>
  roomFolderDiagnosticId(error) === 'v-room-folders.unsupported-version';

export type { NativeRoomFoldersSnapshot, NativeRoomFolderCreateResult };
