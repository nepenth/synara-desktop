import { fileURLToPath } from 'node:url';
import { defineConfig, devices } from '@playwright/test';
export default defineConfig({
  testDir: '..',
  testMatch: 'approvals-center.spec.ts',
  fullyParallel: true,
  retries: 0,
  timeout: 30000,
  reporter: 'line',
  use: { baseURL: 'http://127.0.0.1:4185', headless: true, trace: 'retain-on-failure' },
  webServer: {
    cwd: fileURLToPath(new URL('../..', import.meta.url)),
    command: 'node node_modules/vite/bin/vite.js --config e2e/approvals-harness/vite.config.ts',
    url: 'http://127.0.0.1:4185',
    reuseExistingServer: !process.env.CI,
    timeout: 30000,
  },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } },
  ],
});
