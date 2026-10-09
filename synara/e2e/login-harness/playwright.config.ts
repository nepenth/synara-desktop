import { fileURLToPath } from 'node:url';
import { defineConfig, devices } from '@playwright/test';
export default defineConfig({
  globalSetup: fileURLToPath(new URL('../warm-harness.ts', import.meta.url)),
  metadata: { warmPages: ['/e2e/login-harness/index.html'] },
  testDir: '..',
  testMatch: ['login-screen.spec.ts'],
  workers: 2,
  retries: 0,
  timeout: 60000,
  reporter: 'line',
  use: { baseURL: 'http://127.0.0.1:4191', headless: true },
  webServer: {
    cwd: fileURLToPath(new URL('../..', import.meta.url)),
    command: 'node node_modules/vite/bin/vite.js --config e2e/login-harness/vite.config.ts',
    url: 'http://127.0.0.1:4191',
    reuseExistingServer: false,
    timeout: 30000,
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'], viewport: { width: 1280, height: 860 } },
    },
    {
      name: 'webkit',
      use: { ...devices['Desktop Safari'], viewport: { width: 1280, height: 860 } },
    },
  ],
});
