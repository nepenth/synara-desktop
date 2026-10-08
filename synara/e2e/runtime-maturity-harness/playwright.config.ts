import { fileURLToPath } from 'node:url';
import { defineConfig } from '@playwright/test';
export default defineConfig({
  globalSetup: fileURLToPath(new URL('../warm-harness.ts', import.meta.url)),
  metadata: { warmPages: ['/e2e/runtime-maturity-harness/index.html'] },
  testDir: '..',
  testMatch: 'runtime-maturity.spec.ts',
  retries: 0,
  // One dev server serves every page; bound concurrency so a loaded machine
  // cannot push first renders past the 5 s expect timeout.
  workers: 4,
  outputDir: '../../test-results/runtime-maturity',
  reporter: 'line',
  use: { baseURL: 'http://127.0.0.1:4191', headless: true },
  webServer: {
    cwd: fileURLToPath(new URL('../..', import.meta.url)),
    command:
      'node node_modules/vite/bin/vite.js --config e2e/runtime-maturity-harness/vite.config.ts --port 4191 --host 127.0.0.1 --strictPort',
    url: 'http://127.0.0.1:4191/e2e/runtime-maturity-harness/index.html',
    reuseExistingServer: false,
  },
  // Keep the browser's native user agent aligned with navigator.platform for editor hotkeys.
  projects: [
    { name: 'chromium', use: { browserName: 'chromium', viewport: { width: 1280, height: 720 } } },
    { name: 'webkit', use: { browserName: 'webkit', viewport: { width: 1280, height: 720 } } },
  ],
});
