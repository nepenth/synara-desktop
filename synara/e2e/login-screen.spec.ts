import { expect, test, type Page } from '@playwright/test';

const HARNESS = '/e2e/login-harness/index.html';

const open = async (page: Page, query = '') => {
  await page.goto(`${HARNESS}${query}`);
  await expect(page.getByText('Connected', { exact: false })).toBeVisible();
};

/** Records whether the credential inputs are ever removed from the DOM. */
const watchFormRemovals = (page: Page) =>
  page.evaluate(() => {
    const username = document.querySelector('input[name="usernameInput"]');
    const password = document.querySelector('input[name="passwordInput"]');
    const state = { removed: false };
    (window as unknown as { synaraFormWatch: typeof state }).synaraFormWatch = state;
    new MutationObserver(() => {
      if (!username?.isConnected || !password?.isConnected) state.removed = true;
    }).observe(document.body, { childList: true, subtree: true });
  });

const formWasRemoved = (page: Page) =>
  page.evaluate(
    () => (window as unknown as { synaraFormWatch: { removed: boolean } }).synaraFormWatch.removed
  );

test('renders the brand panel and the sign-in form', async ({ page }) => {
  await open(page);
  await expect(page.getByRole('img', { name: 'Synara' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Welcome back' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Sign in' })).toBeEnabled();
});

test('changing the homeserver keeps the form mounted and typed input intact', async ({ page }) => {
  await open(page, '?delay=900');
  const username = page.locator('input[name="usernameInput"]');
  await username.fill('alice');
  await watchFormRemovals(page);

  const server = page.locator('#synara-homeserver');
  await server.fill('agents.example.test');
  await expect(page.getByText('Connecting to agents.example.test')).toBeVisible();
  // The old server's form must not be submittable while the new one resolves.
  await expect(page.getByRole('button', { name: 'Connecting…' })).toBeDisabled();
  await expect(page.getByText('Sign in to your account on agents.example.test.')).toBeVisible({
    timeout: 10_000,
  });
  await expect(page.getByRole('button', { name: 'Sign in' })).toBeEnabled();

  expect(await formWasRemoved(page)).toBe(false);
  await expect(username).toHaveValue('alice');
});

test('an unreachable homeserver shows an inline error without dropping the form', async ({
  page,
}) => {
  await open(page);
  await page.locator('input[name="usernameInput"]').fill('alice');
  await watchFormRemovals(page);

  await page.locator('#synara-homeserver').fill('down.example.test');
  await expect(page.getByRole('alert')).toContainText("down.example.test isn't responding");
  await expect(page.getByRole('button', { name: 'Retry' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Sign in' })).toBeDisabled();
  expect(await formWasRemoved(page)).toBe(false);

  await page.locator('#synara-homeserver').fill('matrix.example.test');
  await expect(page.getByRole('button', { name: 'Sign in' })).toBeEnabled({ timeout: 10_000 });
  await expect(page.locator('input[name="usernameInput"]')).toHaveValue('alice');
});

test('reduced motion stops the logo and background animation', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await open(page);
  const running = await page.evaluate(
    () => document.getAnimations().filter((animation) => animation.playState === 'running').length
  );
  expect(running).toBe(0);
});

test('the background grid stays still and the mark sits centered over the wordmark', async ({
  page,
}) => {
  await open(page);
  const backgroundMotion = await page.evaluate(
    () =>
      document
        .getAnimations()
        .filter((animation) =>
          (animation.effect as KeyframeEffect | null)
            ?.getKeyframes()
            .some((frame) => 'backgroundPosition' in frame)
        ).length
  );
  expect(backgroundMotion).toBe(0);

  const mark = await page.getByRole('img', { name: 'Synara' }).boundingBox();
  const word = await page.getByRole('heading', { name: 'Synara' }).boundingBox();
  expect(mark && word).toBeTruthy();
  const markCenter = mark!.x + mark!.width / 2;
  const wordCenter = word!.x + word!.width / 2;
  expect(Math.abs(markCenter - wordCenter)).toBeLessThan(2);
});
