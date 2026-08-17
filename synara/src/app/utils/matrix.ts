import to from 'await-to-js';
import { type MatrixClientReading, type MatrixEventReading, type RoomReading } from './room';
import { IImageInfo, IVideoInfo } from '../../types/matrix/common';
import { getStateEvent } from './room';
import { Membership, StateEvent } from '../../types/matrix/room';

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
    const message = typeof data.error === 'string' ? data.error : data.errcode ?? 'Unknown error';
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

type SdkRoomMemberReading = {
  userId: string;
  events?: { member?: { getTs(): number } };
};

type DMRoomReading = RoomReading & {
  getCanonicalAlias(): string | null;
  hasEncryptionStateEvent(): boolean;
  getJoinedMembers(): SdkRoomMemberReading[];
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

export const getCanonicalAliasRoomId = (
  mx: MatrixClientReading,
  alias: string
): string | undefined =>
  mx
    .getRooms()
    ?.find(
      (room) =>
        (room as DMRoomReading).getCanonicalAlias() === alias &&
        getStateEvent(room, StateEvent.RoomTombstone) === undefined
    )?.roomId;

export const getCanonicalAliasOrRoomId = (mx: MatrixClientReading, roomId: string): string => {
  const room = mx.getRoom(roomId);
  if (!room) return roomId;
  if (getStateEvent(room, StateEvent.RoomTombstone) !== undefined) return roomId;
  const alias = (room as DMRoomReading).getCanonicalAlias();
  if (alias && getCanonicalAliasRoomId(mx, alias) === roomId) {
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

export type ContentUploadOptions = {
  name?: string;
  fileType?: string;
  hideFilename?: boolean;
  onPromise?: (promise: Promise<UploadResponse>) => void;
  onProgress?: (progress: UploadProgress) => void;
  onSuccess: (mxc: string) => void;
  onError: (error: MatrixError) => void;
};

export const uploadContent = async (
  mx: MatrixClientReading,
  file: TUploadContent,
  options: ContentUploadOptions
) => {
  const { name, fileType, hideFilename, onProgress, onPromise, onSuccess, onError } = options;

  const uploadClient = mx as unknown as {
    uploadContent(
      file: TUploadContent,
      opts: {
        name?: string;
        type?: string;
        includeFilename?: boolean;
        progressHandler?: (progress: UploadProgress) => void;
      }
    ): Promise<UploadResponse>;
  };
  const uploadPromise = uploadClient.uploadContent(file, {
    name,
    type: fileType,
    includeFilename: !hideFilename,
    progressHandler: onProgress,
  });
  onPromise?.(uploadPromise);
  try {
    const data = await uploadPromise;
    const mxc = data.content_uri;
    if (mxc) onSuccess(mxc);
    else onError(new MatrixError(data));
  } catch (e: any) {
    const message = typeof e?.message === 'string' ? e.message : undefined;
    const errcode = typeof e?.errcode === 'string' ? e.errcode : undefined;
    onError(new MatrixError({ error: message, errcode }));
  }
};

export const matrixEventByRecency = (m1: MatrixEventReading, m2: MatrixEventReading) =>
  m2.getTs() - m1.getTs();

export const factoryEventSentBy = (senderId: string) => (ev: MatrixEventReading) =>
  ev.getSender() === senderId;

export const eventWithShortcode = (ev: MatrixEventReading) =>
  typeof ev.getContent().shortcode === 'string';

export const getDMRoomFor = (mx: MatrixClientReading, userId: string): RoomReading | undefined => {
  const dmLikeRooms = mx
    .getRooms()
    .filter(
      (room) =>
        room.getMyMembership() === Membership.Join &&
        (room as DMRoomReading).hasEncryptionStateEvent() &&
        room.getMembers().length <= 2
    );

  return dmLikeRooms.find((room) => room.getMember(userId));
};

export const guessDmRoomUserId = (room: RoomReading, myUserId: string): string => {
  const getOldestMember = (members: SdkRoomMemberReading[]): SdkRoomMemberReading | undefined => {
    let oldestMemberTs: number | undefined;
    let oldestMember: SdkRoomMemberReading | undefined;

    const pickOldestMember = (member: SdkRoomMemberReading) => {
      if (member.userId === myUserId) return;

      if (
        oldestMemberTs === undefined ||
        (member.events?.member && member.events.member.getTs() < oldestMemberTs)
      ) {
        oldestMember = member;
        oldestMemberTs = member.events?.member?.getTs();
      }
    };

    members.forEach(pickOldestMember);

    return oldestMember;
  };

  // Pick the joined user who's been here longest (and isn't us),
  const member = getOldestMember((room as DMRoomReading).getJoinedMembers());
  if (member) return member.userId;

  // if there are no joined members other than us, use the oldest member
  const currentState = room.currentState as
    | (typeof room.currentState & {
        getMembers(): SdkRoomMemberReading[];
      })
    | undefined;
  const member1 = getOldestMember(currentState?.getMembers() ?? []);
  return member1?.userId ?? myUserId;
};

export const mxcUrlToHttp = (
  mx: MatrixClientReading,
  mxcUrl: string,
  useAuthentication?: boolean,
  width?: number,
  height?: number,
  resizeMethod?: string,
  allowDirectLinks?: boolean,
  allowRedirects?: boolean
): string | null =>
  mx.mxcUrlToHttp(
    mxcUrl,
    width,
    height,
    resizeMethod,
    allowDirectLinks,
    allowRedirects,
    useAuthentication
  );

export const downloadMedia = async (src: string): Promise<Blob> => {
  const res = await fetch(src, { method: 'GET' });
  const blob = await res.blob();
  return blob;
};

export const rateLimitedActions = async <T, R = void>(
  data: T[],
  callback: (item: T, index: number) => Promise<R>,
  maxRetryCount?: number
) => {
  let retryCount = 0;

  let actionInterval = 0;

  const sleepForMs = (ms: number) =>
    new Promise((resolve) => {
      setTimeout(resolve, ms);
    });

  const performAction = async (dataItem: T, index: number) => {
    const [err] = await to<R, MatrixError>(callback(dataItem, index));

    if (err?.httpStatus === 429) {
      if (retryCount === maxRetryCount) {
        return;
      }

      const waitMS = err.getRetryAfterMs() ?? 3000;
      actionInterval = waitMS * 1.5;
      await sleepForMs(waitMS);
      retryCount += 1;

      await performAction(dataItem, index);
    }
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
