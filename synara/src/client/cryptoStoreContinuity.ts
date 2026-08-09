/**
 * D1C — renderer crypto-continuity surface.
 *
 * Under the native cutover (Option A + D1C) the renderer no longer owns crypto
 * state, so it has no local device keys to reconcile with the server: the
 * native layer owns the crypto store, keys, and continuity. This module keeps
 * only the error type + retry-policy helper that the boot UI references; the
 * native path never constructs these errors, so the safety screen stays inert.
 */

export type ContinuityDeviceKeys = { ed25519: string; curve25519: string };

/** Duck-typed js-sdk MatrixError with an `errcode` (kept for UI classification). */
export const isMatrixErrorLike = (error: unknown): error is Error & { errcode?: string } =>
  error instanceof Error && typeof (error as { errcode?: unknown } | null)?.errcode === 'string';

export type CryptoStoreContinuityReason =
  | 'identity-key-mismatch'
  | 'server-device-missing'
  | 'server-query-incomplete'
  | 'crypto-unavailable';

export class CryptoStoreContinuityError extends Error {
  readonly userId: string;
  readonly deviceId: string;
  readonly reason: CryptoStoreContinuityReason;

  constructor(userId: string, deviceId: string, reason: CryptoStoreContinuityReason) {
    super(`Crypto store continuity failed for ${userId}/${deviceId}: ${reason}`);
    this.name = 'CryptoStoreContinuityError';
    this.userId = userId;
    this.deviceId = deviceId;
    this.reason = reason;
  }
}

/**
 * D1C: the renderer never runs a crypto-store continuity check (native owns the
 * store), so no continuity error can be retried from the renderer.
 */
export const canRetryCryptoStoreContinuityFailure = (_error: CryptoStoreContinuityError): boolean =>
  false;
