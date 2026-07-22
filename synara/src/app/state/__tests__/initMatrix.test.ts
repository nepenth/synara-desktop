import test from 'node:test';
import assert from 'node:assert/strict';
import type { MatrixClient } from 'matrix-js-sdk';
import {
  confirmFreshLoginCryptoContinuity,
  initClient,
  startClient,
} from '../../../client/initMatrix';
import {
  assertCryptoStoreContinuity,
  canRetryCryptoStoreContinuityFailure,
  CryptoStoreContinuityError,
} from '../../../client/cryptoStoreContinuity';

const session = {
  baseUrl: 'https://matrix.example.org',
  accessToken: 'access-token',
  userId: '@alice:example.org',
  deviceId: 'ALICE_DEVICE',
  sessionGeneration: 'generation-1',
};

const createMockMatrixClient = (): MatrixClient => ({} as MatrixClient);

test('initClient clears matrix stores before init when bootstrapped identity differs', async () => {
  const calls: string[] = [];

  await initClient(session, {
    isPendingFreshLoginIdentity: () => true,
    clearMatrixStoresForIdentityChange: async () => {
      calls.push('clear-for-identity');
      return true;
    },
    startMatrixClient: async () => {
      calls.push('start');
      return createMockMatrixClient();
    },
    setLastBootstrappedMatrixIdentity: () => {
      calls.push('record-identity');
    },
  });

  assert.deepEqual(calls, ['clear-for-identity', 'start', 'record-identity']);
});

test('initClient preserves stores and does not retry a restored crypto account mismatch', async () => {
  const mismatchError = new Error(
    "the account in the store doesn't match the account in the constructor: expected @bob:example.org:OLD, got @alice:example.org:ALICE_DEVICE"
  );
  let startAttempts = 0;

  await assert.rejects(
    () =>
      initClient(session, {
        clearMatrixStoresForIdentityChange: async () => {
          throw new Error('restored sessions must not clear stores');
        },
        isPendingFreshLoginIdentity: () => false,
        startMatrixClient: async () => {
          startAttempts += 1;
          throw mismatchError;
        },
        setLastBootstrappedMatrixIdentity: () => undefined,
      }),
    (error: unknown) =>
      error instanceof CryptoStoreContinuityError && error.reason === 'identity-key-mismatch'
  );

  assert.equal(startAttempts, 1);
});

test('initClient records bootstrapped identity after successful init', async () => {
  let recordedIdentity: { userId: string; deviceId: string } | undefined;

  await initClient(session, {
    clearMatrixStoresForIdentityChange: async () => false,
    startMatrixClient: async () => createMockMatrixClient(),
    setLastBootstrappedMatrixIdentity: (identity) => {
      recordedIdentity = identity;
    },
  });

  assert.deepEqual(recordedIdentity, {
    userId: '@alice:example.org',
    deviceId: 'ALICE_DEVICE',
  });
});

test('initClient rethrows unrelated startup failures', async () => {
  const startupError = new Error('network timeout');

  await assert.rejects(
    () =>
      initClient(session, {
        clearMatrixStoresForIdentityChange: async () => false,
        startMatrixClient: async () => {
          throw startupError;
        },
      }),
    startupError
  );
});

test('initClient only allows a missing server device for an explicitly fresh login', async () => {
  const options: Array<boolean | undefined> = [];

  await initClient(session, {
    isPendingFreshLoginIdentity: () => true,
    clearMatrixStoresForIdentityChange: async () => false,
    startMatrixClient: async (_session, startOptions) => {
      options.push(startOptions?.allowMissingServerDevice);
      return createMockMatrixClient();
    },
  });
  await initClient(session, {
    isPendingFreshLoginIdentity: () => false,
    clearMatrixStoresForIdentityChange: async () => {
      throw new Error('restored sessions must not clear stores');
    },
    startMatrixClient: async (_session, startOptions) => {
      options.push(startOptions?.allowMissingServerDevice);
      return createMockMatrixClient();
    },
  });

  assert.deepEqual(options, [true, false]);
});

const createContinuityClient = ({
  localEd25519 = 'local-ed25519',
  localCurve25519 = 'local-curve25519',
  serverDevice,
}: {
  localEd25519?: string;
  localCurve25519?: string;
  serverDevice?: { keys: Record<string, string> };
}) => {
  const calls: string[] = [];
  const mx = {
    getCrypto: () => ({
      getOwnDeviceKeys: async () => {
        calls.push('get-local-keys');
        return { ed25519: localEd25519, curve25519: localCurve25519 };
      },
    }),
    downloadKeysForUsers: async (userIds: string[]) => {
      calls.push(`keys-query:${userIds.join(',')}`);
      return {
        failures: {},
        device_keys: {
          '@alice:example.org': serverDevice ? { ALICE_DEVICE: serverDevice } : {},
        },
      };
    },
  } as unknown as MatrixClient;
  return { mx, calls };
};

test('crypto continuity accepts matching authoritative Ed25519 and Curve25519 keys', async () => {
  const { mx, calls } = createContinuityClient({
    serverDevice: {
      keys: {
        'ed25519:ALICE_DEVICE': 'local-ed25519',
        'curve25519:ALICE_DEVICE': 'local-curve25519',
      },
    },
  });

  assert.equal(await assertCryptoStoreContinuity(mx, session), 'matched');
  assert.deepEqual(calls, ['get-local-keys', 'keys-query:@alice:example.org']);
});

test('crypto continuity rejects mismatched server keys without upload or deletion', async () => {
  const { mx, calls } = createContinuityClient({
    serverDevice: {
      keys: {
        'ed25519:ALICE_DEVICE': 'different-ed25519',
        'curve25519:ALICE_DEVICE': 'different-curve25519',
      },
    },
  });

  await assert.rejects(
    () => assertCryptoStoreContinuity(mx, session),
    (error: unknown) =>
      error instanceof CryptoStoreContinuityError && error.reason === 'identity-key-mismatch'
  );
  assert.deepEqual(calls, ['get-local-keys', 'keys-query:@alice:example.org']);
});

test('crypto continuity normalizes authoritative query transport failures', async () => {
  const mx = {
    getCrypto: () => ({
      getOwnDeviceKeys: async () => ({
        ed25519: 'local-ed25519',
        curve25519: 'local-curve25519',
      }),
    }),
    downloadKeysForUsers: async () => {
      throw new Error('network unavailable');
    },
  } as unknown as MatrixClient;

  await assert.rejects(
    () => assertCryptoStoreContinuity(mx, session),
    (error: unknown) =>
      error instanceof CryptoStoreContinuityError && error.reason === 'server-query-incomplete'
  );
});

test('only transient continuity query failures offer retry', () => {
  assert.equal(
    canRetryCryptoStoreContinuityFailure(
      new CryptoStoreContinuityError(session.userId, session.deviceId, 'server-query-incomplete')
    ),
    true
  );
  assert.equal(
    canRetryCryptoStoreContinuityFailure(
      new CryptoStoreContinuityError(session.userId, session.deviceId, 'identity-key-mismatch')
    ),
    false
  );
});

test('crypto continuity permits a missing server device only for a fresh login', async () => {
  const fresh = createContinuityClient({});
  const restored = createContinuityClient({});

  assert.equal(
    await assertCryptoStoreContinuity(fresh.mx, {
      ...session,
      allowMissingServerDevice: true,
    }),
    'fresh-server-device'
  );
  await assert.rejects(
    () => assertCryptoStoreContinuity(restored.mx, session),
    (error: unknown) =>
      error instanceof CryptoStoreContinuityError && error.reason === 'server-device-missing'
  );
});

test('post-start continuity retries missing keys and clears bootstrap only after a match', async () => {
  const mx = createMockMatrixClient();
  const reasons: Array<boolean | undefined> = [];
  const cleared: string[] = [];
  let attempts = 0;

  await confirmFreshLoginCryptoContinuity(mx, session, {
    retryDelaysMs: [0, 0],
    assertContinuity: async (_mx, options) => {
      attempts += 1;
      reasons.push(options.allowMissingServerDevice);
      if (attempts === 1) {
        throw new CryptoStoreContinuityError(
          session.userId,
          session.deviceId,
          'server-device-missing'
        );
      }
      return 'matched';
    },
    clearPendingIdentity: (identity) => {
      cleared.push(identity.sessionGeneration ?? 'missing');
    },
  });

  assert.equal(attempts, 2);
  assert.deepEqual(reasons, [false, false]);
  assert.deepEqual(cleared, ['generation-1']);
});

test('post-start continuity never clears bootstrap when authoritative keys do not match', async () => {
  let cleared = false;
  let attempts = 0;

  await assert.rejects(
    () =>
      confirmFreshLoginCryptoContinuity(createMockMatrixClient(), session, {
        retryDelaysMs: [0, 0],
        assertContinuity: async () => {
          attempts += 1;
          throw new CryptoStoreContinuityError(
            session.userId,
            session.deviceId,
            'identity-key-mismatch'
          );
        },
        clearPendingIdentity: () => {
          cleared = true;
        },
      }),
    (error: unknown) =>
      error instanceof CryptoStoreContinuityError && error.reason === 'identity-key-mismatch'
  );

  assert.equal(attempts, 1);
  assert.equal(cleared, false);
});

test('post-start continuity stops the running client and normalizes raw failures', async () => {
  const calls: string[] = [];
  const mx = {
    startClient: async () => {
      calls.push('start');
    },
    stopClient: () => {
      calls.push('stop');
    },
  } as unknown as MatrixClient;

  await assert.rejects(
    () =>
      startClient(mx, {
        pendingSession: session,
        waitForPrepared: async () => undefined,
        confirmContinuity: async () => {
          throw new Error('query connection reset');
        },
      }),
    (error: unknown) =>
      error instanceof CryptoStoreContinuityError && error.reason === 'server-query-incomplete'
  );

  assert.deepEqual(calls, ['start', 'stop']);
});
