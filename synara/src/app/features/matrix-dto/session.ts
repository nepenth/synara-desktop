/**
 * Session snapshot DTO — product projection of the live Matrix session.
 * Never includes accessToken / refreshToken.
 */

import type { DeviceId, UserId } from './ids';
import {
  hasForbiddenWireFields,
  isObject,
  optString,
  reqBoolean,
  reqNumber,
  reqString,
} from './parseUtil';

export const SESSION_LIFECYCLES = [
  'empty',
  'opening',
  'authenticating',
  'restoring',
  'syncing',
  'ready',
  'stopping',
  'logged_out',
  'failed',
  'wiping',
] as const;

export type SessionLifecycle = typeof SESSION_LIFECYCLES[number];

const LIFECYCLE_SET = new Set<string>(SESSION_LIFECYCLES);

export function isSessionLifecycle(value: unknown): value is SessionLifecycle {
  return typeof value === 'string' && LIFECYCLE_SET.has(value);
}

export type SessionSnapshot = {
  sessionGeneration: number;
  userId: UserId;
  deviceId: DeviceId;
  homeserverUrl: string;
  displayName?: string;
  /** mxc or product media-handle URI — string only. */
  avatarUrl?: string;
  lifecycle: SessionLifecycle;
  cryptoReady: boolean;
};

export function parseSessionSnapshot(value: unknown): SessionSnapshot | null {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const sessionGeneration = reqNumber(value, 'sessionGeneration');
  const userId = reqString(value, 'userId');
  const deviceId = reqString(value, 'deviceId');
  const homeserverUrl = reqString(value, 'homeserverUrl');
  const displayName = optString(value, 'displayName');
  const avatarUrl = optString(value, 'avatarUrl');
  const cryptoReady = reqBoolean(value, 'cryptoReady');
  if (
    sessionGeneration === null ||
    userId === null ||
    deviceId === null ||
    homeserverUrl === null ||
    displayName === null ||
    avatarUrl === null ||
    cryptoReady === null ||
    !isSessionLifecycle(value.lifecycle)
  ) {
    return null;
  }
  return {
    sessionGeneration,
    userId,
    deviceId,
    homeserverUrl,
    displayName,
    avatarUrl,
    lifecycle: value.lifecycle,
    cryptoReady,
  };
}
