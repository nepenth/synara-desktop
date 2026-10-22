import assert from 'node:assert/strict';
import test from 'node:test';
import { subscribeNativeDeviceUpdates } from '../nativeDevices';

test('native status signals reach subscribers and late registration is cleaned up', async () => {
  const previousWindow = globalThis.window;
  const calls: string[] = [];
  const generations: number[] = [];
  let handler: ((event: { payload: { sessionGeneration: number } }) => void) | undefined;
  let finishRegistration: ((id: number) => void) | undefined;
  const registration = new Promise<number>((resolve) => {
    finishRegistration = resolve;
  });
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      __TAURI_INTERNALS__: {
        transformCallback(callback: typeof handler) {
          handler = callback;
          return 1;
        },
        async invoke(command: string, args: { event: string }) {
          calls.push(command);
          assert.equal(args.event, 'matrix-device-list-updated');
          return command === 'plugin:event|listen' ? registration : undefined;
        },
      },
      __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener() {} },
    },
  });
  try {
    const unsubscribe = subscribeNativeDeviceUpdates((generation) => generations.push(generation));
    handler?.({ payload: { sessionGeneration: 42 } });
    assert.deepEqual(generations, [42]);
    // Unmount before the asynchronous native subscription resolves.
    unsubscribe();
    handler?.({ payload: { sessionGeneration: 43 } });
    assert.deepEqual(generations, [42], 'retired UI must not receive queued callbacks');
    finishRegistration?.(7);
    await new Promise<void>((resolve) => setImmediate(resolve));
    assert.deepEqual(calls, ['plugin:event|listen', 'plugin:event|unlisten']);
  } finally {
    if (previousWindow === undefined) Reflect.deleteProperty(globalThis, 'window');
    else Object.defineProperty(globalThis, 'window', { configurable: true, value: previousWindow });
  }
});

test('native status signal bursts coalesce into one trailing refresh per window', async () => {
  const previousWindow = globalThis.window;
  const generations: number[] = [];
  let handler: ((event: { payload: { sessionGeneration: number } }) => void) | undefined;
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      __TAURI_INTERNALS__: {
        transformCallback(callback: typeof handler) {
          handler = callback;
          return 1;
        },
        async invoke(command: string) {
          return command === 'plugin:event|listen' ? 1 : undefined;
        },
      },
      __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener() {} },
    },
  });
  const wait = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));
  try {
    const unsubscribe = subscribeNativeDeviceUpdates(
      (generation) => generations.push(generation),
      40
    );
    const emit = (generation: number) => handler?.({ payload: { sessionGeneration: generation } });
    emit(5);
    assert.deepEqual(generations, [5], 'the first signal refreshes at once');
    for (let index = 0; index < 50; index += 1) emit(5);
    assert.deepEqual(generations, [5], 'a burst waits for the window');
    await wait(80);
    assert.deepEqual(generations, [5, 5], 'the burst collapses into one trailing refresh');
    await wait(80);
    assert.deepEqual(generations, [5, 5], 'nothing pending, nothing delivered');

    emit(5);
    emit(6);
    assert.deepEqual(generations, [5, 5, 5, 6], 'a new session generation is never delayed');

    emit(6);
    unsubscribe();
    await wait(80);
    assert.deepEqual(generations, [5, 5, 5, 6], 'unsubscribe drops the pending refresh');
  } finally {
    if (previousWindow === undefined) Reflect.deleteProperty(globalThis, 'window');
    else Object.defineProperty(globalThis, 'window', { configurable: true, value: previousWindow });
  }
});
