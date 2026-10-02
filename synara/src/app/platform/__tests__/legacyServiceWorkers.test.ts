import assert from 'node:assert/strict';
import { test } from 'node:test';
import { retireLegacyServiceWorkers } from '../legacyServiceWorkers';

test('retires only Synara workers at their original scope, including waiting installs', async () => {
  const removed: string[] = [];
  const entry = (name: string, scriptURL: string, scope = 'https://app.example/synara/') => ({
    scope,
    active: { scriptURL },
    waiting: null,
    installing: null,
    unregister: async () => {
      removed.push(name);
      return true;
    },
  });
  const waiting = entry('waiting', 'https://app.example/synara/sw.js');
  const registrations = [
    entry('production', 'https://app.example/synara/sw.js'),
    entry('development', 'https://app.example/dev-sw.js?dev-sw', 'https://app.example/'),
    entry('other-app', 'https://app.example/other/sw.js', 'https://app.example/other/'),
    entry('different-scope', 'https://app.example/synara/sw.js', 'https://app.example/'),
    entry('different-origin', 'https://other.example/synara/sw.js'),
    { ...waiting, active: null, waiting: waiting.active },
    {
      ...entry('replaced', 'https://app.example/synara/sw.js'),
      waiting: { scriptURL: 'https://app.example/new-worker.js' },
    },
  ];
  await retireLegacyServiceWorkers(
    { serviceWorker: { getRegistrations: async () => registrations } },
    '/synara/',
    'https://app.example/synara/index.html'
  );
  assert.deepEqual(removed, ['production', 'development', 'waiting']);
});

test('worker API and individual unregister failures are contained', async () => {
  await retireLegacyServiceWorkers({}, '/', 'https://app.example/');
  await retireLegacyServiceWorkers(
    {
      serviceWorker: {
        getRegistrations: async () => {
          throw new Error('disabled');
        },
      },
    },
    '/',
    'https://app.example/'
  );
  let attempted = 0;
  await retireLegacyServiceWorkers(
    {
      serviceWorker: {
        getRegistrations: async () => [
          {
            scope: 'https://app.example/',
            active: { scriptURL: 'https://app.example/sw.js' },
            waiting: null,
            installing: null,
            unregister: async () => {
              attempted += 1;
              throw new Error('blocked');
            },
          },
          {
            scope: 'https://app.example/',
            active: { scriptURL: 'https://app.example/sw.js' },
            waiting: null,
            installing: null,
            unregister: async () => {
              attempted += 1;
              return true;
            },
          },
        ],
      },
    },
    '/',
    'https://app.example/'
  );
  assert.equal(attempted, 2);
});
