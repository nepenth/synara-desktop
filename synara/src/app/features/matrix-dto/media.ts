/**
 * Media handle DTO — metadata + handles only; no bytes, no key material.
 */

import type { MediaHandleId } from './ids';
import { hasForbiddenWireFields, isObject, optNumber, optString, reqString } from './parseUtil';

export const MEDIA_SOURCES = ['mxc', 'local_cache', 'upload'] as const;
export type MediaSource = typeof MEDIA_SOURCES[number];
const MEDIA_SOURCE_SET = new Set<string>(MEDIA_SOURCES);

export function isMediaSource(value: unknown): value is MediaSource {
  return typeof value === 'string' && MEDIA_SOURCE_SET.has(value);
}

export type MediaHandle = {
  handleId: MediaHandleId;
  mxcUri?: string;
  source?: MediaSource;
  mimeType?: string;
  sizeBytes?: number;
  width?: number;
  height?: number;
  durationMs?: number;
  thumbnailHandleId?: MediaHandleId;
};

export function parseMediaHandle(value: unknown): MediaHandle | null {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const handleId = reqString(value, 'handleId');
  const mxcUri = optString(value, 'mxcUri');
  const mimeType = optString(value, 'mimeType');
  const sizeBytes = optNumber(value, 'sizeBytes');
  const width = optNumber(value, 'width');
  const height = optNumber(value, 'height');
  const durationMs = optNumber(value, 'durationMs');
  const thumbnailHandleId = optString(value, 'thumbnailHandleId');
  if (
    handleId === null ||
    mxcUri === null ||
    mimeType === null ||
    sizeBytes === null ||
    width === null ||
    height === null ||
    durationMs === null ||
    thumbnailHandleId === null
  ) {
    return null;
  }
  let source: MediaSource | undefined;
  if (value.source !== undefined) {
    if (!isMediaSource(value.source)) return null;
    source = value.source;
  }
  return {
    handleId,
    mxcUri,
    source,
    mimeType,
    sizeBytes,
    width,
    height,
    durationMs,
    thumbnailHandleId,
  };
}
