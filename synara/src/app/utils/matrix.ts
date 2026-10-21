import { type MatrixEventReading, type RoomReading } from './room';
import { IImageInfo, IVideoInfo } from '../../types/matrix/common';
import { getStateEvent } from './room';
import { Membership, StateEvent } from '../../types/matrix/room';

import { getNativeRoom, getNativeRooms, getNativeRoomSummary } from '../native/nativeSession';
import { mxcUrlToNative } from '../native/nativeCommands';
import type { BulkActionFailure } from './bulkActions';
export type UploadProgress = {
  loaded: number;
  total?: number;
};

export type UploadResponse = {
  content_uri: string;
};

/**
 * Structural local re-type of the js-sdk MatrixError, matching the subset the
 * app consumes (message/errcode/data). The web client (runtime) still throws
 * its own MatrixError instances; this keeps upload error handling SDK-neutral.
 */
export class MatrixError extends Error {
  public errcode: string | undefined;

  public data: unknown;

  public httpStatus: number | undefined;

  constructor(
    data: { error?: unknown; errcode?: string; httpStatus?: number; [k: string]: unknown } = {}
  ) {
    const message = typeof data.error === 'string' ? data.error : (data.errcode ?? 'Unknown error');
    super(message);
    this.name = 'MatrixError';
    this.errcode = data.errcode;
    this.data = data;
    if (typeof data.httpStatus === 'number') {
      this.httpStatus = data.httpStatus;
    }
  }

  isRateLimitError(): boolean {
    return this.httpStatus === 429;
  }

  getRetryAfterMs(): number | undefined {
    const d = (this.data ?? {}) as { retry_after_ms?: number; __m_retry_after_ms?: number };
    return d.retry_after_ms ?? d.__m_retry_after_ms;
  }
}

type DMRoomReading = RoomReading & {
  getCanonicalAlias(): string | null;
};

const DOMAIN_REGEX = /\b(?:[a-zA-Z0-9-]+\.)+[a-zA-Z]{2,}\b/;

export const isServerName = (serverName: string): boolean => DOMAIN_REGEX.test(serverName);

const matchMxId = (id: string): RegExpMatchArray | null => id.match(/^([@$+#])([^\s:]+):(\S+)$/);

const validMxId = (id: string): boolean => !!matchMxId(id);

export const getMxIdServer = (userId: string): string | undefined => matchMxId(userId)?.[3];

export const getMxIdLocalPart = (userId: string): string | undefined => matchMxId(userId)?.[2];

export const isUserId = (id: string): boolean => validMxId(id) && id.startsWith('@');

export const isRoomId = (id: string): boolean => id.startsWith('!');

export const isRoomAlias = (id: string): boolean => validMxId(id) && id.startsWith('#');

export const getCanonicalAliasRoomId = (alias: string): string | undefined =>
  getNativeRooms()?.find(
    (room) =>
      (room as DMRoomReading).getCanonicalAlias() === alias &&
      getStateEvent(room, StateEvent.RoomTombstone) === undefined
  )?.roomId;

export const getCanonicalAliasOrRoomId = (roomId: string): string => {
  const room = getNativeRoom(roomId);
  if (!room) return roomId;
  if (getStateEvent(room, StateEvent.RoomTombstone) !== undefined) return roomId;
  const alias = (room as DMRoomReading).getCanonicalAlias();
  if (alias && getCanonicalAliasRoomId(alias) === roomId) {
    return alias;
  }
  return roomId;
};

export const getImageInfo = (img: HTMLImageElement, fileOrBlob: File | Blob): IImageInfo => {
  const info: IImageInfo = {};
  info.w = img.width;
  info.h = img.height;
  info.mimetype = fileOrBlob.type;
  info.size = fileOrBlob.size;
  return info;
};

export const getVideoInfo = (video: HTMLVideoElement, fileOrBlob: File | Blob): IVideoInfo => {
  const info: IVideoInfo = {};
  info.duration = Number.isNaN(video.duration) ? undefined : Math.floor(video.duration * 1000);
  info.w = video.videoWidth;
  info.h = video.videoHeight;
  info.mimetype = fileOrBlob.type;
  info.size = fileOrBlob.size;
  return info;
};

export type TUploadContent = File;

export const matrixEventByRecency = (m1: MatrixEventReading, m2: MatrixEventReading) =>
  m2.getTs() - m1.getTs();

export const factoryEventSentBy = (senderId: string) => (ev: MatrixEventReading) =>
  ev.getSender() === senderId;

export const eventWithShortcode = (ev: MatrixEventReading) =>
  typeof ev.getContent().shortcode === 'string';

/**
 * The joined direct-message room with exactly `userId`, from Core's room
 * summaries (`isDirect` + `directUserId`, which Core derives from `m.direct`).
 * The most recently active one wins when there are several.
 */
export const getDMRoomFor = (userId: string): RoomReading | undefined => {
  let best: { room: RoomReading; activity: number } | undefined;
  getNativeRooms().forEach((room) => {
    const summary = getNativeRoomSummary(room.roomId);
    if (
      !summary ||
      summary.membership !== Membership.Join ||
      !summary.isDirect ||
      summary.directUserId !== userId
    ) {
      return;
    }
    const activity = summary.lastActivityTs ?? 0;
    if (!best || activity > best.activity) best = { room, activity };
  });
  return best?.room;
};

/**
 * The other person in a direct-message room, or `undefined` when Core does
 * not know exactly one. Never falls back to the signed-in user: writing
 * `m.direct` with yourself as the peer corrupts the DM map (D15).
 */
export const guessDmRoomUserId = (room: RoomReading, myUserId: string): string | undefined => {
  const summary = getNativeRoomSummary(room.roomId);
  if (!summary) return undefined;
  if (summary.directUserId && summary.directUserId !== myUserId) return summary.directUserId;
  const heroes = (summary.heroes ?? []).filter((hero) => hero.userId !== myUserId);
  return heroes.length === 1 ? heroes[0].userId : undefined;
};

export const mxcUrlToHttp = (
  mxcUrl: string,
  _useAuthentication?: boolean,
  width?: number,
  height?: number,
  resizeMethod?: string
): string | null => mxcUrlToNative(mxcUrl, width, height, resizeMethod);

export const downloadMedia = async (src: string): Promise<Blob> => {
  const res = await fetch(src, { method: 'GET' });
  const blob = await res.blob();
  return blob;
};

/**
 * Run `callback` for each item in order, waiting out 429 rate limits. Other
 * failures do not stop the run; they are returned so callers can report which
 * items did not complete instead of claiming success.
 */
export const rateLimitedActions = async <T, R = void>(
  data: T[],
  callback: (item: T, index: number) => Promise<R>,
  maxRetryCount?: number
): Promise<BulkActionFailure<T>[]> => {
  let retryCount = 0;

  let actionInterval = 0;
  const failures: BulkActionFailure<T>[] = [];

  const sleepForMs = (ms: number) =>
    new Promise((resolve) => {
      setTimeout(resolve, ms);
    });

  const performAction = async (dataItem: T, index: number) => {
    // A rejection may not be an object at all (`undefined`, a string).
    let err: MatrixError | undefined;
    try {
      await callback(dataItem, index);
      return;
    } catch (error) {
      err = error as MatrixError | undefined;
    }

    if (err?.httpStatus === 429 && retryCount !== maxRetryCount) {
      const waitMS =
        (typeof err.getRetryAfterMs === 'function' ? err.getRetryAfterMs() : undefined) ?? 3000;
      actionInterval = waitMS * 1.5;
      await sleepForMs(waitMS);
      retryCount += 1;

      await performAction(dataItem, index);
      return;
    }

    failures.push({ item: dataItem, error: err });
  };

  for (let i = 0; i < data.length; i += 1) {
    const dataItem = data[i];
    retryCount = 0;
    // eslint-disable-next-line no-await-in-loop
    await performAction(dataItem, i);
    if (actionInterval > 0) {
      // eslint-disable-next-line no-await-in-loop
      await sleepForMs(actionInterval);
    }
  }

  return failures;
};

export const knockSupported = (version: string): boolean => {
  const unsupportedVersion = ['1', '2', '3', '4', '5', '6'];
  return !unsupportedVersion.includes(version);
};
export const restrictedSupported = (version: string): boolean => {
  const unsupportedVersion = ['1', '2', '3', '4', '5', '6', '7'];
  return !unsupportedVersion.includes(version);
};
export const knockRestrictedSupported = (version: string): boolean => {
  const unsupportedVersion = ['1', '2', '3', '4', '5', '6', '7', '8', '9'];
  return !unsupportedVersion.includes(version);
};
export const creatorsSupported = (version: string): boolean => {
  const unsupportedVersion = ['1', '2', '3', '4', '5', '6', '7', '8', '9', '10', '11'];
  return !unsupportedVersion.includes(version);
};
