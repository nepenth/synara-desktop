import { expect, test } from '@playwright/test';
const route = '/e2e/approvals-harness/index.html';

test('permanent approval requires confirmation; cancel sends nothing, deny updates rail and Recent', async ({
  page,
}) => {
  await page.goto(route);
  await expect(
    page.getByRole('button', { name: 'Approvals · 2 pending', exact: true })
  ).toBeVisible();
  const first = page.getByRole('region', { name: /Approval from Publishing/ });
  await first.getByRole('button', { name: '♾️ Approve always', exact: true }).click();
  await expect(first.getByRole('button', { name: /Confirm approve always/ })).toBeFocused();
  await expect(page.getByTestId('fixture-decisions')).toContainText('0 decisions');
  await first.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(first.getByRole('button', { name: /Confirm approve always/ })).toHaveCount(0);
  await expect(first.getByRole('button', { name: '♾️ Approve always', exact: true })).toBeFocused();
  await first.getByRole('button', { name: '❌ Deny', exact: true }).click();
  await expect(
    page.getByRole('button', { name: 'Approvals · 1 pending', exact: true })
  ).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Agent approvals', exact: true })).toBeFocused();
  await page.getByRole('button', { name: 'Recent · 1', exact: true }).click();
  await expect(
    page.getByText('Your account has already sent a decision for this request.')
  ).toBeVisible();
  await expect(page.getByTestId('fixture-decisions')).toContainText(
    '1 decisions · agent-approval.deny'
  );
});

test('confirm approve always sends the intended native action once', async ({ page }) => {
  await page.goto(route);
  const first = page.getByRole('region', { name: /Approval from Publishing/ });
  await first.getByRole('button', { name: '♾️ Approve always', exact: true }).click();
  await first.getByRole('button', { name: /Confirm approve always/ }).click();
  await expect(page.getByTestId('fixture-decisions')).toContainText(
    '1 decisions · agent-approval.approve-always'
  );
  await expect(page.getByTestId('pending-count')).toHaveText('1 pending');
});

test('search does not change global count and source message navigation retains identity', async ({
  page,
}) => {
  await page.goto(route);
  await page.getByRole('searchbox', { name: 'Search approval requests' }).fill('Home');
  await expect(page.getByRole('region', { name: /^Approval from / })).toHaveCount(1);
  await expect(page.getByTestId('pending-count')).toHaveText('2 pending');
  await page.getByRole('button', { name: 'Open message', exact: true }).click();
  await expect(page.getByText('Opened !home:example.test / $request-2')).toBeVisible();
  await page.getByRole('searchbox').fill('no-such-request');
  await expect(page.getByRole('heading', { name: 'No matching requests' })).toBeVisible();
});

test('expiry removes pending actions at the deadline without a network refresh', async ({
  page,
}) => {
  await page.goto(`${route}?expires`);
  await expect(page.getByTestId('pending-count')).toHaveText('1 pending');
  await expect(page.getByTestId('pending-count')).toHaveText('0 pending', { timeout: 6000 });
  await expect(page.getByRole('heading', { name: 'No pending approvals' })).toBeVisible();
  await page.getByRole('button', { name: 'Recent · 1', exact: true }).click();
  await expect(page.getByText('Expired', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: '✅ Approve once', exact: true })).toHaveCount(0);
});

test('remote decisions, partial coverage and network failures are visible with recovery', async ({
  page,
}) => {
  await page.goto(route);
  await page.getByRole('button', { name: 'Toggle partial coverage' }).click();
  await expect(page.getByText(/Some rooms could not be fully checked/)).toBeVisible();
  await page.getByRole('button', { name: 'Toggle network failure' }).click();
  await expect(page.getByRole('alert')).toContainText('could not be refreshed');
  await expect(page.getByTestId('pending-count')).toHaveText('2 pending');
  await page.getByRole('button', { name: 'Toggle network failure' }).click();
  await expect(page.getByRole('alert')).toHaveCount(0);
  await page.getByRole('button', { name: 'Decision on another device' }).click();
  await expect(page.getByTestId('pending-count')).toHaveText('0 pending');
  await expect(page.getByRole('button', { name: '✅ Approve once', exact: true })).toHaveCount(0);
});

test('rejected decision leaves request available for review and preserves count', async ({
  page,
}) => {
  await page.goto(`${route}?decisionError`);
  await page.getByRole('button', { name: '✅ Approve once', exact: true }).first().click();
  await expect(
    page.getByRole('alert').filter({
      hasText: 'Approval could not be submitted. The request may be invalid or expired.',
    })
  ).toBeVisible();
  await expect(page.getByTestId('pending-count')).toHaveText('2 pending');
  await expect(
    page.getByRole('button', { name: '✅ Approve once', exact: true }).first()
  ).toBeEnabled();
});

test('light theme and narrow viewport keep approval controls usable without horizontal overflow', async ({
  page,
}) => {
  await page.setViewportSize({ width: 540, height: 900 });
  await page.goto(`${route}?light`);
  await expect(page.getByRole('searchbox')).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});

test('Core-pending unformatted requests show full source and native approval controls', async ({
  page,
}) => {
  await page.goto(`${route}?unparsed`);
  await expect(page.getByRole('region', { name: /^Approval from / })).toHaveCount(2);
  await expect(
    page.getByText('⚠️ Dangerous command requires approval', { exact: true })
  ).toBeVisible();
  await expect(
    page.getByText('Unrecognized approval request: inspect the original operation', { exact: true })
  ).toBeVisible();
  await expect(page.getByRole('button', { name: '✅ Approve once', exact: true })).toHaveCount(2);
  await expect(page.getByRole('button', { name: 'Open message', exact: true })).toHaveCount(2);
});

test('command block stays visible through clock updates and refreshes', async ({ page }) => {
  await page.goto(route);
  const card = page.getByRole('region', { name: /^Approval from / }).first();
  const command = card.getByText('Command: rm -rf ./build/preview', { exact: true });
  await expect(command).toBeVisible();
  await page.getByRole('button', { name: 'Refresh approval requests' }).click();
  await expect(command).toBeVisible();
  await page.waitForTimeout(1200);
  await expect(command).toBeVisible();
});

test('native session generation resets optimistic decisions on the same client facade', async ({
  page,
}) => {
  await page.goto(route);
  await page.getByRole('button', { name: '✅ Approve once', exact: true }).first().click();
  await expect(page.getByTestId('pending-count')).toHaveText('1 pending');
  await expect(page.getByText(/resolved. Available in Recent/)).toBeVisible();
  await page.getByRole('button', { name: 'Restart native session' }).click();
  await expect(page.getByTestId('pending-count')).toHaveText('2 pending');
  await expect(page.getByText(/resolved. Available in Recent/)).toHaveCount(0);
});

test('late decision completion from an old session cannot resolve the new session request', async ({
  page,
}) => {
  await page.goto(`${route}?delayedDecision`);
  await page.getByRole('button', { name: '✅ Approve once', exact: true }).first().click();
  await page.getByRole('button', { name: 'Restart native session' }).click();
  await expect(
    page.getByRole('button', { name: '✅ Approve once', exact: true }).first()
  ).toBeEnabled();
  await page.getByRole('button', { name: 'Complete delayed decision' }).click();
  await page.getByRole('button', { name: 'Refresh approval requests' }).click();
  await expect(page.getByTestId('pending-count')).toHaveText('2 pending');
  await expect(page.getByText(/resolved. Available in Recent/)).toHaveCount(0);
});

test('settled partial coverage has a distinct rail status even with no pending items', async ({
  page,
}) => {
  await page.goto(`${route}?empty&partial`);
  await expect(
    page.getByRole('button', { name: 'Approvals · partial coverage', exact: true })
  ).toBeVisible();
  await expect(page.getByRole('button', { name: /Approvals.*checking rooms/ })).toHaveCount(0);
});

test('external shared-native decisions update the badge immediately even when refresh fails', async ({
  page,
}) => {
  await page.goto(route);
  await expect(page.getByTestId('pending-count')).toHaveText('2 pending');
  await page.getByRole('button', { name: 'Decide outside inbox while readback fails' }).click();
  await expect(page.getByRole('alert')).toContainText('could not be refreshed');
  await expect(page.getByTestId('pending-count')).toHaveText('1 pending');
  await expect(page.getByRole('button', { name: /Approvals · 1/ })).toBeVisible();
});

test('discovery is active only while the approvals page is selected', async ({ page }) => {
  await page.goto(route);
  await expect(page.getByTestId('discovery-active')).toHaveText('true');
  await page.getByRole('button', { name: 'Leave approvals' }).click();
  await expect(page.getByTestId('discovery-active')).toHaveText('false');
  await page.getByRole('button', { name: 'Show approvals' }).click();
  await expect(page.getByTestId('discovery-active')).toHaveText('true');
});

test('Core permissions disable decisions and truncated previews require the source message', async ({
  page,
}) => {
  await page.goto(`${route}?noPermission`);
  await expect(
    page.getByRole('button', { name: '✅ Approve once', exact: true }).first()
  ).toBeDisabled();
  await expect(
    page.getByText('You do not have permission to react in this room.').first()
  ).toBeVisible();
  await page.goto(`${route}?truncated`);
  await expect(
    page.getByText(
      'This is a shortened preview. Open the original message to review the full request.'
    )
  ).toHaveCount(2);
  await expect(page.getByRole('button', { name: '✅ Approve once', exact: true })).toHaveCount(0);
});

test('healthy idle latest-event coverage has no alarm badge with zero requests', async ({
  page,
}) => {
  await page.goto(`${route}?idle&latest&empty`);
  const rail = page.getByRole('button', { name: 'Approvals · recent activity', exact: true });
  await expect(rail).toBeVisible();
  await expect(rail.locator('..').locator('span[aria-hidden="true"]')).toHaveCount(0);
  await expect(page.getByTestId('discovery-active')).toHaveText('false');
});

test('latest-event pending counts are exact rather than partial-coverage alarms', async ({
  page,
}) => {
  await page.goto(`${route}?idle&latest`);
  const rail = page.getByRole('button', {
    name: 'Approvals · 2 pending · recent activity',
    exact: true,
  });
  await expect(rail).toBeVisible();
  await expect(rail.locator('..').locator('span[aria-hidden="true"]')).toHaveText('2');
});

test('loading by itself neither displays an alarm nor appends a plus to pending counts', async ({
  page,
}) => {
  await page.goto(`${route}?loading&empty`);
  const emptyRail = page.getByRole('button', { name: 'Approvals · checking rooms', exact: true });
  await expect(emptyRail).toBeVisible();
  await expect(emptyRail.locator('..').locator('span[aria-hidden="true"]')).toHaveCount(0);
  await page.goto(`${route}?loading`);
  const pendingRail = page.getByRole('button', {
    name: 'Approvals · 2 pending · checking rooms',
    exact: true,
  });
  await expect(pendingRail).toBeVisible();
  await expect(pendingRail.locator('..').locator('span[aria-hidden="true"]')).toHaveText('2');
});

test('genuine incomplete coverage and refresh failures retain distinct alarm badges', async ({
  page,
}) => {
  await page.goto(`${route}?empty&partial`);
  const emptyRail = page.getByRole('button', { name: 'Approvals · partial coverage', exact: true });
  await expect(emptyRail).toBeVisible();
  await expect(emptyRail.locator('..').locator('span[aria-hidden="true"]')).toHaveText('!');
  await page.goto(`${route}?partial`);
  const partialRail = page.getByRole('button', {
    name: 'Approvals · 2+ pending · partial coverage',
    exact: true,
  });
  await expect(partialRail).toBeVisible();
  await expect(partialRail.locator('..').locator('span[aria-hidden="true"]')).toHaveText('2+');
  await page.getByRole('button', { name: 'Toggle partial coverage' }).click();
  await expect(
    page.getByRole('button', { name: 'Approvals · 2 pending', exact: true })
  ).toBeVisible();
  await page.getByRole('button', { name: 'Toggle network failure' }).click();
  const errorRail = page.getByRole('button', {
    name: 'Approvals · 2+ pending · unavailable',
    exact: true,
  });
  await expect(errorRail).toBeVisible();
  await expect(errorRail.locator('..').locator('span[aria-hidden="true"]')).toHaveText('2+');
  await page.goto(`${route}?empty&error`);
  const emptyErrorRail = page.getByRole('button', { name: 'Approvals · unavailable', exact: true });
  await expect(emptyErrorRail).toBeVisible();
  await expect(emptyErrorRail.locator('..').locator('span[aria-hidden="true"]')).toHaveText('!');
});

test('leaving healthy discovery switches to neutral recent activity without an alarm', async ({
  page,
}) => {
  await page.goto(`${route}?empty`);
  await expect(page.getByRole('button', { name: 'Approvals', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Leave approvals' }).click();
  const rail = page.getByRole('button', { name: 'Approvals · recent activity', exact: true });
  await expect(rail).toBeVisible();
  await expect(rail.locator('..').locator('span[aria-hidden="true"]')).toHaveCount(0);
  await expect(page.getByTestId('discovery-active')).toHaveText('false');
});
