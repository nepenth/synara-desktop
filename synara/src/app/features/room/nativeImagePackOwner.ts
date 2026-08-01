import type { DesktopInvokeResult } from '../../utils/desktop';
import { ImagePack } from '../../plugins/custom-emoji/ImagePack';
import { PackAddress } from '../../plugins/custom-emoji/PackAddress';
import type { PackContent } from '../../plugins/custom-emoji/types';

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
};

export type NativeImagePackDto = {
  id: string;
  roomId?: string;
  stateKey?: string;
  content: PackContent;
};

export type NativeUserImagePackSnapshot = {
  sessionGeneration: number;
  pack?: NativeImagePackDto | null;
};

export type NativeRoomImagePacksSnapshot = {
  sessionGeneration: number;
  roomId: string;
  packs: NativeImagePackDto[];
};

export type NativeGlobalImagePacksSnapshot = {
  sessionGeneration: number;
  packs: NativeImagePackDto[];
};

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

/**
 * V-SEND.R-PACK-READ: sole pack-read owner when a native Matrix session is live.
 * Fail-closed — never falls through to mx.getAccountData / getStateEvent.
 */
export async function isNativePackReadSession(
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<boolean> {
  if (!desktopAvailable) return false;
  const session = await invoke('matrix_session_snapshot');
  if (!session.available) return false;
  const snapshot = session.value as NativeSessionSnapshot | undefined;
  return snapshot?.status === 'logged_in';
}

export function imagePackFromNativeDto(dto: NativeImagePackDto): ImagePack {
  const address =
    dto.roomId && typeof dto.stateKey === 'string'
      ? new PackAddress(dto.roomId, dto.stateKey)
      : undefined;
  return new ImagePack(dto.id, dto.content ?? {}, address);
}

export async function getUserImagePackWithNativeOwner(
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<ImagePack | undefined | 'legacy'> {
  if (!(await isNativePackReadSession(desktopAvailable, invoke))) {
    return 'legacy';
  }
  const result = await invoke('matrix_get_user_image_pack');
  if (!result.available) {
    throw new Error('Native Matrix image pack is unavailable.');
  }
  const snapshot = result.value as NativeUserImagePackSnapshot | undefined;
  if (!snapshot) {
    throw new Error('Native Matrix image pack is unavailable.');
  }
  if (!snapshot.pack) return undefined;
  return imagePackFromNativeDto(snapshot.pack);
}

export async function getRoomImagePacksWithNativeOwner(
  roomId: string,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<ImagePack[] | 'legacy'> {
  if (!(await isNativePackReadSession(desktopAvailable, invoke))) {
    return 'legacy';
  }
  const result = await invoke('matrix_get_room_image_packs', { roomId });
  if (!result.available) {
    throw new Error('Native Matrix image packs are unavailable.');
  }
  const snapshot = result.value as NativeRoomImagePacksSnapshot | undefined;
  if (!snapshot || !Array.isArray(snapshot.packs)) {
    throw new Error('Native Matrix image packs are unavailable.');
  }
  return snapshot.packs.map(imagePackFromNativeDto);
}

export async function getGlobalImagePacksWithNativeOwner(
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<ImagePack[] | 'legacy'> {
  if (!(await isNativePackReadSession(desktopAvailable, invoke))) {
    return 'legacy';
  }
  const result = await invoke('matrix_get_global_image_packs');
  if (!result.available) {
    throw new Error('Native Matrix image packs are unavailable.');
  }
  const snapshot = result.value as NativeGlobalImagePacksSnapshot | undefined;
  if (!snapshot || !Array.isArray(snapshot.packs)) {
    throw new Error('Native Matrix image packs are unavailable.');
  }
  return snapshot.packs.map(imagePackFromNativeDto);
}
