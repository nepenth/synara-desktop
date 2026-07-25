/**
 * Security / crypto status projection — no keys or recovery material.
 */

import {
  hasForbiddenWireFields,
  isObject,
  optNumber,
  reqBoolean,
} from './parseUtil';

export const BACKUP_STATUSES = ['unknown', 'disabled', 'enabled', 'outdated'] as const;
export type BackupStatus = (typeof BACKUP_STATUSES)[number];
const BACKUP_SET = new Set<string>(BACKUP_STATUSES);

export const RECOVERY_STATUSES = [
  'unknown',
  'not_setup',
  'ready',
  'incomplete',
] as const;
export type RecoveryStatus = (typeof RECOVERY_STATUSES)[number];
const RECOVERY_SET = new Set<string>(RECOVERY_STATUSES);

export const VERIFICATION_STATES = [
  'unverified',
  'verified',
  'unavailable',
] as const;
export type VerificationState = (typeof VERIFICATION_STATES)[number];
const VERIFICATION_SET = new Set<string>(VERIFICATION_STATES);

export type SecurityStatus = {
  crossSigningActive: boolean;
  backupStatus: BackupStatus;
  recoveryStatus: RecoveryStatus;
  verificationState: VerificationState;
  deviceCount?: number;
  hasPendingVerificationRequests: boolean;
};

export function parseSecurityStatus(value: unknown): SecurityStatus | null {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const crossSigningActive = reqBoolean(value, 'crossSigningActive');
  const hasPendingVerificationRequests = reqBoolean(
    value,
    'hasPendingVerificationRequests'
  );
  const deviceCount = optNumber(value, 'deviceCount');
  if (
    crossSigningActive === null ||
    hasPendingVerificationRequests === null ||
    deviceCount === null ||
    typeof value.backupStatus !== 'string' ||
    !BACKUP_SET.has(value.backupStatus) ||
    typeof value.recoveryStatus !== 'string' ||
    !RECOVERY_SET.has(value.recoveryStatus) ||
    typeof value.verificationState !== 'string' ||
    !VERIFICATION_SET.has(value.verificationState)
  ) {
    return null;
  }
  return {
    crossSigningActive,
    backupStatus: value.backupStatus as BackupStatus,
    recoveryStatus: value.recoveryStatus as RecoveryStatus,
    verificationState: value.verificationState as VerificationState,
    deviceCount,
    hasPendingVerificationRequests,
  };
}
