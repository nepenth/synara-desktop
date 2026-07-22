import type { MatrixClient } from 'matrix-js-sdk';
import type { OwnDeviceKeys } from 'matrix-js-sdk/lib/crypto-api';

export type CryptoStoreContinuityFailureReason =
  | 'crypto-unavailable'
  | 'server-device-missing'
  | 'server-keys-missing'
  | 'identity-key-mismatch'
  | 'server-query-incomplete';

export class CryptoStoreContinuityError extends Error {
  public readonly name = 'CryptoStoreContinuityError';

  public constructor(
    public readonly userId: string,
    public readonly deviceId: string,
    public readonly reason: CryptoStoreContinuityFailureReason
  ) {
    super(
      `Crypto safety check blocked startup for device ${deviceId} (${reason}). ` +
        'The local crypto store was preserved. Do not sign out unless another trusted client can ' +
        'decrypt your history or you have tested your recovery key/key backup; signing out removes ' +
        "this device's local encryption data."
    );
  }
}

export type CryptoStoreContinuityResult = 'matched' | 'fresh-server-device';

export const canRetryCryptoStoreContinuityFailure = (error: CryptoStoreContinuityError): boolean =>
  error.reason === 'server-query-incomplete';

const serverKeysMatchLocalIdentity = (
  serverKeys: Record<string, string>,
  deviceId: string,
  localKeys: OwnDeviceKeys
): boolean =>
  serverKeys[`ed25519:${deviceId}`] === localKeys.ed25519 &&
  serverKeys[`curve25519:${deviceId}`] === localKeys.curve25519;

/**
 * Verify that the Rust crypto store still owns the keys registered for this
 * homeserver device. This intentionally uses MatrixClient.downloadKeysForUsers
 * so the answer comes from an authoritative /keys/query, not the local device
 * cache. It must run after initRustCrypto and before startClient.
 */
export const assertCryptoStoreContinuity = async (
  mx: MatrixClient,
  {
    userId,
    deviceId,
    allowMissingServerDevice = false,
  }: {
    userId: string;
    deviceId: string;
    allowMissingServerDevice?: boolean;
  }
): Promise<CryptoStoreContinuityResult> => {
  const crypto = mx.getCrypto();
  if (!crypto) {
    throw new CryptoStoreContinuityError(userId, deviceId, 'crypto-unavailable');
  }

  const localKeys = await crypto.getOwnDeviceKeys();
  let serverResult: Awaited<ReturnType<MatrixClient['downloadKeysForUsers']>>;
  try {
    serverResult = await mx.downloadKeysForUsers([userId]);
  } catch {
    throw new CryptoStoreContinuityError(userId, deviceId, 'server-query-incomplete');
  }

  if (Object.keys(serverResult.failures ?? {}).length > 0) {
    throw new CryptoStoreContinuityError(userId, deviceId, 'server-query-incomplete');
  }

  const serverDevice = serverResult.device_keys[userId]?.[deviceId];
  if (!serverDevice) {
    if (allowMissingServerDevice) return 'fresh-server-device';
    throw new CryptoStoreContinuityError(userId, deviceId, 'server-device-missing');
  }

  const serverEd25519 = serverDevice.keys?.[`ed25519:${deviceId}`];
  const serverCurve25519 = serverDevice.keys?.[`curve25519:${deviceId}`];
  if (!serverEd25519 || !serverCurve25519) {
    throw new CryptoStoreContinuityError(userId, deviceId, 'server-keys-missing');
  }

  if (!serverKeysMatchLocalIdentity(serverDevice.keys, deviceId, localKeys)) {
    throw new CryptoStoreContinuityError(userId, deviceId, 'identity-key-mismatch');
  }

  return 'matched';
};
