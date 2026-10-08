import { fileURLToPath } from 'node:url';
import { defineConfig, devices } from '@playwright/test';
export default defineConfig({
  testDir: '..',
  testMatch: ['settings-layout.spec.ts'],
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
