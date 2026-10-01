import test from 'node:test';
import assert from 'node:assert/strict';
import { clearLoginData } from '../../../client/initMatrix';
import { SESSION_LOCAL_STORAGE_EXACT_KEYS, type SessionLocalStorage } from '../sessions';

const createEnumeratedMemoryStorage = (
  initialValues: Record<string, string> = {}
): SessionLocalStorage => {
  const values = new Map(Object.entries(initialValues));

  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => {
      values.set(key, value);
    },
    removeItem: (key) => {
      values.delete(key);
    },
    get length() {
      return values.size;
    },
    key: (index) => Array.from(values.keys())[index] ?? null,
  };
};

const withWindow = async (
  windowValue: {
    localStorage: SessionLocalStorage;
    location: { reload: () => void };
    __SYNARA_DESKTOP__?: { invoke: (command: string) => Promise<unknown> };
  },
  run: () => void | Promise<void>
) => {
  const originalWindow = globalThis.window;

  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: windowValue,
  });

  try {
    await run();
  } finally {
    Object.defineProperty(globalThis, 'window', {
      configurable: true,
      value: originalWindow,
    });
  }
};

test('clearLoginData removes session keys only', async () => {
  const storage = createEnumeratedMemoryStorage({
    synara_access_token: 'token',
    synara_device_id: 'DEVICE',
    synara_user_id: '@alice:example.org',
    synara_hs_base_url: 'https://matrix.example.org',
    after_login_redirect_url: '/room/123',
    'navToActivePath@alice:example.org': '{"home":{"pathname":"/home"}}',
    settings: JSON.stringify({ themeId: 'aurora', pageZoom: 120 }),
    platformSettings: JSON.stringify({ desktopShortcutShow: 'CmdOrCtrl+1' }),
  });
  let reloaded = false;

  await withWindow(
    {
      localStorage: storage,
      __SYNARA_DESKTOP__: {
        invoke: async (command) => {
          assert.equal(command, 'matrix_logout');
          return { status: 'logged_out' };
        },
      },
      location: {
        reload: () => {
          reloaded = true;
        },
      },
    },
    async () => {
      await clearLoginData(storage);
    }
  );

  SESSION_LOCAL_STORAGE_EXACT_KEYS.forEach((key) => {
    assert.equal(storage.getItem(key), null, `expected session key ${key} to be removed`);
  });
  assert.equal(storage.getItem('navToActivePath@alice:example.org'), null);
  assert.equal(storage.getItem('settings'), JSON.stringify({ themeId: 'aurora', pageZoom: 120 }));
  assert.equal(
    storage.getItem('platformSettings'),
    JSON.stringify({ desktopShortcutShow: 'CmdOrCtrl+1' })
  );
  assert.equal(reloaded, true);
});

test('clearLoginData rejects when the native owner is unavailable without reloading', async () => {
  const storage = createEnumeratedMemoryStorage({ after_login_redirect_url: '/home' });
  let reloaded = false;
  await withWindow(
    {
      localStorage: storage,
      location: {
        reload: () => {
          reloaded = true;
        },
      },
    },
    async () => {
      await assert.rejects(clearLoginData(storage), /Native logout did not complete/);
    }
  );
  assert.equal(storage.getItem('after_login_redirect_url'), '/home');
  assert.equal(reloaded, false);
});
