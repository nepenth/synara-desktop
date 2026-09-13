import { expect, test } from '@playwright/test';

for (const kind of ['home', 'direct', 'space'] as const) {
  test(`Open message uses the production ${kind} route and preserves encoded Matrix identifiers`, async ({
    page,
  }) => {
    await page.goto(`/e2e/approvals-harness/routing.html?kind=${kind}`);
    await page.getByRole('button', { name: 'Open approvals', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Agent approvals' })).toBeVisible();
    await page.getByRole('button', { name: 'Open message', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Room destination' })).toBeVisible();
    const prefix = kind === 'space' ? '/%23origin-parent%3Aexample.test' : `/${kind}`;
    await expect(page.getByTestId('location')).toHaveText(
      `${prefix}/%23agent-room%3Aexample.test/%24approval%2F%2B%3F%2525%3Aexample.test`
    );
    await expect(page.getByTestId('room-param')).toHaveText('#agent-room:example.test');
    await expect(page.getByTestId('event-param')).toHaveText('$approval/+?%25:example.test');
    await expect(page.getByTestId('space-param')).toHaveText(
      kind === 'space' ? '#origin-parent:example.test' : ''
    );
  });
}

for (const kind of ['home', 'direct', 'space'] as const) {
  test(`Open message preserves an unaliased Matrix room ID on the ${kind} route`, async ({
    page,
  }) => {
    await page.goto(`/e2e/approvals-harness/routing.html?kind=${kind}&rawId`);
    await page.getByRole('button', { name: 'Open approvals', exact: true }).click();
    await page.getByRole('button', { name: 'Open message', exact: true }).click();
    const prefix = kind === 'space' ? '/%23origin-parent%3Aexample.test' : `/${kind}`;
    await expect(page.getByTestId('location')).toHaveText(
      `${prefix}/!agent-room%3Aexample.test/%24approval%2F%2B%3F%2525%3Aexample.test`
    );
    await expect(page.getByTestId('room-param')).toHaveText('!agent-room:example.test');
    await expect(page.getByTestId('event-param')).toHaveText('$approval/+?%25:example.test');
  });
}

for (const origin of [
  '/direct/',
  '/inbox/later/?saved=true',
  '/%23origin-parent%3Aexample.test/',
]) {
  test(`Mobile Approvals Back returns to ${origin}`, async ({ page }) => {
    await page.goto(`/e2e/approvals-harness/routing.html?origin=${encodeURIComponent(origin)}`);
    await page.getByRole('button', { name: 'Open approvals', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Agent approvals' })).toBeVisible();
    await page.getByRole('button', { name: 'Back', exact: true }).click();
    await expect(page.getByTestId('location')).toHaveText(origin);
    await expect(page.getByRole('heading', { name: 'Origin section' })).toBeVisible();
  });
}

test('directly loaded Approvals Back has a safe Home fallback', async ({ page }) => {
  await page.goto('/e2e/approvals-harness/routing.html?origin=%2Fapprovals%2F');
  await page.getByRole('button', { name: 'Back', exact: true }).click();
  await expect(page.getByTestId('location')).toHaveText('/home/');
});
