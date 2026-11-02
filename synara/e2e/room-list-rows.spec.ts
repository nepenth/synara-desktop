import { expect, test, type Locator, type Page } from '@playwright/test';

const open = (page: Page, light = false) =>
  page.goto(`/e2e/room-list-harness/index.html?roomRows${light ? '&theme=light' : ''}`);
const row = (page: Page, roomId: string) => page.locator(`[data-room-id="${roomId}"]`);
const nameOf = (r: Locator) => r.getByRole('link');
const nameText = (r: Locator) => r.locator('[data-room-name]');
const weight = (r: Locator) => nameText(r).evaluate((el) => getComputedStyle(el).fontWeight);
const color = (r: Locator) => nameText(r).evaluate((el) => getComputedStyle(el).color);
const cssVar = (page: Page, name: string) =>
  page.evaluate(
    (variable) =>
      getComputedStyle(document.querySelector('main')!).getPropertyValue(variable).trim(),
    name
  );
const toRgb = (hex: string) => {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
  return `rgb(${r}, ${g}, ${b})`;
};
/** The NavItem surface of a fixture row. */
const surface = (r: Locator) => r.locator('[data-highlight]').first();
const marker = (r: Locator) =>
  surface(r).evaluate((el) => {
    const style = getComputedStyle(el, '::before');
    return {
      content: style.content,
      width: style.width,
      height: style.height,
      background: style.backgroundColor,
      left: style.left,
    };
  });
const sectionLabel = (header: Locator) => header.locator('[data-section-label]');

test('a 56-room list plus DMs keeps 32px rows, 15px names, 600 unread and 400 read', async ({
  page,
}) => {
  await open(page);
  const rows = page.getByTestId('room-row');
  await expect(rows).toHaveCount(58);
  const heights = await rows.evaluateAll((nodes) =>
    nodes.map((node) => Math.round(node.getBoundingClientRect().height))
  );
  expect(heights).toEqual(heights.map(() => 32));

  const unread = row(page, '!unread:example.test');
  const read = row(page, '!room-3:example.test');
  await expect(nameText(unread)).toHaveCSS('font-size', '15px');
  await expect(nameText(unread)).toHaveCSS('line-height', '20px');
  expect(await weight(unread)).toBe('600');
  expect(await weight(read)).toBe('400');
  // Plain unread carries no count badge: weight, colour and the marker do.
  await expect(unread.locator('[data-badge-role]')).toHaveCount(0);
  await expect(read.locator('[data-badge-role]')).toHaveCount(0);

  // Reading roles: unread name primary with a secondary glyph; read name
  // secondary with a tertiary glyph.
  const primary = toRgb(await cssVar(page, '--synara-content-primary'));
  const secondary = toRgb(await cssVar(page, '--synara-content-secondary'));
  const tertiary = toRgb(await cssVar(page, '--synara-content-tertiary'));
  expect(await color(unread)).toBe(primary);
  expect(await color(read)).toBe(secondary);
  await expect(unread.locator('[data-room-glyph]')).toHaveCSS('color', secondary);
  await expect(read.locator('[data-room-glyph]')).toHaveCSS('color', tertiary);

  // Section headers: 28px captions, 12px semibold secondary with tracking.
  const header = page.getByRole('button', { name: 'Rooms', exact: true });
  const box = await header.boundingBox();
  expect(Math.round(box!.height)).toBe(28);
  await expect(sectionLabel(header)).toHaveCSS('font-size', '12px');
  await expect(sectionLabel(header)).toHaveCSS('font-weight', '600');
  await expect(sectionLabel(header)).toHaveCSS('letter-spacing', '0.24px');
  await expect(sectionLabel(header)).toHaveCSS('color', secondary);
});

test('section headers show the chevron at rest, left of the label, and space the groups', async ({
  page,
}) => {
  await open(page);
  const rooms = page.getByRole('button', { name: 'Rooms', exact: true });
  const favorites = page.getByRole('button', { name: 'Favorites', exact: true });
  await page.mouse.move(700, 700);
  const chevron = rooms.locator('svg');
  await expect(chevron).toBeVisible();
  await expect(chevron).toHaveCSS('opacity', '1');
  const chevronBox = (await chevron.boundingBox())!;
  const labelBox = (await sectionLabel(rooms).boundingBox())!;
  expect(chevronBox.x + chevronBox.width).toBeLessThanOrEqual(labelBox.x);
  // The label starts where the room names start.
  const nameBox = (await nameText(row(page, '!room-3:example.test')).boundingBox())!;
  expect(Math.abs(labelBox.x - nameBox.x)).toBeLessThanOrEqual(1);
  // The first header has no gap above it; the others get S500 (20px).
  await expect(favorites.locator('xpath=..')).toHaveCSS('margin-top', '0px');
  await expect(rooms.locator('xpath=..')).toHaveCSS('margin-top', '20px');
  // The sort control stays hover-revealed while the chevron is always shown.
  const sort = page.getByRole('button', { name: 'Sort Rooms' }).locator('xpath=..');
  await expect(sort).toHaveCSS('opacity', '0');
  await rooms.hover();
  await expect(sort).toHaveCSS('opacity', '1');
  // Collapsing turns the chevron.
  await expect(chevron).toHaveCSS('transform', 'none');
  await rooms.click();
  await expect(chevron).not.toHaveCSS('transform', 'none');
  await expect(chevron).toBeVisible();
});

test('unread rows carry a 3×16 gutter marker; read and muted rows do not', async ({ page }) => {
  await open(page);
  const primary = toRgb(await cssVar(page, '--synara-content-primary'));
  for (const id of ['!unread', '!mention', '!dm', '!dm-mention']) {
    const m = await marker(row(page, `${id}:example.test`));
    expect(m.content, id).not.toBe('none');
    expect(m.width, id).toBe('3px');
    expect(m.height, id).toBe('16px');
    expect(m.background, id).toBe(primary);
    expect(m.left, id).toBe('0px');
  }
  for (const id of ['!room-3', '!muted', '!dm-read', '!dm-muted', '!fav']) {
    expect((await marker(row(page, `${id}:example.test`))).content, id).toBe('none');
  }
  // The marker sits in the padding: unread and read names start at one x.
  const unreadX = (await nameText(row(page, '!unread:example.test')).boundingBox())!.x;
  const readX = (await nameText(row(page, '!room-3:example.test')).boundingBox())!.x;
  expect(unreadX).toBe(readX);
});

test('mentions and DMs get the red count; it survives hover and caps at 99+', async ({ page }) => {
  await open(page);
  const mention = row(page, '!mention:example.test');
  const badge = mention.locator('[data-badge-role="mention"]');
  await expect(badge).toHaveText('2');
  const critical = await badge.evaluate((el) => getComputedStyle(el).backgroundColor);
  // Dark theme Critical.Main is #E69D9D.
  expect(critical).toBe('rgb(230, 157, 157)');

  // Hovering reveals the kebab beside the badge; the badge stays mounted.
  await mention.hover();
  await expect(badge).toBeVisible();
  await expect(badge).toHaveText('2');
  const kebab = mention.getByRole('button', { name: 'More options' });
  await expect(kebab).toHaveCSS('opacity', '1');
  await expect(kebab).toBeVisible();
  // At rest the kebab slot is still laid out (no reflow) but invisible.
  await page.mouse.move(700, 700);
  const resting = mention.getByRole('button', { name: 'More options' });
  await expect(resting).toBeAttached();
  const slot = resting.locator('xpath=..');
  await expect(slot).toHaveCSS('opacity', '0');

  const dm = row(page, '!dm:example.test');
  await expect(dm.locator('[data-badge-role="mention"]')).toHaveText('4');
  await expect(row(page, '!dm-read:example.test').locator('[data-badge-role]')).toHaveCount(0);
  await expect(row(page, '!many:example.test').locator('[data-badge-role="mention"]')).toHaveText(
    '99+'
  );
});

test('muted, private and favourite rooms are distinguishable at rest', async ({ page }) => {
  await open(page);
  const muted = row(page, '!muted:example.test');
  const tertiary = await cssVar(page, '--synara-content-tertiary');
  const secondary = await cssVar(page, '--synara-content-secondary');
  expect(tertiary).not.toBe('');
  expect(await color(muted)).toBe(toRgb(tertiary));
  await expect(muted.locator('[data-room-glyph]')).toHaveCSS('color', toRgb(tertiary));
  expect(await weight(muted)).toBe('400');
  await expect(muted.locator('[data-room-state="muted"]')).toHaveCount(1);
  await expect(muted.locator('[data-badge-role]')).toHaveCount(0);
  expect(await color(row(page, '!room-3:example.test'))).toBe(toRgb(secondary));

  const priv = row(page, '!private:example.test');
  await expect(priv.locator('[data-room-glyph="lock"]')).toHaveCount(1);
  await expect(priv.locator('[data-room-glyph="lock"] svg')).toHaveCount(1);
  await expect(priv).not.toContainText('#');
  await expect(row(page, '!room-3:example.test').locator('[data-room-glyph="hash"]')).toHaveText(
    '#'
  );
  const lockBox = await priv.locator('[data-room-glyph="lock"]').boundingBox();
  expect(Math.round(lockBox!.width)).toBe(18);

  // Favourites: star marker in lists that ask for it; the Favorites section
  // row itself does not repeat the star.
  await expect(row(page, '!fav:example.test').locator('[data-room-state="favorite"]')).toHaveCount(
    0
  );
  await expect(page.getByRole('button', { name: 'Favorites', exact: true })).toBeVisible();

  // Long names truncate inside the row instead of growing it.
  const long = row(page, '!long:example.test');
  expect(Math.round((await long.boundingBox())!.height)).toBe(32);
  await expect(long.locator('[data-badge-role="mention"]')).toHaveText('1');
});

test('the selected row is anchored: Primary container fill, on-container name, stronger than hover', async ({
  page,
}) => {
  await open(page);
  const fav = row(page, '!fav:example.test');
  const item = fav.locator('[data-selected="true"]');
  await expect(item).toHaveCount(1);
  await expect(item).toHaveCSS('box-shadow', 'none');
  expect(await weight(fav)).toBe('600');
  // Dark Primary.Container #1F3A63, OnContainer #DCE7FB.
  await expect(item).toHaveCSS('background-color', 'rgb(31, 58, 99)');
  expect(await color(fav)).toBe('rgb(220, 231, 251)');
  await expect(fav.locator('[data-room-glyph]')).toHaveCSS('color', 'rgb(220, 231, 251)');
  // No accent bar or unread marker on a read selected row.
  expect((await marker(fav)).content).toBe('none');

  // Hover is a neutral tint, different from (weaker than) the selected fill.
  const hoverRow = row(page, '!room-5:example.test');
  const hoverTarget = surface(hoverRow);
  await hoverTarget.hover();
  await expect(hoverTarget).toHaveCSS('box-shadow', 'none');
  await expect(hoverTarget).toHaveCSS('border-top-color', 'rgba(0, 0, 0, 0)');
  const hoverBg = await hoverTarget.evaluate((el) => getComputedStyle(el).backgroundColor);
  expect(hoverBg).not.toBe('rgb(31, 58, 99)');
  expect(hoverBg).not.toBe('rgba(0, 0, 0, 0)');

  // Selected + hover keeps the selected fill.
  await item.hover();
  await expect(item).toHaveCSS('background-color', 'rgb(31, 58, 99)');

  // Selecting another row moves the fill.
  await nameOf(hoverRow).click();
  await expect(hoverRow.locator('[data-selected="true"]')).toHaveCount(1);
  await expect(item).toHaveCount(0);
});

test('long room names truncate in the middle; the pane title ends in an ellipsis', async ({
  page,
}) => {
  await open(page);
  // [hidden full name, visual start, visual end]
  const spans = (r: Locator) => r.locator('[data-middle-truncated="true"] > span');
  const project = row(page, '!home-mgmt:example.test');
  await expect(spans(project)).toHaveCount(3);
  // Assistive tech gets the full name once; the visual spans are hidden from it.
  await expect(spans(project).nth(0)).toHaveText('Project - Home Management');
  await expect(spans(project).nth(1)).toHaveAttribute('aria-hidden', 'true');
  await expect(spans(project).nth(2)).toHaveAttribute('aria-hidden', 'true');
  await expect(nameOf(project)).toHaveAccessibleName(/^Project - Home Management/);

  // An overflowing start clips under a fade; the ellipsis leads the tail,
  // flush against the start, so there is no gap before it.
  const long = row(page, '!long:example.test');
  const [start, end] = [spans(long).nth(1), spans(long).nth(2)];
  await expect(end).toHaveText('…on pane');
  await expect(end).toBeVisible();
  await expect(start).toHaveAttribute('data-overflowing', 'true');
  expect(await start.evaluate((el) => el.scrollWidth > el.clientWidth)).toBe(true);
  await expect(start).toHaveCSS('text-overflow', 'clip');
  expect(await start.evaluate((el) => getComputedStyle(el).webkitMaskImage)).toContain(
    'linear-gradient'
  );
  const startBox = (await start.boundingBox())!;
  const endBox = (await end.boundingBox())!;
  const rowBox = (await long.boundingBox())!;
  expect(Math.abs(endBox.x - (startBox.x + startBox.width))).toBeLessThanOrEqual(1);
  expect(endBox.x + endBox.width).toBeLessThanOrEqual(rowBox.x + rowBox.width);

  // Short names are one span.
  await expect(
    row(page, '!unread:example.test').locator('[data-middle-truncated="false"]')
  ).toHaveCount(1);

  // The pane title: 15px semibold, whole text with an ordinary end ellipsis.
  const title = page.getByTestId('nav-pane-title');
  await expect(title).toHaveCSS('font-size', '15px');
  await expect(title).toHaveCSS('font-weight', '600');
  await expect(title).toHaveText('matrix.whyland.net');
  await expect(title).toHaveCSS('text-overflow', 'ellipsis');
  await expect(title.locator('[data-middle-truncated]')).toHaveCount(0);
});

test('DM rows show the peer presence from Core: online, offline, or nothing when unknown', async ({
  page,
}) => {
  await open(page);
  const dot = (id: string) => row(page, `${id}:example.test`).locator('[data-presence]');
  await expect(dot('!dm')).toHaveAttribute('data-presence', 'online');
  // Dark Success.Main #85E0BA.
  await expect(dot('!dm')).toHaveCSS('background-color', 'rgb(133, 224, 186)');
  const box = (await dot('!dm').boundingBox())!;
  expect(Math.round(box.width)).toBe(8);
  expect(Math.round(box.height)).toBe(8);
  await expect(dot('!dm-read')).toHaveAttribute('data-presence', 'offline');
  await expect(dot('!dm-read')).toHaveCSS(
    'background-color',
    toRgb(await cssVar(page, '--synara-content-tertiary'))
  );
  await expect(dot('!dm-muted')).toHaveAttribute('data-presence', 'offline');
  // The dot is aria-hidden; its meaning is in the row's accessible name.
  await expect(dot('!dm')).toHaveAttribute('aria-hidden', 'true');
  await expect(nameOf(row(page, '!dm:example.test'))).toHaveAccessibleName(
    /Alice Liddell\s*, Online/
  );
  await expect(nameOf(row(page, '!dm-read:example.test'))).toHaveAccessibleName(/Bob\s*, Offline/);
  await expect(row(page, '!dm-mention:example.test').locator('[data-presence-label]')).toHaveCount(
    0
  );
  // Carol's presence is unknown: no dot.
  await expect(dot('!dm-mention')).toHaveCount(0);
  // Rooms never get one.
  await expect(row(page, '!room-3:example.test').locator('[data-presence]')).toHaveCount(0);
  // The dot sits on the glyph's bottom-right.
  const glyph = (await row(page, '!dm:example.test')
    .locator('[data-presence]')
    .locator('..')
    .boundingBox())!;
  expect(box.x + box.width / 2).toBeGreaterThan(glyph.x + glyph.width / 2);
  expect(box.y + box.height / 2).toBeGreaterThan(glyph.y + glyph.height / 2);
});

test('collapsing Rooms keeps unread and selected rows visible', async ({ page }) => {
  await open(page);
  await page.getByRole('button', { name: 'Rooms', exact: true }).click();
  const rows = page.getByTestId('room-row');
  // 1 favourite + mention, unread, private-unread, long, hosting, many (muted
  // and read rows hide) + the 4 DMs, whose section stays open.
  await expect(rows).toHaveCount(11);
  await expect(row(page, '!muted:example.test')).toHaveCount(0);
  await expect(row(page, '!unread:example.test')).toHaveCount(1);
});

test('Direct messages is the last section of the one list, with DM badges (NAV-12)', async ({
  page,
}) => {
  await open(page);
  const favorites = page.getByRole('button', { name: 'Favorites', exact: true });
  const rooms = page.getByRole('button', { name: 'Rooms', exact: true });
  const directs = page.getByRole('button', { name: 'Direct messages', exact: true });
  // Sentence case, same 28px / 12px semibold header as the other sections.
  await expect(directs).toHaveText('Direct messages');
  expect(Math.round((await directs.boundingBox())!.height)).toBe(28);
  await expect(sectionLabel(directs)).toHaveCSS('font-size', '12px');
  await expect(sectionLabel(directs)).toHaveCSS('font-weight', '600');
  const top = async (header: typeof directs) => (await header.boundingBox())!.y;
  expect(await top(favorites)).toBeLessThan(await top(rooms));
  expect(await top(rooms)).toBeLessThan(await top(directs));
  // Every DM row sits below the header, after every room row.
  const lastRoom = (await row(page, '!room-43:example.test').boundingBox())!;
  for (const id of ['!dm', '!dm-read', '!dm-mention', '!dm-muted']) {
    const box = (await row(page, `${id}:example.test`).boundingBox())!;
    expect(box.y).toBeGreaterThan(await top(directs));
    expect(box.y).toBeGreaterThan(lastRoom.y);
    expect(Math.round(box.height)).toBe(32);
  }

  // The hover-revealed sort sits in the header, as for Rooms.
  await directs.hover();
  await expect(page.getByRole('button', { name: 'Sort Direct messages' })).toBeVisible();

  // Same row rules as rooms: an unread DM and a DM mention get the red count;
  // a read DM and a muted DM get none, the muted one keeps its glyph.
  await expect(row(page, '!dm:example.test').locator('[data-badge-role="mention"]')).toHaveText(
    '4'
  );
  await expect(
    row(page, '!dm-mention:example.test').locator('[data-badge-role="mention"]')
  ).toHaveText('1');
  expect(await weight(row(page, '!dm-mention:example.test'))).toBe('600');
  await expect(row(page, '!dm-read:example.test').locator('[data-badge-role]')).toHaveCount(0);
  expect(await weight(row(page, '!dm-read:example.test'))).toBe('400');
  const muted = row(page, '!dm-muted:example.test');
  await expect(muted.locator('[data-badge-role]')).toHaveCount(0);
  await expect(muted.locator('[data-room-state="muted"]')).toHaveCount(1);
  // DM rows link to their own route.
  await expect(nameOf(row(page, '!dm:example.test'))).toHaveAttribute(
    'href',
    '/direct/!dm:example.test/'
  );

  // Collapsing keeps the unread DMs only; expanding brings the rest back.
  await directs.click();
  await expect(directs).toHaveAttribute('aria-expanded', 'false');
  await expect(row(page, '!dm:example.test')).toHaveCount(1);
  await expect(row(page, '!dm-mention:example.test')).toHaveCount(1);
  await expect(row(page, '!dm-read:example.test')).toHaveCount(0);
  await expect(row(page, '!dm-muted:example.test')).toHaveCount(0);
  await directs.click();
  await expect(directs).toHaveAttribute('aria-expanded', 'true');
  await expect(page.getByTestId('room-row')).toHaveCount(58);
});

test('light theme keeps the same roles', async ({ page }) => {
  await open(page, true);
  const badge = row(page, '!mention:example.test').locator('[data-badge-role="mention"]');
  // Light theme Critical.Main is #9D0F0F with white digits.
  await expect(badge).toHaveCSS('background-color', 'rgb(157, 15, 15)');
  await expect(badge).toHaveCSS('color', 'rgb(255, 255, 255)');
  // Light Primary.Container #C4D0E9 with OnContainer #0D3076.
  const fav = row(page, '!fav:example.test');
  await expect(fav.locator('[data-selected="true"]')).toHaveCSS(
    'background-color',
    'rgb(196, 208, 233)'
  );
  expect(await color(fav)).toBe('rgb(13, 48, 118)');
  const m = await marker(row(page, '!unread:example.test'));
  expect(m.background).toBe(toRgb(await cssVar(page, '--synara-content-primary')));
  // The search field opens the global search: "Search" plus its shortcut.
  const field = page.getByTestId('nav-search-field');
  await expect(field).toContainText('Search');
  await expect(field).toHaveAttribute('aria-haspopup', 'dialog');
});
