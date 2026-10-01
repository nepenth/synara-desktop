import { fileURLToPath } from 'node:url';
import { defineConfig, devices } from '@playwright/test';
export default defineConfig({
  testDir: '..',
  testMatch: 'runtime-maturity.spec.ts',
  retries: 0,
  outputDir: '../../test-results/runtime-maturity',
  reporter: 'line',
  use: { baseURL: 'http://127.0.0.1:4190', headless: true },
  webServer: {
    cwd: fileURLToPath(new URL('../..', import.meta.url)),
    command: 'node node_modules/vite/bin/vite.js --port 4190 --host 127.0.0.1 --strictPort',
    url: 'http://127.0.0.1:4190/e2e/runtime-maturity-harness/index.html',
    reuseExistingServer: false,
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
});
