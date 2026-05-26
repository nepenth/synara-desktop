import test from 'node:test';
import assert from 'node:assert/strict';
import { createLocalStorageSessionStore, type SessionStorage } from '../sessions';

const createMemoryStorage = (initialValues: Record<string, string> = {}): SessionStorage => {
  const values = new Map(Object.entries(initialValues));

  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => {
      values.set(key, value);
    },
    removeItem: (key) => {
      values.delete(key);
    },
  };
};

test('local storage session store persists the fallback session shape', () => {
  const store = createLocalStorageSessionStore(createMemoryStorage());

  store.setFallbackSession({
    accessToken: 'token',
    deviceId: 'DEVICE',
    userId: '@alice:example.org',
    baseUrl: 'https://matrix.example.org',
  });

  assert.deepEqual(store.getFallbackSession(), {
    accessToken: 'token',
    deviceId: 'DEVICE',
    userId: '@alice:example.org',
    baseUrl: 'https://matrix.example.org',
    fallbackSdkStores: true,
  });
});

test('local storage session store returns undefined for incomplete fallback sessions', () => {
  const store = createLocalStorageSessionStore(
    createMemoryStorage({
      synara_access_token: 'token',
      synara_user_id: '@alice:example.org',
    })
  );

  assert.equal(store.getFallbackSession(), undefined);
});

test('local storage session store removes all fallback session fields', () => {
  const store = createLocalStorageSessionStore(createMemoryStorage());
  store.setFallbackSession({
    accessToken: 'token',
    deviceId: 'DEVICE',
    userId: '@alice:example.org',
    baseUrl: 'https://matrix.example.org',
  });

  store.removeFallbackSession();

  assert.equal(store.getFallbackSession(), undefined);
});
