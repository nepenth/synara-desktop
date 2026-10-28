import { expect, test } from '@playwright/test';

const open = async (page: import('@playwright/test').Page) => {
  await page.goto('/e2e/native-timeline-harness/index.html?scenario=live&nativeEvents=1');
  await expect(page.locator('[data-native-timeline-event-id]').first()).toBeVisible();
};

const addReactionToLastRow = async (page: import('@playwright/test').Page) => {
  const eventId = await page
    .locator('[data-native-timeline-event-id]')
    .last()
    .getAttribute('data-native-timeline-event-id');
  await page.evaluate((id) => {
    (
      window as unknown as { nativeTimelineFixture: { addReaction(eventId?: string): void } }
    ).nativeTimelineFixture.addReaction(id ?? undefined);
  }, eventId);
  return page.locator(`[data-native-timeline-event-id="${eventId}"]`);
};

test('a reaction pill names who reacted, on hover and to assistive tech', async ({ page }) => {
  await open(page);
  const row = await addReactionToLastRow(page);
  const pill = row.getByRole('button', { name: /Alice Liddell reacted with ✅/ });
  await expect(pill).toBeVisible();
  await expect(pill).toHaveText('✅ 1');

  // The harness turns reacting off, so the pill is disabled; its wrapper
  // still carries the tooltip and the context menu, as in the app.
  await row.locator('[data-reaction-key="✅"]').hover();
  await expect(page.getByRole('tooltip')).toContainText('Alice Liddell reacted with ✅');
});

test('the context menu on a pill lists each person by name and Matrix ID', async ({ page }) => {
  await open(page);
  const row = await addReactionToLastRow(page);
  await row.locator('[data-reaction-key="✅"]').click({ button: 'right' });

  const dialog = page.getByRole('dialog', { name: 'Reactions' });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByText('Alice Liddell')).toBeVisible();
  await expect(dialog.getByText('@alice:example.org')).toBeVisible();
});
