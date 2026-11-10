import { expect, test, type Page } from '@playwright/test';

// The list column without the rail (N1–N5): a header whose title opens the
// space switcher and a bottom bar with Home, Activity, Approvals and the
// account. 60 Home rooms, three DMs and twelve spaces, two of them in a
// "Work" folder; the space order lives in `in.synara.spaces`.
const open = (page: Page, query = '') =>
  page.goto(`/e2e/room-list-harness/index.html?navPane${query}`);
const pane = (page: Page) => page.locator('#synara-room-list-pane');
const bar = (page: Page) => pane(page).getByTestId('nav-bottom-bar');
const barItem = (page: Page, id: string) => pane(page).getByTestId(`nav-bar-${id}`);
const trigger = (page: Page) => pane(page).getByTestId('space-switcher-trigger');
const switcher = (page: Page) => page.getByTestId('space-switcher');
const row = (page: Page, name: string | RegExp) =>
  switcher(page).getByRole('button', { name, exact: typeof name === 'string' });
const route = (page: Page) => page.getByTestId('current-route');
const spaceOrder = (page: Page) =>
  switcher(page)
    .locator('[data-space-id], [data-folder-id]')
    .evaluateAll((rows) =>
      rows.map((el) => el.getAttribute('data-space-id') ?? el.getAttribute('data-folder-id'))
    );
const savedSidebar = async (page: Page) =>
  JSON.parse((await page.locator('body').getAttribute('data-saved-spaces')) ?? 'null');

test('the bar: four places, icons with tooltips, labels once the column is wide', async ({
  page,
}) => {
  await open(page);
  await expect(bar(page)).toHaveAttribute('aria-label', 'Main');
  const items = bar(page).getByRole('button');
  await expect(items).toHaveCount(4);
  await expect(barItem(page, 'home')).toHaveAttribute('aria-current', 'page');
  await expect(barItem(page, 'home')).toHaveAccessibleName('Home');
  await expect(barItem(page, 'activity')).toHaveAccessibleName('Activity');
  await expect(barItem(page, 'account')).toHaveAttribute('aria-haspopup', 'menu');
  // One pending approval from Core: a red count, and the name says so.
  await expect(barItem(page, 'approvals')).toHaveAccessibleName(/^Approvals · 1 pending/);
  await expect(barItem(page, 'approvals').locator('[data-badge-role="mention"]')).toHaveText('1');

  // At the default 256px the bar is icons only, named and with tooltips.
  await expect(bar(page)).not.toContainText('Activity');
  await barItem(page, 'activity').hover();
  await expect(page.getByRole('tooltip')).toHaveText('Activity');

  // Wider than 300px: text labels under the icons.
  const handle = page.getByRole('button', { name: /Resize navigation pane/ });
  await handle.focus();
  for (let i = 0; i < 5; i += 1) await page.keyboard.press('ArrowRight');
  await expect(bar(page)).toContainText('Home');
  await expect(bar(page)).toContainText('Activity');
  await expect(bar(page)).toContainText('Approvals');
  await expect(bar(page)).toContainText('Account');

  // The bar sits at the bottom of the list column, below the scrolling list.
  const paneBox = (await pane(page).boundingBox())!;
  const barBox = (await bar(page).boundingBox())!;
  expect(Math.round(barBox.y + barBox.height)).toBe(Math.round(paneBox.y + paneBox.height));
});

test('the bar navigates: Approvals keeps the Home list, Home returns', async ({ page }) => {
  await open(page);
  await barItem(page, 'approvals').click();
  await expect(route(page)).toHaveText('/approvals/');
  await expect(barItem(page, 'approvals')).toHaveAttribute('aria-current', 'page');
  await expect(barItem(page, 'home')).not.toHaveAttribute('aria-current');
  // The Home list stays beside Approvals (D2).
  await expect(pane(page).getByTestId('nav-search-field')).toBeVisible();
  await barItem(page, 'activity').click();
  await expect(route(page)).toHaveText('/inbox/notifications/');
  await barItem(page, 'home').click();
  await expect(route(page)).toHaveText('/home/');
});

test('Home: badge, right-click and Shift+F10 open "Mark as read"', async ({ page }) => {
  await open(page);
  await page.getByRole('button', { name: 'Receive Home' }).click();
  await expect(barItem(page, 'home').locator('[data-badge-role="neutral"]')).toHaveCount(1);
  await page.getByRole('button', { name: 'Mention Home' }).click();
  await expect(barItem(page, 'home').locator('[data-badge-role="mention"]')).toHaveText('2');
  await expect(barItem(page, 'home')).toHaveAccessibleName('Home, 2 mentions and direct messages');
  await barItem(page, 'home').focus();
  await page.keyboard.press('Shift+F10');
  await page.getByRole('button', { name: 'Mark as read', exact: true }).click();
  await expect(barItem(page, 'home').locator('[data-badge-role]')).toHaveCount(0);
  await expect(page.locator('body')).toHaveAttribute('data-read-rooms', /!room-00:example\.test/);
});

test('the account menu: Settings and Log out, focus back on the avatar', async ({ page }) => {
  await open(page);
  // No cross-signing to offer in the fixture: no warning and no verify entry.
  await expect(barItem(page, 'account')).toHaveAccessibleName('Account');
  await barItem(page, 'account').click();
  await expect(barItem(page, 'account')).toHaveAttribute('aria-expanded', 'true');
  const settings = page.getByRole('button', { name: 'Settings', exact: true });
  await expect(settings).toBeVisible();
  await expect(page.getByRole('button', { name: 'Log out', exact: true })).toBeVisible();
  await expect(
    page.getByRole('button', { name: /^(Verify this device|Review unverified)/ })
  ).toHaveCount(0);
  await page.keyboard.press('Escape');
  await expect(settings).toHaveCount(0);
  await expect(barItem(page, 'account')).toBeFocused();
});

test('the switcher lists Home and every space, with folders and a filter', async ({ page }) => {
  await open(page);
  await expect(trigger(page)).toHaveAttribute('aria-haspopup', 'dialog');
  await trigger(page).click();
  await expect(switcher(page)).toBeVisible();
  await expect(trigger(page)).toHaveAttribute('aria-expanded', 'true');
  // Home is current; the folder is a closed group; twelve spaces > eight: a filter.
  await expect(row(page, 'Home')).toHaveAttribute('aria-current', 'page');
  const work = row(page, /^Work, folder/);
  await expect(work).toHaveAttribute('aria-expanded', 'false');
  await expect(row(page, 'Engineering')).toHaveCount(0);
  const filter = switcher(page).getByRole('searchbox', { name: 'Find a space' });
  await expect(filter).toBeFocused();

  // Type-ahead: accent-insensitive, searches inside closed folders.
  await filter.fill('cafe');
  await expect(row(page, 'Café club')).toBeVisible();
  await expect(row(page, 'Design')).toHaveCount(0);
  await filter.fill('eng');
  await expect(row(page, 'Engineering')).toBeVisible();
  // Esc clears the filter first, then closes.
  await page.keyboard.press('Escape');
  await expect(filter).toHaveValue('');
  await expect(switcher(page)).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(switcher(page)).toHaveCount(0);
  await expect(trigger(page)).toBeFocused();

  // Enter in the filter opens the first match.
  await trigger(page).click();
  await switcher(page).getByRole('searchbox', { name: 'Find a space' }).fill('jazz');
  await page.keyboard.press('Enter');
  await expect(route(page)).toHaveText(/^\/!space-jazz%3Aexample\.test\/lobby$/);
  await expect(pane(page).getByTestId('space-pane')).toHaveText('Jazz');
  await expect(trigger(page)).toHaveAccessibleName(/^Jazz, switch space/);
  await trigger(page).click();
  await expect(row(page, 'Jazz')).toHaveAttribute('aria-current', 'page');
  await row(page, 'Home').click();
  await expect(route(page)).toHaveText('/home/');
});

test('the switcher is keyboard operable: arrows, folders, Shift+F10', async ({ page }) => {
  await open(page);
  await trigger(page).focus();
  await page.keyboard.press('Enter');
  await expect(switcher(page)).toBeVisible();
  // From the filter, ↓ enters the list at Home; ↓ again reaches the first space.
  await page.keyboard.press('ArrowDown');
  await expect(row(page, 'Home')).toBeFocused();
  await page.keyboard.press('ArrowDown');
  await expect(row(page, 'Design')).toBeFocused();
  await page.keyboard.press('ArrowDown');
  const work = row(page, /^Work, folder/);
  await expect(work).toBeFocused();
  // → opens the folder, ↓ walks into it, ← goes back to the folder, ← closes it.
  await page.keyboard.press('ArrowRight');
  await expect(work).toHaveAttribute('aria-expanded', 'true');
  await page.keyboard.press('ArrowDown');
  await expect(row(page, 'Engineering')).toBeFocused();
  await page.keyboard.press('ArrowLeft');
  await expect(work).toBeFocused();
  await page.keyboard.press('ArrowLeft');
  await expect(work).toHaveAttribute('aria-expanded', 'false');
  // End reaches the last space; one row in the tab order (roving tabindex).
  await page.keyboard.press('End');
  await expect(row(page, 'Zeta')).toBeFocused();
  await expect(switcher(page).locator('[data-switcher-row][tabindex="0"]')).toHaveCount(1);
  // Shift+F10 opens the space's menu: the rail's entries.
  await page.keyboard.press('Shift+F10');
  await expect(row(page, 'Zeta options')).toHaveAttribute('aria-expanded', 'true');
  for (const entry of ['Mark as read', 'Invite', 'Copy link', 'Space settings']) {
    await expect(page.getByRole('button', { name: entry, exact: true })).toBeVisible();
  }
  // Zeta has no parent space, so it is always listed: no Unpin.
  await expect(page.getByRole('button', { name: 'Unpin', exact: true })).toHaveCount(0);
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button', { name: 'Copy link', exact: true })).toHaveCount(0);
  await expect(row(page, 'Zeta')).toBeFocused();
  await expect(switcher(page)).toBeVisible();
  await page.keyboard.press('Enter');
  await expect(route(page)).toHaveText(/^\/!space-zeta%3Aexample\.test\/lobby$/);
});

test('unread: space badges in the switcher and a dot on the title chevron', async ({ page }) => {
  await open(page);
  const dot = pane(page).getByTestId('space-switcher-dot');
  await expect(dot).toHaveCount(0);
  await page.getByRole('button', { name: 'Unread in Jazz' }).click();
  await expect(dot.locator('[data-badge-role="neutral"]')).toHaveCount(1);
  await expect(trigger(page)).toHaveAccessibleName(/unread in other spaces$/);
  await page.getByRole('button', { name: 'Mention in Design' }).click();
  await expect(dot.locator('[data-badge-role="mention"]')).toHaveCount(1);
  await expect(trigger(page)).toHaveAccessibleName(/mentions in other spaces$/);
  await trigger(page).click();
  await expect(row(page, 'Design, 1 mention')).toBeVisible();
  await expect(row(page, 'Design, 1 mention').locator('[data-badge-role="mention"]')).toHaveText(
    '1'
  );
  await expect(row(page, 'Jazz, unread messages').locator('[data-badge-role]')).toHaveAttribute(
    'data-badge-role',
    'neutral'
  );
  // In Jazz, Jazz's own unread no longer counts for the dot; Design's still does.
  await row(page, 'Jazz, unread messages').click();
  await expect(dot.locator('[data-badge-role="mention"]')).toHaveCount(1);
  await page.getByRole('button', { name: 'Clear spaces' }).click();
  await expect(dot).toHaveCount(0);
});

test('drag to reorder saves to in.synara.spaces; a failed save rolls back', async ({ page }) => {
  await open(page);
  await trigger(page).click();
  const before = await spaceOrder(page);
  expect(before.slice(0, 3)).toEqual([
    '!space-design:example.test',
    'folder-work',
    '!space-finance:example.test',
  ]);
  // Drag Zeta above Design.
  const zeta = row(page, 'Zeta');
  const design = row(page, 'Design');
  await zeta.hover();
  await page.mouse.down();
  const target = (await design.boundingBox())!;
  await page.mouse.move(target.x + 20, target.y + 4, { steps: 8 });
  await page.mouse.move(target.x + 20, target.y + 2, { steps: 2 });
  await page.mouse.up();
  await expect.poll(async () => (await spaceOrder(page))[0]).toBe('!space-zeta:example.test');
  await expect.poll(async () => (await savedSidebar(page))?.[0]).toBe('!space-zeta:example.test');

  // Now saves fail: the order snaps back and the switcher says so.
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: 'Fail saves' }).click();
  await trigger(page).click();
  const finance = row(page, 'Finance');
  await finance.hover();
  await page.mouse.down();
  const top = (await row(page, 'Zeta').boundingBox())!;
  await page.mouse.move(top.x + 20, top.y + 4, { steps: 8 });
  await page.mouse.move(top.x + 20, top.y + 2, { steps: 2 });
  await page.mouse.up();
  await expect(switcher(page).getByRole('alert')).toContainText(
    "Couldn't save the new order of your spaces. Try again."
  );
  await expect.poll(async () => (await spaceOrder(page))[0]).toBe('!space-zeta:example.test');
  await switcher(page).getByRole('button', { name: 'Dismiss' }).click();
  await expect(switcher(page).getByRole('alert')).toHaveCount(0);
});

test('Manage spaces: move with buttons, group into a folder, ungroup', async ({ page }) => {
  await open(page);
  await trigger(page).click();
  await switcher(page).getByRole('button', { name: 'Manage spaces' }).click();
  const dialog = page.getByRole('dialog', { name: 'Manage spaces' });
  await expect(dialog).toBeVisible();
  await expect(switcher(page)).toHaveCount(0);

  await dialog.getByRole('button', { name: 'Move Finance up' }).click();
  await expect
    .poll(async () => JSON.stringify((await savedSidebar(page))?.slice(0, 2)))
    .toBe(JSON.stringify(['!space-design:example.test', '!space-finance:example.test']));
  expect((await savedSidebar(page))[2].id).toBe('folder-work');

  await dialog.getByRole('checkbox', { name: 'Select Garden' }).check();
  await dialog.getByRole('checkbox', { name: 'Select Hiking' }).check();
  await dialog.getByRole('textbox', { name: 'New folder name' }).fill('Outdoors');
  await dialog.getByRole('button', { name: 'Create folder' }).click();
  await expect
    .poll(async () => JSON.stringify(await savedSidebar(page)))
    .toContain(
      '"name":"Outdoors","content":["!space-garden:example.test","!space-hiking:example.test"]'
    );
  await expect(dialog.getByRole('list', { name: 'In Outdoors' })).toBeVisible();

  await dialog.getByRole('button', { name: 'Take Engineering out of Work' }).click();
  await expect
    .poll(async () => JSON.stringify(await savedSidebar(page)))
    .toContain('"folder-work","name":"Work","content":["!space-infra:example.test"]');

  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);
  // The switcher shows the new folder as a group.
  await trigger(page).click();
  await expect(row(page, /^Outdoors, folder/)).toBeVisible();
});

test('with the list hidden, page headers offer "Show room list" with its unread', async ({
  page,
}) => {
  await open(page);
  const hide = pane(page).getByTestId('room-list-hide');
  await hide.click();
  const show = page.locator('#synara-room-list-toggle');
  await expect(show).toBeFocused();
  // The bar hides with the list; the toggle carries Home's unread instead.
  await expect(bar(page)).not.toBeInViewport();
  await page.getByRole('button', { name: 'Mention Home' }).click();
  await expect(show).toHaveAccessibleName('Show room list, 2 mentions and direct messages');
  await show.click();
  await expect(bar(page)).toBeInViewport();
  await expect(show).toHaveCount(0);
  await expect(pane(page).getByTestId('nav-search-field')).toBeFocused();
});
