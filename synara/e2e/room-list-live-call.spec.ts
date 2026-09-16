import { expect, test } from '@playwright/test';

test('hasActiveCall shows a live chip and isCall alone does not', async ({ page }) => {
  await page.goto('/e2e/room-list-harness/index.html?livecall');
  await expect(page.getByTestId('live-call-chip')).toHaveText('3 live');
  await expect(page.getByTestId('live-call-room')).toHaveAttribute('data-has-active-call', 'true');
  await expect(page.getByTestId('voice-room-type')).toHaveAttribute('data-is-call', 'true');
  await expect(page.getByTestId('voice-room-type')).toHaveAttribute(
    'data-has-active-call',
    'false'
  );
  await expect(page.getByTestId('voice-room-type')).not.toContainText('live');
  await expect(page.getByTestId('voice-room-type')).not.toContainText('In a call');
});
