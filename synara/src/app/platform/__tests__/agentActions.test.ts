import test from 'node:test';
import assert from 'node:assert/strict';
import {
  PLATFORM_AGENT_ACTION_EVENT,
  executePlatformAgentAction,
  handleIncomingPlatformAgentAction,
  parseIncomingPlatformAgentAction,
  registerPlatformAgentActionListener,
} from '../agentActions';

test('registerPlatformAgentActionListener registers synara://agent-action via listen', async () => {
  const registrations: Array<{ event: string; handler: (event: { payload: unknown }) => void }> =
    [];
  const originalWindow = globalThis.window;
  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
    },
    __TAURI_INTERNALS__: {
      invoke: async (command: string, args?: Record<string, unknown>) => {
        if (command !== 'plugin:event|listen') {
          throw new Error(`unexpected invoke command: ${command}`);
        }
        const handler = args?.handler as number;
        registrations.push({
          event: String(args?.event),
          handler: (globalThis as any).__handlers[handler],
        });
        return 7;
      },
      transformCallback: (callback: (event: { payload: unknown }) => void) => {
        const handlers = ((globalThis as any).__handlers ??= {});
        const id = Object.keys(handlers).length + 1;
        handlers[id] = callback;
        return id;
      },
    },
  };

  try {
    const unlisten = await registerPlatformAgentActionListener();
    assert.equal(registrations.length, 1);
    assert.equal(registrations[0]?.event, PLATFORM_AGENT_ACTION_EVENT);
    assert.equal(PLATFORM_AGENT_ACTION_EVENT, 'synara://agent-action');
    assert.equal(typeof unlisten, 'function');
  } finally {
    (globalThis as any).window = originalWindow;
  }
});

test('handleIncomingPlatformAgentAction executes valid payloads and rejects invalid payloads', async () => {
  const openedUrls: string[] = [];
  const originalWindow = globalThis.window;

  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      invoke: async (command: string, args?: Record<string, unknown>) => {
        if (command === 'desktop_open_external_url') {
          openedUrls.push(String(args?.url));
          return true;
        }
        return false;
      },
    },
    open: () => null,
  };

  try {
    assert.equal(
      await handleIncomingPlatformAgentAction({
        action: {
          id: 'regenerate',
          title: 'Regenerate',
          kind: 'regenerate',
          url: 'https://agent.example.org/runs/1/regenerate',
        },
      }),
      true
    );
    assert.deepEqual(openedUrls, ['https://agent.example.org/runs/1/regenerate']);

    assert.equal(
      await handleIncomingPlatformAgentAction({
        action: {
          id: 'regenerate',
          title: 'Regenerate',
          kind: 'regenerate',
          url: 'https://agent.example.org/runs/1/regenerate',
        },
      }),
      false
    );

    assert.equal(
      await handleIncomingPlatformAgentAction({ action: { title: 'Missing id' } }),
      false
    );
    assert.equal(
      await handleIncomingPlatformAgentAction({
        action: {
          id: 'unsafe',
          title: 'Unsafe',
          url: 'http://example.org',
        },
      }),
      false
    );
    assert.equal(
      await handleIncomingPlatformAgentAction({
        action: {
          id: 'shell',
          title: 'Shell',
          kind: 'shell',
          prompt: 'rm -rf /',
        },
      }),
      false
    );
  } finally {
    (globalThis as any).window = originalWindow;
  }
});

test('executePlatformAgentAction opens safe workflow URLs', async () => {
  const openedUrls: string[] = [];
  const originalWindow = globalThis.window;

  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      invoke: async (command: string, args?: Record<string, unknown>) => {
        if (command === 'desktop_open_external_url') {
          openedUrls.push(String(args?.url));
          return true;
        }
        return false;
      },
    },
    open: () => null,
  };

  try {
    const action = parseIncomingPlatformAgentAction({
      action: {
        id: 'regenerate',
        title: 'Regenerate',
        kind: 'regenerate',
        url: 'https://agent.example.org/runs/1/regenerate',
      },
    });
    assert.ok(action);
    assert.equal(await executePlatformAgentAction(action!), true);
    assert.deepEqual(openedUrls, ['https://agent.example.org/runs/1/regenerate']);
  } finally {
    (globalThis as any).window = originalWindow;
  }
});
