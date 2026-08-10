/**
 * Space hierarchy summary DTO.
 */

import type { RoomId } from './ids';
import {
  hasForbiddenWireFields,
  isObject,
  optBoolean,
  optString,
  reqString,
  stringArray,
} from './parseUtil';

export type SpaceChild = {
  roomId: RoomId;
  order?: string;
  suggested?: boolean;
};

export type SpaceSummary = {
  roomId: RoomId;
  name?: string;
  avatarUrl?: string;
  children: SpaceChild[];
  parentRoomIds?: RoomId[];
};

function parseChild(value: unknown): SpaceChild | null {
  if (!isObject(value)) return null;
  const roomId = reqString(value, 'roomId');
  const order = optString(value, 'order');
  const suggested = optBoolean(value, 'suggested');
  if (roomId === null || order === null || suggested === null) return null;
  return { roomId, order, suggested };
}

export function parseSpaceSummary(value: unknown): SpaceSummary | null {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const roomId = reqString(value, 'roomId');
  const name = optString(value, 'name');
  const avatarUrl = optString(value, 'avatarUrl');
  if (roomId === null || name === null || avatarUrl === null || !Array.isArray(value.children)) {
    return null;
  }
  const children: SpaceChild[] = [];
  for (const c of value.children) {
    const parsed = parseChild(c);
    if (!parsed) return null;
    children.push(parsed);
  }
  let parentRoomIds: string[] | undefined;
  if (value.parentRoomIds !== undefined) {
    const arr = stringArray(value.parentRoomIds);
    if (arr === null) return null;
    parentRoomIds = arr;
  }
  return { roomId, name, avatarUrl, children, parentRoomIds };
}
