import { fileURLToPath } from 'node:url';
import { defineConfig, devices } from '@playwright/test';
export default defineConfig({
  // Load the harness once before tests so a cold Vite transform cannot fail
  // the first test's 5 s expect timeouts.
  globalSetup: fileURLToPath(new URL('../warm-harness.ts', import.meta.url)),
  metadata: { warmPages: ['/e2e/settings-harness/index.html'] },
  testDir: '..',
  testMatch: ['settings-layout.spec.ts', 'settings-screens.spec.ts'],
  workers: 2,
  retries: 0,
  timeout: 60000,
  reporter: 'line',
  use: { baseURL: 'http://127.0.0.1:4189', headless: true },
  webServer: {
    cwd: fileURLToPath(new URL('../..', import.meta.url)),
    command: 'node node_modules/vite/bin/vite.js --config e2e/settings-harness/vite.config.ts',
    url: 'http://127.0.0.1:4189',
    // Never reuse a server on this fixed port; strictPort fails a busy port.
    reuseExistingServer: false,
    timeout: 30000,
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'], viewport: { width: 1280, height: 860 } },
    },
  ],
});
