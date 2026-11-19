import { expect, test, type Page } from '@playwright/test';

// Room folders on the production Home and Space panes (RoomFoldersFixture):
// 60 Home rooms, 3 DMs and a Design space, filed into Work (with a favorite
// and two Design rooms), Side projects (a mention) and an empty Reading list.
// The `matrix_room_folders_*` commands are fakes that validate like Core and
// wake the room list after each write.
const open = (page: Page, query = '') =>
  page.goto(`/e2e/room-list-harness/index.html?roomFolders${query}`);
const pane = (page: Page) => page.locator('#synara-room-list-pane');
const header = (page: Page, sectionId: string) =>
  pane(page).locator(`[data-section-id="${sectionId}"]`);
const toggle = (page: Page, sectionId: string) => header(page, sectionId).locator('button').first();
const section = (page: Page, sectionId: string) =>
  pane(page).locator(`[data-room-section="${sectionId}"]`);
const rowsOf = (page: Page, sectionId: string) =>
  section(page, sectionId)
    .locator('a[data-room-nav-link]')
    .evaluateAll((links) => links.map((link) => link.getAttribute('data-room-nav-id')));
const sectionOrder = (page: Page) =>
  pane(page)
    .locator('[data-section-id]')
    .evaluateAll((nodes) => nodes.map((node) => node.getAttribute('data-section-id')));
const roomId = (slug: string) => `!${slug}:example.test`;
const row = (page: Page, slug: string) =>
  pane(page).locator(`a[data-room-nav-link][data-room-nav-id="${roomId(slug)}"]`);
const current = (page: Page) => pane(page).locator('a[data-room-nav-link][aria-current="page"]');
const commands = async (page: Page) =>
  JSON.parse((await page.locator('body').getAttribute('data-folder-commands')) ?? '[]') as Array<{
    command: string;
    args?: Record<string, unknown>;
  }>;
const folderMenuButton = (page: Page, name: string) =>
  pane(page).getByRole('button', { name: `${name} folder options` });
const openFolderMenu = async (page: Page, sectionId: string, name: string) => {
  await header(page, sectionId).hover();
  await folderMenuButton(page, name).click();
  return page.getByTestId('room-folder-menu');
};
const openRoomMenu = async (page: Page, slug: string) => {
  await row(page, slug).click({ button: 'right' });
  await page.getByRole('button', { name: 'Move to folder' }).click();
  return page.getByTestId('room-folder-submenu');
};
const nameField = (page: Page) => page.getByTestId('room-folder-name');

test('Home lists Favorites, each folder in order, Rooms, then Direct messages', async ({
  page,
}) => {
  await open(page);
  await expect(row(page, 'general')).toBeVisible();
  expect(await sectionOrder(page)).toEqual([
    'favorites',
    'folder:work',
    'folder:side',
    'folder:reading',
    'rooms',
    'directs',
  ]);
  await expect(toggle(page, 'folder:work')).toHaveText('Work');
  // Filed rooms leave Rooms; the favorite stays in Favorites, not in Work.
  expect(await rowsOf(page, 'folder:work')).toEqual(
    ['release-train', 'incidents', 'oncall', 'frontend', 'backend', 'infra', 'security'].map(roomId)
  );
  expect(await rowsOf(page, 'favorites')).toEqual([roomId('announcements'), roomId('standup')]);
  const rooms = await rowsOf(page, 'rooms');
  expect(rooms).toContain(roomId('general'));
  expect(rooms).not.toContain(roomId('incidents'));
  expect(rooms).not.toContain(roomId('docs-2'));
  // Design rooms are not Home rooms, so Work shows none of them here.
  expect(await rowsOf(page, 'folder:work')).not.toContain(roomId('design-system'));
  // An empty folder is still listed, with one quiet line.
  await expect(section(page, 'folder:reading').locator('[data-room-folder-empty]')).toHaveText(
    'No rooms'
  );
  await expect(section(page, 'folder:reading').locator('a[data-room-nav-link]')).toHaveCount(0);
});

test('a collapsed folder shows its unread on the header and keeps unread rows', async ({
  page,
}) => {
  await open(page);
  const badge = header(page, 'folder:side').locator('[data-room-folder-badge] [data-badge-role]');
  await expect(badge).toHaveCount(0);
  await toggle(page, 'folder:side').click();
  await expect(toggle(page, 'folder:side')).toHaveAttribute('aria-expanded', 'false');
  // Mentions: the red count, as rows and the switcher show them.
  await expect(badge).toHaveAttribute('data-badge-role', 'mention');
  await expect(badge).toHaveText('2');
  await expect(toggle(page, 'folder:side')).toHaveAccessibleName('Side projects, 2 mentions');
  // Like every section, collapsed keeps the unread rows.
  expect(await rowsOf(page, 'folder:side')).toEqual([roomId('docs-2'), roomId('support-2')]);

  // Plain unread only: the quiet neutral dot, no number.
  await page.evaluate(() =>
    (
      window as unknown as { __roomFolders: { receive: (s: string, c: object) => void } }
    ).__roomFolders.receive('incidents', { highlightCount: 0, unreadCount: 4 })
  );
  await toggle(page, 'folder:work').click();
  const workBadge = header(page, 'folder:work').locator(
    '[data-room-folder-badge] [data-badge-role]'
  );
  await expect(workBadge).toHaveAttribute('data-badge-role', 'neutral');
  await expect(workBadge).toHaveText('');
  await expect(toggle(page, 'folder:work')).toHaveAccessibleName('Work, unread');

  // Open again: no badge; a read folder collapsed has none either.
  await toggle(page, 'folder:side').click();
  await expect(badge).toHaveCount(0);
  await toggle(page, 'folder:reading').click();
  await expect(header(page, 'folder:reading').locator('[data-room-folder-badge]')).toHaveCount(0);
  await expect(section(page, 'folder:reading').locator('[data-room-folder-empty]')).toHaveCount(0);
});

test('the room menu files a room, shows its folder and takes it out again', async ({ page }) => {
  await open(page);
  let submenu = await openRoomMenu(page, 'general');
  await expect(submenu.getByRole('button')).toHaveText([
    'Work',
    'Side projects',
    'Reading list',
    'New folder…',
  ]);
  await submenu.getByRole('button', { name: 'Reading list' }).click();
  await expect(submenu).toHaveCount(0);
  await expect.poll(() => rowsOf(page, 'folder:reading')).toEqual([roomId('general')]);
  await expect(section(page, 'folder:reading').locator('[data-room-folder-empty]')).toHaveCount(0);
  expect(await rowsOf(page, 'rooms')).not.toContain(roomId('general'));
  expect((await commands(page)).at(-1)).toEqual({
    command: 'matrix_room_folders_set_room',
    args: { roomId: roomId('general'), folderId: 'reading' },
  });

  submenu = await openRoomMenu(page, 'general');
  await expect(submenu.getByRole('button', { name: 'Reading list' })).toHaveAttribute(
    'aria-pressed',
    'true'
  );
  await expect(submenu.getByRole('button', { name: 'Work' })).toHaveAttribute(
    'aria-pressed',
    'false'
  );
  await submenu.getByRole('button', { name: 'Remove from folder' }).click();
  await expect.poll(() => rowsOf(page, 'rooms')).toContain(roomId('general'));
  expect((await commands(page)).at(-1)).toEqual({
    command: 'matrix_room_folders_set_room',
    args: { roomId: roomId('general'), folderId: null },
  });
});

test('direct messages offer no folders', async ({ page }) => {
  await open(page);
  await pane(page)
    .locator(`a[data-room-nav-id="${roomId('dm-bob')}"]`)
    .click({ button: 'right' });
  await expect(page.getByRole('button', { name: 'Add to favorites' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Move to folder' })).toHaveCount(0);
});

test('New folder… in the room menu creates the folder with the room in one command', async ({
  page,
}) => {
  await open(page);
  const submenu = await openRoomMenu(page, 'random');
  await submenu.getByRole('button', { name: 'New folder…' }).click();
  const dialog = page.getByRole('alertdialog', { name: 'New folder' });
  await expect(dialog).toBeVisible();
  await expect(nameField(page)).toBeFocused();
  await nameField(page).fill('  Later   on ');
  await page.keyboard.press('Enter');
  await expect(dialog).toHaveCount(0);
  expect((await commands(page)).at(-1)).toEqual({
    command: 'matrix_room_folders_create',
    args: { name: 'Later on', roomId: roomId('random') },
  });
  await expect(toggle(page, 'folder:folder-4')).toHaveText('Later on');
  expect(await rowsOf(page, 'folder:folder-4')).toEqual([roomId('random')]);
  // New folders go last, before Rooms.
  expect((await sectionOrder(page)).slice(1, 6)).toEqual([
    'folder:work',
    'folder:side',
    'folder:reading',
    'folder:folder-4',
    'rooms',
  ]);
});

test('Home options › New folder: Enter saves, Escape cancels, errors stay in the dialog', async ({
  page,
}) => {
  await open(page, '&maxFolders=4');
  const newFolder = async () => {
    await pane(page).getByRole('button', { name: 'Home options' }).click();
    await page.getByRole('button', { name: 'New folder' }).click();
    await expect(nameField(page)).toBeFocused();
  };
  await newFolder();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('alertdialog')).toHaveCount(0);
  expect(await commands(page)).toEqual([]);

  await newFolder();
  // Only spaces: the name rule, inline, before any command.
  await nameField(page).fill('   ');
  await page.keyboard.press('Enter');
  const dialog = page.getByRole('alertdialog', { name: 'New folder' });
  await expect(dialog).toContainText('Folder names need 1 to 64 characters.');
  expect(await commands(page)).toEqual([]);
  await nameField(page).fill('Ops');
  await expect(dialog).not.toContainText('Folder names need');
  await page.keyboard.press('Enter');
  await expect(dialog).toHaveCount(0);
  await expect(toggle(page, 'folder:folder-4')).toHaveText('Ops');
  await expect(section(page, 'folder:folder-4').locator('[data-room-folder-empty]')).toBeVisible();

  // The fifth folder is over this fixture's limit: Core's error, in plain copy.
  await newFolder();
  await nameField(page).fill('One more');
  await page.keyboard.press('Enter');
  await expect(dialog).toContainText('You can have up to 100 folders. Delete one to add another.');
  await expect(nameField(page)).toHaveValue('One more');
  await page.getByRole('button', { name: 'Cancel' }).click();
  await expect(dialog).toHaveCount(0);
});

test('folder header menu: rename, move up and down, delete with confirmation', async ({ page }) => {
  await open(page);
  // Rename (kebab on hover).
  let menu = await openFolderMenu(page, 'folder:work', 'Work');
  await menu.getByRole('button', { name: 'Rename' }).click();
  await expect(nameField(page)).toHaveValue('Work');
  await nameField(page).fill('Day job');
  await page.keyboard.press('Enter');
  await expect(toggle(page, 'folder:work')).toHaveText('Day job');

  // Right-click opens the same menu.
  await header(page, 'folder:side').click({ button: 'right' });
  menu = page.getByTestId('room-folder-menu');
  await expect(menu).toHaveAttribute('data-room-folder-id', 'side');
  await expect(menu.getByRole('button')).toHaveText([
    'Rename',
    'Move up',
    'Move down',
    'Delete folder',
  ]);
  await menu.getByRole('button', { name: 'Move up' }).click();
  await expect
    .poll(async () => (await sectionOrder(page)).slice(1, 4))
    .toEqual(['folder:side', 'folder:work', 'folder:reading']);
  expect((await commands(page)).at(-1)).toEqual({
    command: 'matrix_room_folders_move',
    args: { folderId: 'side', toIndex: 0 },
  });
  menu = await openFolderMenu(page, 'folder:side', 'Side projects');
  await expect(menu.getByRole('button', { name: 'Move up' })).toBeDisabled();
  await page.keyboard.press('Escape');
  menu = await openFolderMenu(page, 'folder:reading', 'Reading list');
  await expect(menu.getByRole('button', { name: 'Move down' })).toBeDisabled();
  await menu.getByRole('button', { name: 'Move up' }).click();
  await expect
    .poll(async () => (await sectionOrder(page)).slice(1, 4))
    .toEqual(['folder:side', 'folder:reading', 'folder:work']);

  // Delete asks first and says where the rooms go.
  menu = await openFolderMenu(page, 'folder:work', 'Day job');
  await menu.getByRole('button', { name: 'Delete folder' }).click();
  const confirm = page.getByRole('alertdialog', { name: 'Delete the Day job folder?' });
  await expect(confirm).toContainText('Its rooms go back to Rooms.');
  // A destructive dialog starts on Cancel.
  await expect(confirm.getByRole('button', { name: 'Cancel' })).toBeFocused();
  await confirm.getByRole('button', { name: 'Delete folder' }).click();
  await expect(confirm).toHaveCount(0);
  await expect(header(page, 'folder:work')).toHaveCount(0);
  await expect.poll(() => rowsOf(page, 'rooms')).toContain(roomId('release-train'));
});

test('a favorite stays in Favorites, keeps its folder, and returns to it', async ({ page }) => {
  await open(page);
  const submenu = await openRoomMenu(page, 'announcements');
  await expect(submenu.getByRole('button', { name: 'Work' })).toHaveAttribute(
    'aria-pressed',
    'true'
  );
  await page.keyboard.press('Escape');
  await page.keyboard.press('Escape');
  await row(page, 'announcements').click({ button: 'right' });
  await page.getByRole('button', { name: 'Remove from favorites' }).click();
  await expect.poll(() => rowsOf(page, 'folder:work')).toContain(roomId('announcements'));
  expect(await rowsOf(page, 'favorites')).toEqual([roomId('standup')]);
});

test('previous and next room walk the folder sections in display order', async ({ page }) => {
  await open(page);
  await row(page, 'standup').click();
  const composer = page.getByRole('textbox', { name: 'Composer' });
  await composer.click();
  // Favorites' last row, then the first row of Work.
  await page.keyboard.press('Alt+ArrowDown');
  await expect(current(page)).toHaveAttribute('data-room-nav-id', roomId('release-train'));
  await row(page, 'legal-2').click();
  await composer.click();
  // Side projects' last row, over the empty Reading list, into Rooms.
  await page.keyboard.press('Alt+ArrowDown');
  await expect(current(page)).toHaveAttribute('data-room-nav-id', roomId('general'));
  await page.keyboard.press('Alt+ArrowUp');
  await expect(current(page)).toHaveAttribute('data-room-nav-id', roomId('legal-2'));
  // Next unread from the top of Work: the mention in Work.
  await row(page, 'release-train').click();
  await composer.click();
  await page.keyboard.press('Alt+Shift+ArrowDown');
  await expect(current(page)).toHaveAttribute('data-room-nav-id', roomId('incidents'));
});

test('a folder of hundreds of rooms renders only what is on screen', async ({ page }) => {
  await open(page, '&manyFiled');
  await expect(toggle(page, 'folder:archive')).toHaveText('Archive');
  await section(page, 'folder:archive').scrollIntoViewIfNeeded();
  await expect(
    section(page, 'folder:archive').locator('a[data-room-nav-link]').first()
  ).toBeVisible();
  const rendered = await section(page, 'folder:archive').locator('a[data-room-nav-link]').count();
  expect(rendered).toBeGreaterThan(0);
  expect(rendered).toBeLessThan(80);
  // Keyboard navigation reaches rows on either side that are not rendered.
  await row(page, 'legal-2').click();
  const composer = page.getByRole('textbox', { name: 'Composer' });
  await composer.click();
  await page.keyboard.press('Alt+ArrowDown');
  await expect(current(page)).toHaveAttribute('data-room-nav-id', roomId('archive-000'));
  // Next unread: general, in Rooms, below all 300 filed rooms.
  await page.keyboard.press('Alt+Shift+ArrowDown');
  await expect(current(page)).toHaveAttribute('data-room-nav-id', roomId('general'));
  await expect(current(page)).toBeInViewport();
  await page.keyboard.press('Alt+ArrowUp');
  await expect(current(page)).toHaveAttribute('data-room-nav-id', roomId('archive-299'));
  await expect(current(page)).toBeInViewport();
});

test('read-only (a newer client format): the folders show, every management action hides', async ({
  page,
}) => {
  await open(page, '&readOnly');
  await expect(toggle(page, 'folder:work')).toHaveText('Work');
  await header(page, 'folder:work').hover();
  await expect(folderMenuButton(page, 'Work')).toHaveCount(0);
  await header(page, 'folder:work').click({ button: 'right' });
  await expect(page.getByTestId('room-folder-menu')).toHaveCount(0);
  await row(page, 'general').click({ button: 'right' });
  await expect(page.getByRole('button', { name: 'Add to favorites' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Move to folder' })).toHaveCount(0);
  await page.keyboard.press('Escape');
  await pane(page).getByRole('button', { name: 'Home options' }).click();
  await expect(page.getByRole('button', { name: 'Mark as read' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'New folder' })).toHaveCount(0);
});

test('a failed filing stays in the submenu with Core’s reason', async ({ page }) => {
  await open(page, '&tooLarge');
  const submenu = await openRoomMenu(page, 'general');
  await submenu.getByRole('button', { name: 'Work' }).click();
  await expect(submenu.getByRole('alert')).toHaveText(
    'Your folders are too large to save. Take some rooms out of folders and try again.'
  );
  expect(await rowsOf(page, 'rooms')).toContain(roomId('general'));
});

test('a space lists its rooms of each folder after Favorites, and only those folders', async ({
  page,
}) => {
  await open(page, '&space');
  await expect(pane(page).getByRole('link', { name: 'Lobby' })).toBeVisible();
  await expect(row(page, 'brand')).toBeVisible();
  // Work has rooms here; Reading list holds no rooms anywhere, so it shows too.
  // Side projects has rooms, but none in this space, so it stays out.
  expect(await sectionOrder(page)).toEqual(['folder:work', 'folder:reading', 'rooms']);
  expect(await rowsOf(page, 'folder:work')).toEqual([
    roomId('design-system'),
    roomId('ux-research'),
  ]);
  const spaceRooms = await pane(page)
    .getByTestId('space-room-list')
    .locator('a[data-room-nav-link]')
    .evaluateAll((links) => links.map((link) => link.getAttribute('data-room-nav-id')));
  expect(spaceRooms).toEqual(['brand', 'icons', 'motion', 'critique'].map(roomId));
  // Collapse ids are per view: the space's Work, not Home's.
  await toggle(page, 'folder:work').click();
  await expect(toggle(page, 'folder:work')).toHaveAttribute('aria-expanded', 'false');
  expect(await rowsOf(page, 'folder:work')).toEqual([]);
  // Filing from the space's list works the same.
  const submenu = await openRoomMenu(page, 'icons');
  await submenu.getByRole('button', { name: 'Side projects' }).click();
  await expect
    .poll(() => sectionOrder(page))
    .toEqual(['folder:work', 'folder:side', 'folder:reading', 'rooms']);
  expect(await rowsOf(page, 'folder:side')).toEqual([roomId('icons')]);
});

test("a space's options menu creates a folder, which shows there while it is empty", async ({
  page,
}) => {
  await open(page, '&space');
  await expect(row(page, 'brand')).toBeVisible();
  await pane(page)
    .getByRole('button', { name: /options$/ })
    .first()
    .click();
  await page.getByRole('button', { name: 'New folder' }).click();
  const dialog = page.getByRole('alertdialog', { name: 'New folder' });
  await dialog.getByRole('textbox').fill('Launch');
  await page.keyboard.press('Enter');
  await expect(dialog).toHaveCount(0);
  await expect.poll(() => sectionOrder(page)).toContain('folder:work');
  // Reading list and the new folder are both empty.
  await expect(pane(page).locator('[data-room-folder-empty]')).toHaveCount(2);
  await expect(pane(page).getByRole('button', { name: 'Launch folder options' })).toHaveCount(1);
});

for (const theme of ['dark', 'light'] as const) {
  test(`folder headers match the other section headers (${theme})`, async ({ page }) => {
    await open(page, theme === 'light' ? '&light' : '');
    await expect(row(page, 'general')).toBeVisible();
    const style = (sectionId: string) =>
      header(page, sectionId)
        .locator('[data-section-label]')
        .evaluate((el) => {
          const css = getComputedStyle(el);
          return [css.color, css.fontSize, css.fontWeight].join(' ');
        });
    expect(await style('folder:work')).toBe(await style('rooms'));
    const empty = section(page, 'folder:reading').locator('[data-room-folder-empty]');
    const [emptyColor, background] = await empty.evaluate((el) => {
      let node: HTMLElement | null = el;
      let bg = 'rgba(0, 0, 0, 0)';
      while (node && bg === 'rgba(0, 0, 0, 0)') {
        bg = getComputedStyle(node).backgroundColor;
        node = node.parentElement;
      }
      return [getComputedStyle(el).color, bg];
    });
    expect(emptyColor).not.toBe(background);
  });
}
