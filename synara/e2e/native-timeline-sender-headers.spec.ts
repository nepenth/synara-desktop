import { expect, test } from '@playwright/test';

test("a message after the sender's own join row still shows name and avatar", async ({ page }) => {
  await page.goto('/e2e/native-timeline-harness/index.html?scenario=sender-headers');
  const hola = page.locator('[data-native-timeline-event-id="$ian-hola"]');
  await expect(hola).toContainText('hola');
  await expect(hola.getByText('ian', { exact: true })).toBeVisible();
  // No avatar URL: the default letter avatar stands in.
  await expect(hola.getByText('i', { exact: true })).toBeVisible();

  // A second message straight after a visible one from Ian stays grouped.
  const again = page.locator('[data-native-timeline-event-id="$ian-again"]');
  await expect(again).toContainText('again');
  await expect(again.getByText('ian', { exact: true })).toHaveCount(0);
  await expect(again.getByText('i', { exact: true })).toHaveCount(0);
});

test("a sender's name and avatar open that sender's profile", async ({ page }) => {
  await page.goto('/e2e/native-timeline-harness/index.html?scenario=sender-headers');
  const hola = page.locator('[data-native-timeline-event-id="$ian-hola"]');
  const opened = async () =>
    JSON.parse((await page.getByTestId('opened-profile').textContent()) || 'null');

  // The name is the control keyboard and screen-reader users reach.
  const name = hola.getByRole('button', { name: 'ian', exact: true });
  await name.focus();
  await page.keyboard.press('Enter');
  const target = await opened();
  expect(target.kind).toBe('user');
  expect(target.userId).toMatch(/^@ian:/);

  // The avatar opens the same profile for a pointer, without a second
  // control with the same name in the accessibility tree.
  await page.getByTestId('opened-profile').evaluate((node) => {
    node.textContent = '';
  });
  const avatar = hola.locator('[data-native-timeline-sender-avatar]');
  await expect(avatar).toHaveAttribute('aria-hidden', 'true');
  await expect(avatar).toHaveAttribute('tabindex', '-1');
  await avatar.click();
  expect(await opened()).toEqual(target);

  // A grouped follow-on message has no header, so no second control.
  const again = page.locator('[data-native-timeline-event-id="$ian-again"]');
  await expect(again.locator('[data-native-timeline-sender]')).toHaveCount(0);
});
