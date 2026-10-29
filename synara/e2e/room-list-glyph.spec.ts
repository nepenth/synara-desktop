import { expect, test } from '@playwright/test';

test('DM rows show the peer in the # slot and other rooms keep #', async ({ page }) => {
  await page.goto('/e2e/room-list-harness/index.html?roomGlyph');
  const rows = page.getByTestId('room-glyph-row');
  await expect(rows).toHaveCount(4);
  const row = (roomId: string) => page.locator(`[data-room-id="${roomId}"]`);

  // Avatar when Core has one, initial otherwise; both decorative.
  const alice = row('!alice:example.test');
  await expect(alice.locator('[aria-hidden="true"] img')).toHaveCount(1);
  await expect(alice).not.toContainText('#');
  const bob = row('!bob:example.test');
  await expect(bob.locator('[aria-hidden="true"]').first()).toHaveText('B', { useInnerText: true });
  await expect(bob.locator('img')).toHaveCount(0);

  // A group, and a DM whose single peer Core does not know, keep #.
  await expect(row('!ops:example.test').locator('[aria-hidden="true"]')).toHaveText('#');
  await expect(row('!group:example.test').locator('[aria-hidden="true"]')).toHaveText('#');

  // The row's accessible name is still the room name alone.
  await expect(page.getByRole('link', { name: 'Alice Liddell', exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: 'bob', exact: true })).toBeVisible();

  // The person glyph fills the # slot without changing the 32px row height.
  const glyphBox = await alice.locator('[aria-hidden="true"]').first().boundingBox();
  expect(Math.round(glyphBox!.width)).toBe(18);
  expect(Math.round(glyphBox!.height)).toBe(18);
  const heights = await rows.evaluateAll((nodes) =>
    nodes.map((node) => Math.round(node.getBoundingClientRect().height))
  );
  expect(heights).toEqual(heights.map(() => 32));
});
