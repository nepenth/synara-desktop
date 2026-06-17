import test from 'node:test';
import assert from 'node:assert/strict';
import { openDesktopExternalAnchorFromClick } from '../appLinks';

const waitForAsyncOpen = () => new Promise((resolve) => setTimeout(resolve, 0));

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
