import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { ImagePack } from './ImagePack';
import { PackAddress } from './PackAddress';
import { ImageUsage, PackContent, PackImages } from './types';

/**
 * V-SEND.R-PACK-READ native pack-read projection.
 *
 * On desktop (native), pack discovery is owned by the Rust host and exposed
 * over read-only IPC (`matrix_get_user_image_pack`, `matrix_get_global_image_packs`,
 * `matrix_get_room_image_packs`). These helpers invoke the native commands and
 * convert the DTO back into the existing `ImagePack` class shape so downstream
 * consumers (emoji board, autocomplete, settings) keep working unchanged.
 *
 * Fail-closed: on a native logged-in session, absence/failure of a native
 * command is terminal — callers must not fall through to `mx.getAccountData` /
 * `mx.getStateEvent`. On non-native web sessions these helpers return
 * `undefined` / `[]` and the legacy JS read path remains in use.
 */

export type NativeImagePackAddress = {
  roomId: string;
  stateKey: string;
};

export type NativeImagePackMeta = {
  name?: string;
  avatar?: string;
  attribution?: string;
  usage: string[];
};

export type NativeImagePackImage = {
  url: string;
  body?: string;
  usage?: string[];
  info?: unknown;
};

export type NativeImagePack = {
  id: string;
  deleted: boolean;
  address?: NativeImagePackAddress;
  meta: NativeImagePackMeta;
  images: Record<string, NativeImagePackImage>;
};

export type NativeImagePackSnapshot = {
  sessionGeneration: number;
  packs: NativeImagePack[];
};

const isNativeImagePack = (value: unknown): value is NativeImagePack => {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.id === 'string' &&
    typeof candidate.deleted === 'boolean' &&
    !!candidate.meta &&
    typeof candidate.meta === 'object' &&
    !!candidate.images &&
    typeof candidate.images === 'object'
  );
};

const isNativeImagePackSnapshot = (value: unknown): value is NativeImagePackSnapshot => {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Record<string, unknown>;
  return Array.isArray(candidate.packs) && candidate.packs.every(isNativeImagePack);
};

const toPackContent = (pack: NativeImagePack): PackContent => {
  const meta = pack.meta ?? {};
  const images: PackImages = {};
  for (const [shortcode, image] of Object.entries(pack.images ?? {})) {
    if (typeof image?.url !== 'string') continue;
    images[shortcode] = {
      url: image.url,
      body: typeof image.body === 'string' ? image.body : undefined,
      usage: Array.isArray(image.usage)
        ? (image.usage.filter(
            (u) => u === ImageUsage.Emoticon || u === ImageUsage.Sticker
          ) as ImageUsage[])
        : undefined,
      info: image.info as PackImages[string]['info'],
    };
  }
  return {
    pack: {
      display_name: typeof meta.name === 'string' ? meta.name : undefined,
      avatar_url: typeof meta.avatar === 'string' ? meta.avatar : undefined,
      attribution: typeof meta.attribution === 'string' ? meta.attribution : undefined,
      usage: Array.isArray(meta.usage)
        ? (meta.usage.filter(
            (u) => u === ImageUsage.Emoticon || u === ImageUsage.Sticker
          ) as ImageUsage[])
        : undefined,
    },
    images,
  };
};

const toImagePack = (pack: NativeImagePack): ImagePack => {
  const address =
    pack.address && typeof pack.address.roomId === 'string'
      ? new PackAddress(pack.address.roomId, pack.address.stateKey)
      : undefined;
  return new ImagePack(pack.id, toPackContent(pack), address);
};

const toImagePacks = (snapshot: NativeImagePackSnapshot | undefined): ImagePack[] =>
  snapshot ? snapshot.packs.map(toImagePack) : [];

/**
 * Fetch the personal `im.ponies.user_emotes` pack from native. Returns
 * `undefined` on non-native web sessions or when no pack is present.
 */
export const fetchNativeUserImagePack = async (): Promise<ImagePack | undefined> => {
  if (!isSynaraDesktop()) return undefined;
  const result = await invokeDesktopWithAvailability<unknown>('matrix_get_user_image_pack');
  if (!result.available || !isNativeImagePackSnapshot(result.value)) return undefined;
  return toImagePacks(result.value)[0];
};

/**
 * Fetch the enabled global packs from native `im.ponies.emote_rooms`. Returns
 * `[]` on non-native web sessions.
 */
export const fetchNativeGlobalImagePacks = async (): Promise<ImagePack[]> => {
  if (!isSynaraDesktop()) return [];
  const result = await invokeDesktopWithAvailability<unknown>('matrix_get_global_image_packs');
  if (!result.available || !isNativeImagePackSnapshot(result.value)) return [];
  return toImagePacks(result.value);
};

/**
 * Fetch a room's `im.ponies.room_emotes` packs from native. Returns `[]` on
 * non-native web sessions.
 */
export const fetchNativeRoomImagePacks = async (roomId: string): Promise<ImagePack[]> => {
  if (!isSynaraDesktop()) return [];
  const result = await invokeDesktopWithAvailability<unknown>('matrix_get_room_image_packs', {
    roomId,
  });
  if (!result.available || !isNativeImagePackSnapshot(result.value)) return [];
  return toImagePacks(result.value);
};
