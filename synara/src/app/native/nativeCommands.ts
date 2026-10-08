import { isObject } from '../features/matrix-dto/parseUtil';
import type { EventId, RoomId } from '../features/matrix-dto/ids';
import { nativeThumbnailContentUri } from '../matrix/nativeThumbnail';
import { invokeDesktopWithAvailability } from '../utils/desktop';
import { getNativeIdentity } from '../state/nativeIdentity';
import {
  parseCryptoStatus,
  parseMediaConfig,
  parseOwnProfile,
  parseSendTextResult,
  parseTimelineEventReadback,
  parseUploadMediaResult,
  parseUserDirectorySearch,
  UNAVAILABLE_MESSAGE,
  type NativeCryptoStatus,
  type NativeInvoke,
  type NativeInvokeResult,
  type NativeMediaConfig,
  type NativeSendStateEventResult,
  type NativeSendTextResult,
  type NativeTimelineEventReading,
  type NativeUploadMediaResult,
  type NativeUserDirectoryResult,
} from './nativeWire';

/**
 * Renderer calls into native commands that have no dedicated owner module.
 * Each one fails closed: an unavailable desktop shell resolves `null` (or
 * throws where the caller must surface an error), never a fabricated success.
 */

let invoke: NativeInvoke = (command, args) => invokeDesktopWithAvailability(command, args);

/** Route commands through a fake invoke (tests). */
export const setNativeCommandInvokeForTests = (next: NativeInvoke): void => {
  invoke = next;
};

/** Send an `m.room.message` through `matrix_send_text`. */
export const sendNativeMessage = async (
  roomId: RoomId,
  content: Record<string, unknown>
): Promise<NativeSendTextResult | null> => {
  const body = typeof content.body === 'string' ? content.body : '';
  const result = await invoke('matrix_send_text', {
    roomId,
    body,
    msgType: typeof content.msgtype === 'string' ? content.msgtype : undefined,
    formattedBody: typeof content.formatted_body === 'string' ? content.formatted_body : undefined,
    txnId: typeof content.txnid === 'string' ? content.txnid : undefined,
    ...(typeof content['m.mentions'] === 'object' ? { mentions: content['m.mentions'] } : {}),
  });
  if (!result.available) return null;
  return parseSendTextResult(result.value);
};

/**
 * Send a room event. Only `m.room.message` has a native send path; other
 * event types resolve `null`.
 */
export const sendNativeEvent = (
  roomId: RoomId,
  type: string,
  content: Record<string, unknown>
): Promise<NativeSendTextResult | null> =>
  type === 'm.room.message' ? sendNativeMessage(roomId, content) : Promise.resolve(null);

/** Name, topic and avatar use their dedicated commands; other state uses the generic send. */
export const sendNativeStateEvent = async (
  roomId: RoomId,
  type: string,
  content: Record<string, unknown>,
  stateKey?: string
): Promise<NativeSendStateEventResult | null> => {
  if (type === 'm.room.name' || type === 'm.room.topic' || type === 'm.room.avatar') {
    const command =
      type === 'm.room.name'
        ? 'matrix_set_room_name'
        : type === 'm.room.topic'
          ? 'matrix_set_room_topic'
          : 'matrix_set_room_avatar';
    const result = await invoke(command, {
      roomId,
      name: content.name,
      topic: content.topic,
      mxc: content.url,
      avatarUrl: content.url,
    });
    if (!result.available) return null;
    return isObject(result.value) ? (result.value as NativeSendStateEventResult) : null;
  }
  const result = await invoke('matrix_send_state_event', {
    roomId,
    eventType: type,
    stateKey: stateKey ?? '',
    content,
  });
  if (!result.available) return null;
  return isObject(result.value) ? (result.value as NativeSendStateEventResult) : { status: 'ok' };
};

/** Upload bytes through `matrix_upload_media`. */
export const uploadNativeMedia = async (input: unknown): Promise<NativeUploadMediaResult> => {
  let mimeType = '';
  let bytes: number[] = [];
  if (isObject(input)) {
    const record = input as Record<string, unknown>;
    mimeType = typeof record.mimeType === 'string' ? record.mimeType : '';
    const rawBytes = record.bytes;
    bytes = Array.isArray(rawBytes)
      ? rawBytes.filter((byte): byte is number => typeof byte === 'number')
      : [];
  }
  const result = await invoke('matrix_upload_media', { mimeType, bytes });
  if (!result.available) {
    throw new Error('Native media upload is unavailable.');
  }
  const parsed = parseUploadMediaResult(result.value);
  if (!parsed) {
    throw new Error('Native media upload returned an invalid result.');
  }
  return parsed;
};

/** The homeserver's upload size limit (`matrix_media_config`). */
export const getNativeMediaConfig = async (): Promise<NativeMediaConfig> => {
  const result = await invoke('matrix_media_config');
  if (!result.available) return {};
  return parseMediaConfig(result.value);
};

/** The signed-in user's profile. Other users' profiles have no native read and resolve `{}`. */
export const getNativeProfileInfo = async (
  userId?: string
): Promise<{ avatar_url?: string; displayname?: string }> => {
  const ownId = getNativeIdentity().userId;
  if (userId && ownId && userId !== ownId) {
    return {};
  }
  const result = await invoke('matrix_get_own_profile');
  const profile = result.available ? parseOwnProfile(result.value) : null;
  if (!profile) {
    throw new Error('Native Matrix profile is unavailable.');
  }
  return { avatar_url: profile.avatarUrl, displayname: profile.displayName };
};

export const setNativeDisplayName = async (displayName: string): Promise<NativeInvokeResult> => {
  const result = await invoke('matrix_set_own_display_name', { displayName });
  if (!result.available) throw new Error(UNAVAILABLE_MESSAGE);
  return result.value as NativeInvokeResult;
};

export const setNativeAvatarUrl = async (mxc: string): Promise<NativeInvokeResult> => {
  const result = await invoke('matrix_set_own_avatar', { avatarUrl: mxc });
  if (!result.available) throw new Error(UNAVAILABLE_MESSAGE);
  return result.value as NativeInvokeResult;
};

/** One event read back through `matrix_timeline_event_readback`. */
export const fetchNativeRoomEvent = async (
  roomId: RoomId,
  eventId: EventId
): Promise<NativeTimelineEventReading | null> => {
  const result = await invoke('matrix_timeline_event_readback', { roomId, eventId });
  if (!result.available) return null;
  return parseTimelineEventReadback(result.value);
};

/** Crypto status only; key material never crosses IPC. */
export const getNativeCryptoStatus = async (): Promise<NativeCryptoStatus | null> => {
  const result = await invoke('matrix_crypto_status');
  if (!result.available) return null;
  return parseCryptoStatus(result.value);
};

export const searchNativeUserDirectory = async (opts: {
  term: string;
  limit?: number;
}): Promise<NativeUserDirectoryResult> => {
  const result = await invoke('matrix_user_directory_search', {
    term: opts.term,
    limit: opts.limit,
  });
  if (!result.available) {
    return { limited: false, results: [] };
  }
  return parseUserDirectorySearch(result.value);
};

/** Redact a message or event (`matrix_timeline_redact`). */
export const redactNativeEvent = async (
  roomId: RoomId,
  eventId: EventId,
  reason?: string
): Promise<{ event_id: string } | null> => {
  const result = await invoke('matrix_timeline_redact', { roomId, eventId, reason });
  if (!result.available) return null;
  return isObject(result.value) &&
    typeof (result.value as { event_id?: string }).event_id === 'string'
    ? { event_id: (result.value as { event_id: string }).event_id }
    : null;
};

/**
 * Turn an `mxc://` URI into the native media URI the webview loads. A
 * requested size becomes a native thumbnail so avatars do not download the
 * original upload. Anything that is not an `mxc://` URI resolves `null`.
 */
export const mxcUrlToNative = (
  mxcUrl: string,
  width?: number,
  height?: number,
  resizeMethod?: string
): string | null => {
  if (!mxcUrl.startsWith('mxc://')) return null;
  if (!width || !height) return mxcUrl;
  return nativeThumbnailContentUri(
    mxcUrl,
    width,
    height,
    resizeMethod === 'scale' ? 'scale' : 'crop'
  );
};
