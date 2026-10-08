import { chromium, type FullConfig } from '@playwright/test';

/**
 * Global setup for dev-server harnesses: load each harness page once before
 * any test runs.
 *
 * Vite transforms modules on first request. Without this, the first tests of
 * a run pay that cold cost inside their own 5 s expect timeouts, which fails
 * them intermittently on a busy machine. Playwright starts `webServer` before
 * global setup, so the server is already listening here.
 *
 * Pages to warm come from `metadata.warmPages` in the harness config.
 */
export default async function warmHarness(config: FullConfig): Promise<void> {
  const pages = (config.metadata.warmPages ?? []) as string[];
  const server = config.webServer;
  if (pages.length === 0 || !server?.url) return;
  const origin = new URL(server.url).origin;
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    for (const path of pages) {
      await page.goto(new URL(path, origin).href, { waitUntil: 'networkidle', timeout: 120_000 });
    }
  } finally {
    await browser.close();
  }
}
