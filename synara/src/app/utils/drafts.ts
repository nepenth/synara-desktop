import { Descendant } from 'slate';
import { ROOM_DRAFT_STORAGE_PREFIX } from '../state/sessions';

const DRAFT_VERSION = 1;
const MAX_DRAFT_BYTES = 64 * 1024;

export type StoredRoomDraft = {
  version: number;
  updatedAt: number;
  value: Descendant[];
};

export const getRoomDraftStorageKey = (userId: string, roomId: string): string =>
  `${ROOM_DRAFT_STORAGE_PREFIX}${encodeURIComponent(userId)}.${encodeURIComponent(roomId)}`;

export const normalizeStoredRoomDraft = (value: unknown): StoredRoomDraft | undefined => {
  if (!value || typeof value !== 'object') return undefined;
  const draft = value as Partial<StoredRoomDraft>;
  const { updatedAt } = draft;
  if (draft.version !== DRAFT_VERSION) return undefined;
  if (typeof updatedAt !== 'number' || !Number.isFinite(updatedAt)) return undefined;
  if (!Array.isArray(draft.value)) return undefined;
  return {
    version: DRAFT_VERSION,
    updatedAt,
    value: draft.value,
  };
};

export const loadRoomDraft = (
  storage: Pick<Storage, 'getItem'>,
  userId: string,
  roomId: string
): Descendant[] | undefined => {
  const raw = storage.getItem(getRoomDraftStorageKey(userId, roomId));
  if (!raw || raw.length > MAX_DRAFT_BYTES) return undefined;

  try {
    return normalizeStoredRoomDraft(JSON.parse(raw))?.value;
  } catch {
    return undefined;
  }
};

/** When the stored draft last changed, for cross-device sync. */
export const loadRoomDraftUpdatedAt = (
  storage: Pick<Storage, 'getItem'>,
  userId: string,
  roomId: string
): number | undefined => {
  const raw = storage.getItem(getRoomDraftStorageKey(userId, roomId));
  if (!raw || raw.length > MAX_DRAFT_BYTES) return undefined;
  try {
    return normalizeStoredRoomDraft(JSON.parse(raw))?.updatedAt;
  } catch {
    return undefined;
  }
};

export const saveRoomDraft = (
  storage: Pick<Storage, 'setItem' | 'removeItem'>,
  userId: string,
  roomId: string,
  value: Descendant[],
  now = Date.now()
): boolean => {
  const payload = JSON.stringify({
    version: DRAFT_VERSION,
    updatedAt: now,
    value,
  } satisfies StoredRoomDraft);
  if (payload.length > MAX_DRAFT_BYTES) {
    // Too large to persist: drop the older stored draft so a restart can not
    // bring back text the user has since replaced. The composer keeps the
    // large draft for this session.
    storage.removeItem(getRoomDraftStorageKey(userId, roomId));
    return false;
  }
  storage.setItem(getRoomDraftStorageKey(userId, roomId), payload);
  return true;
};

export const clearRoomDraft = (
  storage: Pick<Storage, 'removeItem'>,
  userId: string,
  roomId: string
): void => {
  storage.removeItem(getRoomDraftStorageKey(userId, roomId));
};
