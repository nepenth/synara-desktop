import { expect, Page, test } from '@playwright/test';

// "Verify This Device" after sign-in and the Reset Identity warning, mounted
// from the production app shell (e2e/settings-harness). Screenshots are
// written only when SETTINGS_SHOTS_DIR is set.
const SHOTS = process.env.SETTINGS_SHOTS_DIR;
const HARNESS = '/e2e/settings-harness/index.html';

const shot = async (page: Page, name: string) => {
  if (SHOTS) await page.screenshot({ path: `${SHOTS}/${name}.png` });
};

const prompt = (page: Page) => page.getByRole('dialog', { name: 'Verify This Device' });

test('the prompt offers verification, the recovery key and a reset', async ({ page }) => {
  await page.goto(`${HARNESS}?verifyPrompt=1#/home/`);
  await expect(prompt(page)).toBeVisible();
  for (const name of [
    'Verify from Another Device',
    'Use Recovery Key',
    'Reset Identity',
    'Later',
  ]) {
    await expect(prompt(page).getByRole('button', { name, exact: true })).toBeVisible();
  }
  await shot(page, 'verify-this-device-prompt');

  await prompt(page).getByRole('button', { name: 'Reset Identity', exact: true }).click();
  const reset = page.getByRole('alertdialog', { name: 'Reset Identity' });
  await expect(reset).toBeVisible();
  await expect(reset).toContainText('People you talk to will see that your identity changed.');
  await expect(reset).toContainText(
    'You will need to verify your other devices and contacts again.'
  );
  await expect(reset).toContainText(
    'Messages that only your old keys can read may be lost, unless another device still has them.'
  );
  await expect(reset).toContainText(
    'Starting the reset removes your key backup and recovery key right away, even if you cancel before it finishes.'
  );
  await shot(page, 'reset-identity-warning');

  await reset.getByRole('button', { name: 'Reset Identity', exact: true }).click();
  const password = page.getByRole('dialog', { name: 'Reset Identity' });
  await expect(password.getByText('Account Password', { exact: true })).toBeVisible();
  await password.getByRole('button', { name: 'Cancel', exact: true }).click();
  // Cancelling after Start points back to recovery setup.
  await expect(password).toContainText('Set up recovery in Settings once this device is verified');
  await password.getByRole('button', { name: 'Close', exact: true }).last().click();
  await expect(password).toBeHidden();
});

test('Later dismisses the prompt', async ({ page }) => {
  await page.goto(`${HARNESS}?verifyPrompt=1#/home/`);
  await prompt(page).getByRole('button', { name: 'Later', exact: true }).click();
  await expect(prompt(page)).toBeHidden();
});

test('no prompt when Core does not ask for one', async ({ page }) => {
  await page.goto(`${HARNESS}#/home/`);
  await expect(page.getByRole('navigation').first()).toBeVisible();
  await expect(prompt(page)).toHaveCount(0);
});
