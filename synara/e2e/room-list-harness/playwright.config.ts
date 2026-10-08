import { fileURLToPath } from 'node:url';
import { defineConfig, devices } from '@playwright/test';
export default defineConfig({
  testDir: '..',
  testMatch: [
    'room-list-scroll.spec.ts',
    'avatar-lifetime.spec.ts',
    'room-list-live-call.spec.ts',
    'navigation-unread.spec.ts',
  ],
  fullyParallel: false,
  retries: 0,
  timeout: 30000,
  reporter: 'line',
  use: { baseURL: 'http://127.0.0.1:4183', headless: true, trace: 'retain-on-failure' },
  webServer: {
    cwd: fileURLToPath(new URL('../..', import.meta.url)),
    command: 'node node_modules/vite/bin/vite.js --config e2e/room-list-harness/vite.config.ts',
    url: 'http://127.0.0.1:4183',
    // Never reuse a server already on this fixed port: it may belong to another
    // checkout, so every test would silently run against the wrong code. With
    // strictPort in the harness Vite config, a busy port fails at startup instead.
    reuseExistingServer: false,
    timeout: 30000,
  },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    {
      name: 'webkit',
      testMatch: 'navigation-unread.spec.ts',
      use: { ...devices['Desktop Safari'] },
    },
  ],
});
