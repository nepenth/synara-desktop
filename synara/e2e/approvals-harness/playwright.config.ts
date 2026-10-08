import { fileURLToPath } from 'node:url';
import { defineConfig, devices } from '@playwright/test';
export default defineConfig({
  globalSetup: fileURLToPath(new URL('../warm-harness.ts', import.meta.url)),
  metadata: {
    warmPages: [
      '/e2e/approvals-harness/index.html',
      '/e2e/approvals-harness/lifecycle.html',
      '/e2e/approvals-harness/routing.html',
    ],
  },
  testDir: '..',
  testMatch: [
    'approvals-center.spec.ts',
    'approvals-routing.spec.ts',
    'approvals-provider-lifecycle.spec.ts',
  ],
  fullyParallel: true,
  // One dev server serves every page; bound concurrency so a loaded machine
  // cannot push first renders past the 5 s expect timeout.
  workers: 4,
  retries: 0,
  timeout: 30000,
  reporter: 'line',
  use: { baseURL: 'http://127.0.0.1:4185', headless: true, trace: 'retain-on-failure' },
  webServer: {
    cwd: fileURLToPath(new URL('../..', import.meta.url)),
    command: 'node node_modules/vite/bin/vite.js --config e2e/approvals-harness/vite.config.ts',
    url: 'http://127.0.0.1:4185',
    // Never reuse a server already on this fixed port: it may belong to another
    // checkout, so every test would silently run against the wrong code. With
    // strictPort in the harness Vite config, a busy port fails at startup instead.
    reuseExistingServer: false,
    timeout: 30000,
  },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } },
  ],
});
