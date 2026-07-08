import test from 'node:test';
import assert from 'node:assert/strict';
import type { MouseEvent as ReactMouseEvent } from 'react';
import {
  DESKTOP_EXTERNAL_LINK_CLICK_OPTIONS,
  openDesktopExternalAnchorFromClick,
  openExternalUrl,
  openExternalUrlFromClick,
} from '../appLinks';

const waitForAsyncOpen = () => new Promise((resolve) => setTimeout(resolve, 0));

test('desktop link interceptor uses capture phase', () => {
  assert.equal(DESKTOP_EXTERNAL_LINK_CLICK_OPTIONS.capture, true);
});

test('desktop anchor click opens external links through the native bridge', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  let prevented = false;
  const originalWindow = globalThis.window;
  const originalElement = globalThis.Element;

  class FakeElement {
    constructor(private readonly anchor: unknown) {}

    closest(selector: string) {
      return selector === 'a[href]' ? this.anchor : undefined;
    }
  }

  (globalThis as any).Element = FakeElement;
  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      invoke: async (command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        return true;
      },
    },
  };

  try {
    const anchor = {
      href: 'https://example.org/docs',
      getAttribute: (name: string) => (name === 'href' ? 'https://example.org/docs' : null),
    };

    openDesktopExternalAnchorFromClick({
      defaultPrevented: false,
      button: 0,
      metaKey: false,
      ctrlKey: false,
      shiftKey: false,
      altKey: false,
      target: new FakeElement(anchor),
      preventDefault: () => {
        prevented = true;
      },
    } as unknown as MouseEvent);

    await waitForAsyncOpen();
  } finally {
    (globalThis as any).window = originalWindow;
    (globalThis as any).Element = originalElement;
  }

  assert.equal(prevented, true);
  assert.deepEqual(calls, [
    {
      command: 'desktop_open_external_url',
      args: { url: 'https://example.org/docs' },
    },
  ]);
});

test('desktop anchor click ignores handled, modified, and app-relative links', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  let prevented = false;
  const originalWindow = globalThis.window;
  const originalElement = globalThis.Element;

  class FakeElement {
    constructor(private readonly anchor: unknown) {}

    closest(selector: string) {
      return selector === 'a[href]' ? this.anchor : undefined;
    }
  }

  (globalThis as any).Element = FakeElement;
  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      invoke: async (command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        return true;
      },
    },
  };

  try {
    const externalAnchor = {
      href: 'https://example.org/docs',
      getAttribute: (name: string) => (name === 'href' ? 'https://example.org/docs' : null),
    };
    const relativeAnchor = {
      href: 'http://localhost:44548/settings',
      getAttribute: (name: string) => (name === 'href' ? '/settings' : null),
    };

    for (const event of [
      { defaultPrevented: true, metaKey: false, target: new FakeElement(externalAnchor) },
      { defaultPrevented: false, metaKey: true, target: new FakeElement(externalAnchor) },
      { defaultPrevented: false, metaKey: false, target: new FakeElement(relativeAnchor) },
    ]) {
      openDesktopExternalAnchorFromClick({
        button: 0,
        ctrlKey: false,
        shiftKey: false,
        altKey: false,
        preventDefault: () => {
          prevented = true;
        },
        ...event,
      } as unknown as MouseEvent);
    }

    await waitForAsyncOpen();
  } finally {
    (globalThis as any).window = originalWindow;
    (globalThis as any).Element = originalElement;
  }

  assert.equal(prevented, false);
  assert.deepEqual(calls, []);
});

test('openExternalUrl uses the desktop external-url bridge without window.open fallback', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const openedUrls: string[] = [];
  const originalWindow = globalThis.window;

  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      invoke: async (command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        return command === 'desktop_open_external_url' && args?.url === 'https://example.org/docs';
      },
    },
    open: (url: string) => {
      openedUrls.push(url);
      return {};
    },
  };

  try {
    assert.equal(await openExternalUrl('https://example.org/docs'), true);
    assert.equal(await openExternalUrl('file:///Users/example/.ssh/id_rsa'), false);
  } finally {
    (globalThis as any).window = originalWindow;
  }

  assert.deepEqual(
    calls.filter((call) => call.command !== 'desktop_append_log'),
    [
      {
        command: 'desktop_open_external_url',
        args: { url: 'https://example.org/docs' },
      },
    ]
  );
  assert.deepEqual(openedUrls, []);
});

test('openExternalUrl falls back to window.open outside desktop', async () => {
  const openedUrls: string[] = [];
  const originalWindow = globalThis.window;

  (globalThis as any).window = {
    open: (url: string, target: string, features: string) => {
      openedUrls.push(`${url}|${target}|${features}`);
      return {};
    },
  };

  try {
    assert.equal(await openExternalUrl('https://example.org/docs'), true);
  } finally {
    (globalThis as any).window = originalWindow;
  }

  assert.deepEqual(openedUrls, ['https://example.org/docs|_blank|noopener,noreferrer']);
});

test('openExternalUrlFromClick preserves modified and relative link behavior', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  let prevented = false;
  const originalWindow = globalThis.window;

  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      invoke: async (command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        return true;
      },
    },
  };

  try {
    for (const event of [
      { metaKey: true, url: 'https://example.org/docs' },
      { metaKey: false, url: '/terms' },
    ]) {
      openExternalUrlFromClick(
        {
          altKey: false,
          button: 0,
          ctrlKey: false,
          currentTarget: {},
          defaultPrevented: false,
          preventDefault: () => {
            prevented = true;
          },
          shiftKey: false,
          ...event,
        } as unknown as ReactMouseEvent<HTMLElement>,
        event.url
      );
    }

    await waitForAsyncOpen();
  } finally {
    (globalThis as any).window = originalWindow;
  }

  assert.equal(prevented, false);
  assert.deepEqual(calls, []);
});
