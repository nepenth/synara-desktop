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
