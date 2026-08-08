import test, { mock } from 'node:test';
import assert from 'node:assert/strict';
// Local wire projections (js-sdk MatrixError/IRefreshTokenResponse/MatrixClient no longer imported):
// - matrix errors are duck-typed as Error & { errcode?: string } (see initMatrix.isMatrixErrorLike)
// - IRefreshTokenResponse.shape is a plain refresh response body
// - the live client is mocked with any
type RefreshResponse = { access_token: string; refresh_token: string; expires_in_ms: number };
import {
  REFRESH_BEFORE_EXPIRY_MS,
  applyRefreshedCredentialsToClient,
  createTokenRefreshFunction,
  refreshAndPersistSession,
  scheduleProactiveTokenRefresh,
  toAccessTokens,
  toRefreshedSession,
  type MatrixClientSession,
} from '../../../client/initMatrix';

const baseSession: MatrixClientSession = {
  baseUrl: 'https://matrix.example.org',
  accessToken: 'access-token',
  userId: '@alice:example.org',
  deviceId: 'ALICE_DEVICE',
  refreshToken: 'refresh-token',
  expiresInMs: 60_000,
  storedAtMs: 1_000,
};

const refreshResponse: RefreshResponse = {
  access_token: 'new-access-token',
  refresh_token: 'new-refresh-token',
  expires_in_ms: 120_000,
};

test('toRefreshedSession maps refresh response fields', () => {
  assert.deepEqual(toRefreshedSession(baseSession, refreshResponse), {
    baseUrl: baseSession.baseUrl,
    userId: baseSession.userId,
    deviceId: baseSession.deviceId,
    accessToken: 'new-access-token',
    refreshToken: 'new-refresh-token',
    expiresInMs: 120_000,
  });
});

test('toAccessTokens includes expiry date from expires_in_ms', () => {
  const before = Date.now();
  const tokens = toAccessTokens(refreshResponse);
  const after = Date.now();

  assert.equal(tokens.accessToken, 'new-access-token');
  assert.equal(tokens.refreshToken, 'new-refresh-token');
  assert.ok(tokens.expiry);
  assert.ok(tokens.expiry!.getTime() >= before + refreshResponse.expires_in_ms);
  assert.ok(tokens.expiry!.getTime() <= after + refreshResponse.expires_in_ms);
});

test('refreshAndPersistSession persists refreshed credentials', async () => {
  const persistCalls: MatrixClientSession[] = [];
  const swCalls: Array<{ baseUrl?: string; accessToken?: string }> = [];
  let accessToken = baseSession.accessToken;
  const http = { opts: { refreshToken: baseSession.refreshToken } };
  const mx = {
    refreshToken: async () => refreshResponse,
    setAccessToken: (token: string) => {
      accessToken = token;
    },
    http,
  } as unknown as any;

  const { tokens, session } = await refreshAndPersistSession(mx, baseSession, 'refresh-token', {
    persistAuthenticatedSession: async (persistedSession) => {
      persistCalls.push(persistedSession);
      return { session: persistedSession, source: 'native' };
    },
    pushSessionToSW: (baseUrl, swAccessToken) => {
      swCalls.push({ baseUrl, accessToken: swAccessToken });
    },
  });

  assert.equal(tokens.accessToken, 'new-access-token');
  assert.equal(accessToken, 'new-access-token');
  assert.equal(http.opts.refreshToken, 'new-refresh-token');
  assert.equal(typeof session.storedAtMs, 'number');
  assert.deepEqual(persistCalls[0], session);
  assert.deepEqual(swCalls, [{ baseUrl: baseSession.baseUrl, accessToken: 'new-access-token' }]);
});

test('applyRefreshedCredentialsToClient updates live matrix client tokens', () => {
  let accessToken = 'old-access-token';
  const http = { opts: { refreshToken: 'old-refresh-token' } };
  const mx = {
    setAccessToken(token: string) {
      accessToken = token;
    },
    http,
  } as unknown as any;

  applyRefreshedCredentialsToClient(mx, refreshResponse);

  assert.equal(accessToken, 'new-access-token');
  assert.equal(http.opts.refreshToken, 'new-refresh-token');
});

test('createTokenRefreshFunction rethrows MatrixError refresh failures', async () => {
  const matrixError = Object.assign(new Error('M_UNKNOWN_TOKEN'), { errcode: 'M_UNKNOWN_TOKEN' });
  const mx = {
    refreshToken: async () => {
      throw matrixError;
    },
  } as unknown as any;

  const refreshFn = createTokenRefreshFunction(() => mx, baseSession, {
    persistAuthenticatedSession: async (session) => ({ session, source: 'native' }),
    pushSessionToSW: () => undefined,
  });

  await assert.rejects(() => refreshFn('refresh-token'), matrixError);
});

test('scheduleProactiveTokenRefresh no-ops without expiry metadata', () => {
  const mx = {} as any;
  const handle = scheduleProactiveTokenRefresh(
    mx,
    { ...baseSession, expiresInMs: undefined },
    {},
    0
  );

  assert.equal(typeof handle.dispose, 'function');
});

test('scheduleProactiveTokenRefresh refreshes shortly before token expiry', async () => {
  const persistCalls: MatrixClientSession[] = [];
  const mx = {
    refreshToken: async () => refreshResponse,
    setAccessToken: () => undefined,
    http: { opts: { refreshToken: baseSession.refreshToken } },
  } as unknown as any;

  const now = baseSession.storedAtMs! + baseSession.expiresInMs! - REFRESH_BEFORE_EXPIRY_MS;
  const handle = scheduleProactiveTokenRefresh(
    mx,
    baseSession,
    {
      persistAuthenticatedSession: async (session) => {
        persistCalls.push(session);
        return { session, source: 'native' };
      },
      pushSessionToSW: () => undefined,
    },
    now
  );

  await new Promise((resolve) => setTimeout(resolve, 10));
  handle.dispose();

  assert.equal(persistCalls.length, 1);
  assert.equal(persistCalls[0]?.accessToken, 'new-access-token');
  assert.equal(persistCalls[0]?.refreshToken, 'new-refresh-token');
  assert.equal(typeof persistCalls[0]?.storedAtMs, 'number');
});

test('scheduleProactiveTokenRefresh schedules a follow-up refresh after rotation', async () => {
  mock.timers.enable({ apis: ['setTimeout', 'Date'] });

  try {
    let refreshCount = 0;
    const persistCalls: MatrixClientSession[] = [];
    const mx = {
      refreshToken: async () => {
        refreshCount += 1;
        return {
          access_token: `access-${refreshCount}`,
          refresh_token: `refresh-${refreshCount}`,
          expires_in_ms: refreshResponse.expires_in_ms,
        };
      },
      setAccessToken: () => undefined,
      http: { opts: { refreshToken: baseSession.refreshToken } },
    } as unknown as any;

    const initialNow =
      baseSession.storedAtMs! + baseSession.expiresInMs! - REFRESH_BEFORE_EXPIRY_MS;
    mock.timers.setTime(initialNow);

    const handle = scheduleProactiveTokenRefresh(
      mx,
      baseSession,
      {
        persistAuthenticatedSession: async (session) => {
          persistCalls.push(session);
          return { session, source: 'native' };
        },
        pushSessionToSW: () => undefined,
      },
      initialNow
    );

    mock.timers.runAll();
    await new Promise((resolve) => setImmediate(resolve));
    assert.equal(refreshCount, 1);
    assert.equal(persistCalls.length, 1);

    mock.timers.tick(REFRESH_BEFORE_EXPIRY_MS);
    mock.timers.runAll();
    await new Promise((resolve) => setImmediate(resolve));

    assert.equal(refreshCount, 2);
    assert.equal(persistCalls.length, 2);
    assert.equal(persistCalls[1]?.refreshToken, 'refresh-2');
    handle.dispose();
  } finally {
    mock.timers.reset();
  }
});
