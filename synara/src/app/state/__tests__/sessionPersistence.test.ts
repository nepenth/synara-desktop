import test from 'node:test';
import assert from 'node:assert/strict';
import {
  clearPendingFreshLoginIdentity,
  clearPersistedSessions,
  createFreshLoginSessionGeneration,
  FRESH_LOGIN_BOOTSTRAP_TTL_MS,
  getPendingFreshLoginIdentity,
  isPendingFreshLoginIdentity,
  markPendingFreshLoginIdentity,
} from '../sessionPersistence';
import { getSessionBootstrapResult, resetSessionBootstrapForTests } from '../sessionBootstrap';
import type { SessionStorage } from '../sessions';

const createMemoryStorage = (initial: Record<string, string> = {}): SessionStorage => {
  const values = new Map(Object.entries(initial));
  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
    removeItem: (key) => values.delete(key),
  };
};

test('fresh login generation is non-empty and distinct', () => {
  const first = createFreshLoginSessionGeneration();
  const second = createFreshLoginSessionGeneration();
  assert.ok(first.length >= 16);
  assert.notEqual(first, second);
});

test('fresh login marker is identity-scoped and expires closed', () => {
  const storage = createMemoryStorage();
  const identity = {
    userId: '@alice:example.org',
    deviceId: 'DEVICE',
    baseUrl: 'https://matrix.example.org',
    sessionGeneration: 'generation-1',
  };
  markPendingFreshLoginIdentity(identity, storage, 1_000);
  assert.equal(isPendingFreshLoginIdentity(identity, storage, 1_000), true);
  assert.equal(
    isPendingFreshLoginIdentity({ ...identity, deviceId: 'OTHER' }, storage, 1_000),
    false
  );
  assert.equal(
    getPendingFreshLoginIdentity(storage, 1_000 + FRESH_LOGIN_BOOTSTRAP_TTL_MS + 1),
    undefined
  );
});

test('fresh login marker clears only for the exact identity', () => {
  const storage = createMemoryStorage();
  const identity = {
    userId: '@alice:example.org',
    deviceId: 'DEVICE',
    baseUrl: 'https://matrix.example.org',
    sessionGeneration: 'generation-1',
  };
  markPendingFreshLoginIdentity(identity, storage, 1_000);
  clearPendingFreshLoginIdentity({ ...identity, deviceId: 'OTHER' }, storage, 1_000);
  assert.notEqual(getPendingFreshLoginIdentity(storage, 1_000), undefined);
  clearPendingFreshLoginIdentity(identity, storage, 1_000);
  assert.equal(getPendingFreshLoginIdentity(storage, 1_000), undefined);
});

test('clearPersistedSessions clears renderer bootstrap without credential writes', async () => {
  resetSessionBootstrapForTests();
  await clearPersistedSessions();
  assert.deepEqual(getSessionBootstrapResult(), {
    session: undefined,
    source: 'none',
    nativeStoreError: undefined,
  });
});
