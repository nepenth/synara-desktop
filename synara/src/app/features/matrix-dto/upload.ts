/**
 * Upload job DTO — no file bytes.
 */

import type { MediaHandleId, RoomId, UploadId } from './ids';
import { hasForbiddenWireFields, isObject, optNumber, optString, reqString } from './parseUtil';

export const UPLOAD_STATES = ['queued', 'uploading', 'completed', 'failed', 'cancelled'] as const;
export type UploadState = typeof UPLOAD_STATES[number];
const UPLOAD_STATE_SET = new Set<string>(UPLOAD_STATES);

export function isUploadState(value: unknown): value is UploadState {
  return typeof value === 'string' && UPLOAD_STATE_SET.has(value);
}

export type UploadJob = {
  uploadId: UploadId;
  roomId?: RoomId;
  fileName: string;
  mimeType?: string;
  sizeBytes?: number;
  state: UploadState;
  progress01?: number;
  mediaHandleId?: MediaHandleId;
};

export function parseUploadJob(value: unknown): UploadJob | null {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const uploadId = reqString(value, 'uploadId');
  const roomId = optString(value, 'roomId');
  const fileName = reqString(value, 'fileName');
  const mimeType = optString(value, 'mimeType');
  const sizeBytes = optNumber(value, 'sizeBytes');
  const progress01 = optNumber(value, 'progress01');
  const mediaHandleId = optString(value, 'mediaHandleId');
  if (
    uploadId === null ||
    roomId === null ||
    fileName === null ||
    mimeType === null ||
    sizeBytes === null ||
    progress01 === null ||
    mediaHandleId === null ||
    !isUploadState(value.state)
  ) {
    return null;
  }
  return {
    uploadId,
    roomId,
    fileName,
    mimeType,
    sizeBytes,
    state: value.state,
    progress01,
    mediaHandleId,
  };
}
