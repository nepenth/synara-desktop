import test from 'node:test';
import assert from 'node:assert/strict';
import { pushActiveSessionToSW, pushSessionToSW } from '../../../sw-session';

type MockServiceWorker = {
  postMessage: (message: unknown) => void;
};

type MockNavigator = {
  serviceWorker?: {
    controller: MockServiceWorker | null;
  };
};

const withNavigator = async (
  navigatorValue: MockNavigator | undefined,
  run: () => void | Promise<void>
) => {
  const originalNavigator = globalThis.navigator;

  Object.defineProperty(globalThis, 'navigator', {
    configurable: true,
    value: navigatorValue,
  });

  try {
    await run();
  } finally {
    Object.defineProperty(globalThis, 'navigator', {
      configurable: true,
      value: originalNavigator,
    });
  }
};

test('pushSessionToSW is a no-op when navigator is unavailable', async () => {
  await withNavigator(undefined, () => {
    assert.doesNotThrow(() => pushSessionToSW('https://matrix.example.org', 'access-token'));
  });
});

test('pushSessionToSW is a no-op when service workers are unavailable', async () => {
  await withNavigator({}, () => {
    assert.doesNotThrow(() => pushSessionToSW('https://matrix.example.org', 'access-token'));
  });
});

test('pushSessionToSW is a no-op when no service worker controller is registered', async () => {
  const postMessages: unknown[] = [];

  await withNavigator(
    {
      serviceWorker: {
        controller: null,
      },
    },
    () => {
      pushSessionToSW('https://matrix.example.org', 'access-token');
    }
  );

  assert.equal(postMessages.length, 0);
});

test('pushSessionToSW posts session credentials to the active controller', async () => {
  const postMessages: unknown[] = [];
  const controller: MockServiceWorker = {
    postMessage: (message) => {
      postMessages.push(message);
    },
  };

  await withNavigator(
    {
      serviceWorker: {
        controller,
      },
    },
    () => {
      pushSessionToSW('https://matrix.example.org', 'access-token');
    }
  );

  assert.deepEqual(postMessages, [
    {
      type: 'setSession',
      accessToken: 'access-token',
      baseUrl: 'https://matrix.example.org',
    },
  ]);
});

test('pushActiveSessionToSW forwards the active session credentials', async () => {
  const postMessages: unknown[] = [];
  const controller: MockServiceWorker = {
    postMessage: (message) => {
      postMessages.push(message);
    },
  };

  await withNavigator(
    {
      serviceWorker: {
        controller,
      },
    },
    () => {
      pushActiveSessionToSW(() => ({
        baseUrl: 'https://matrix.example.org',
        accessToken: 'access-token',
      }));
    }
  );

  assert.deepEqual(postMessages, [
    {
      type: 'setSession',
      accessToken: 'access-token',
      baseUrl: 'https://matrix.example.org',
    },
  ]);
});
