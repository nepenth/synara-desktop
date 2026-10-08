import assert from 'node:assert/strict';
import { test } from 'node:test';
import { isLinuxOS, isMacOS, mobileOrTablet, synaraDeviceDisplayName } from '../user-agent';

test('UA-parser2 keeps desktop shortcut and device-name platform decisions accurate', () => {
  const originalWindow = globalThis.window;
  try {
    for (const [userAgent, expectedName, mac, linux, mobile] of [
      [
        'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 Version/17.0 Safari/605.1.15',
        'Synara macOS',
        true,
        false,
        false,
      ],
      [
        'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/153.0.0.0 Safari/537.36',
        'Synara Linux',
        false,
        true,
        false,
      ],
      [
        'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 Version/18.0 Mobile/15E148 Safari/604.1',
        'Synara iOS',
        false,
        false,
        true,
      ],
    ] as const) {
      Object.defineProperty(globalThis, 'window', {
        configurable: true,
        value: { navigator: { userAgent } },
      });
      assert.equal(synaraDeviceDisplayName(), expectedName);
      assert.equal(isMacOS(), mac);
      assert.equal(isLinuxOS(), linux);
      assert.equal(mobileOrTablet(), mobile);
    }
  } finally {
    Object.defineProperty(globalThis, 'window', { configurable: true, value: originalWindow });
  }
});

test('the desktop shell OS wins over a borrowed macOS user agent', () => {
  const originalWindow = globalThis.window;
  try {
    Object.defineProperty(globalThis, 'window', {
      configurable: true,
      value: {
        navigator: {
          userAgent:
            'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 Version/17.0 Safari/605.1.15',
        },
        __SYNARA_DESKTOP__: { platform: 'tauri', os: 'linux' },
      },
    });
    assert.equal(isLinuxOS(), true);
    assert.equal(isMacOS(), false);
    assert.equal(synaraDeviceDisplayName(), 'Synara Linux');
  } finally {
    Object.defineProperty(globalThis, 'window', { configurable: true, value: originalWindow });
  }
});
