import test from 'node:test';
import assert from 'node:assert/strict';
import {
  clearSessionExpiryNotice,
  hasSessionExpiryNotice,
  recordSessionExpiry,
} from '../sessionExpiry';

test('expiry explanation survives reload, clears after fresh login, and tolerates unavailable storage', () => {
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, 'sessionStorage');
  const values = new Map<string, string>();
  try {
    Object.defineProperty(globalThis, 'sessionStorage', {
      configurable: true,
      value: {
        getItem: (key: string) => values.get(key) ?? null,
        setItem: (key: string, value: string) => values.set(key, value),
        removeItem: (key: string) => values.delete(key),
      },
    });
    assert.equal(hasSessionExpiryNotice(), false);
    recordSessionExpiry();
    assert.equal(hasSessionExpiryNotice(), true);
    assert.deepEqual([...values.values()], ['true']);
    clearSessionExpiryNotice();
    assert.equal(hasSessionExpiryNotice(), false);
    Object.defineProperty(globalThis, 'sessionStorage', {
      configurable: true,
      get: () => {
        throw new Error('test storage blocked');
      },
    });
    assert.doesNotThrow(recordSessionExpiry);
    assert.doesNotThrow(clearSessionExpiryNotice);
    assert.equal(hasSessionExpiryNotice(), false);
  } finally {
    if (descriptor) Object.defineProperty(globalThis, 'sessionStorage', descriptor);
    else Reflect.deleteProperty(globalThis, 'sessionStorage');
  }
});
