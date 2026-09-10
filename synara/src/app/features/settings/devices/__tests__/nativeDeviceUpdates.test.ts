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
