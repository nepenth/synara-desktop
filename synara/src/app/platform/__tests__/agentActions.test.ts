import { readFileSync } from 'node:fs';
import test from 'node:test';
import assert from 'node:assert/strict';
import {
  PLATFORM_AGENT_ACTION_EVENT,
  executePlatformAgentAction,
  handleIncomingPlatformAgentAction,
  parseIncomingPlatformAgentAction,
  registerPlatformAgentActionListener,
  sendPlatformAgentAction,
  withAgentActionInvocationId,
} from '../agentActions';
import { MAX_AGENT_ACTION_TEXT_LENGTH } from '../../agents/agentActions';

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

test('executePlatformAgentAction does not fall back to window.open on rejected desktop URLs', async () => {
  const browserOpenedUrls: string[] = [];
  const originalWindow = globalThis.window;

  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      invoke: async () => false,
    },
    open: (url: string) => {
      browserOpenedUrls.push(url);
      return {};
    },
  };

  try {
    const action = parseIncomingPlatformAgentAction({
      action: {
        id: 'unsafe',
        title: 'Unsafe',
        kind: 'open_url',
        url: 'https://agent.example.org/runs/1',
      },
    });
    assert.ok(action);
    assert.equal(await executePlatformAgentAction(action!), false);
  } finally {
    (globalThis as any).window = originalWindow;
  }

  assert.deepEqual(browserOpenedUrls, []);
});

test('a repeated click or a second card with the same action id runs again', async () => {
  const originalWindow = globalThis.window;
  const originalNavigator = globalThis.navigator;
  const copied: string[] = [];
  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      // The shell hands workflow actions back to the renderer listener.
      invoke: async (command: string, args?: Record<string, unknown>) => {
        assert.equal(command, 'desktop_agent_action');
        await handleIncomingPlatformAgentAction({ action: args?.action });
        return true;
      },
    },
  };
  Object.defineProperty(globalThis, 'navigator', {
    configurable: true,
    value: { clipboard: { writeText: async (text: string) => copied.push(text) } },
  });

  try {
    // Hermes cards default to `action-0`, so ids repeat across cards.
    const action = { id: 'action-0', title: 'Run', kind: 'run', prompt: 'npm test' };
    assert.equal(await sendPlatformAgentAction(action), true);
    assert.equal(await sendPlatformAgentAction(action), true);
    assert.equal(await sendPlatformAgentAction({ ...action, prompt: 'npm run lint' }), true);
    assert.deepEqual(copied, ['npm test', 'npm test', 'npm run lint']);
  } finally {
    (globalThis as any).window = originalWindow;
    Object.defineProperty(globalThis, 'navigator', {
      configurable: true,
      value: originalNavigator,
    });
  }
});

test('one invocation delivered twice still runs once', async () => {
  const originalWindow = globalThis.window;
  const opened: string[] = [];
  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      invoke: async (command: string, args?: Record<string, unknown>) => {
        if (command === 'desktop_open_external_url') opened.push(String(args?.url));
        return true;
      },
    },
  };
  try {
    const envelope = {
      action: withAgentActionInvocationId(
        { id: 'open', title: 'Open', kind: 'open', url: 'https://agent.example.org/run' },
        'nonce-1'
      ),
    };
    assert.equal(await handleIncomingPlatformAgentAction(envelope), true);
    assert.equal(await handleIncomingPlatformAgentAction(envelope), false);
    assert.deepEqual(opened, ['https://agent.example.org/run']);
  } finally {
    (globalThis as any).window = originalWindow;
  }
});

test('invocation ids stay within the payload limit and never validate a missing id', () => {
  const long = withAgentActionInvocationId(
    { id: 'x'.repeat(MAX_AGENT_ACTION_TEXT_LENGTH), title: 'Run', prompt: 'p' },
    'abc'
  );
  assert.equal(long.id.length, MAX_AGENT_ACTION_TEXT_LENGTH);
  assert.ok(long.id.endsWith('#abc'));
  assert.ok(parseIncomingPlatformAgentAction({ action: long }));
  assert.equal(
    withAgentActionInvocationId({ id: '  ', title: 'Run', prompt: 'p' }, 'abc').id,
    '  '
  );
});

test('an action no plan can run reports unhandled so the card falls back', async () => {
  const originalWindow = globalThis.window;
  let invoked = 0;
  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      invoke: async () => {
        invoked += 1;
        return true;
      },
    },
  };
  try {
    assert.equal(
      await sendPlatformAgentAction({ id: 'open', title: 'Open', kind: 'open', prompt: 'p' }),
      false
    );
    assert.equal(invoked, 0);
  } finally {
    (globalThis as any).window = originalWindow;
  }
});

test('a Hermes card confirms a copied action instead of looking ignored', () => {
  const card = readFileSync('src/app/components/hermes/HermesAgentCard.tsx', 'utf8');
  assert.match(card, /setActionDoneId\(action\.id\)/);
  assert.match(card, /role="status"/);
  assert.match(card, /'modernization\.hermes\.action_copied', 'Copied'/);
});
