import { expect, test, type Page } from '@playwright/test';

const route = '/e2e/approvals-harness/index.html?questions';

const answers = (page: Page) =>
  page.evaluate(
    () => (window as unknown as { synaraQuestionAnswers: unknown[] }).synaraQuestionAnswers
  );

const card = (page: Page, name: string) =>
  page.getByRole('region', { name, exact: true }).getByRole('group', {
    name: 'Question from the agent',
  });

test('single choice sends the chosen index once and settles without an alert', async ({ page }) => {
  await page.goto(route);
  const single = card(page, 'Single choice');
  await expect(single.getByText('Which colour should the cover use?')).toBeVisible();
  await expect(single.getByRole('button', { name: 'Red (recommended)' })).toBeVisible();
  await single.getByRole('button', { name: 'Blue', exact: true }).click();
  await expect(single.getByRole('status')).toHaveText('Answered: Blue');
  await expect(single.getByRole('button')).toHaveCount(0);
  await expect(single.getByRole('alert')).toHaveCount(0);
  expect(await answers(page)).toEqual([
    {
      roomId: '!book:example.test',
      eventId: '$structured-single',
      answer: { kind: 'choice', index: 2 },
    },
  ]);
});

test('multi-select toggles choices, then sends them together', async ({ page }) => {
  await page.goto(route);
  const multi = card(page, 'Multiple choice');
  const send = multi.getByRole('button', { name: 'Send selected answers' });
  await expect(send).toBeDisabled();
  await multi.getByRole('button', { name: 'Print', exact: true }).click();
  await multi.getByRole('button', { name: 'PDF (recommended)' }).click();
  await expect(multi.getByRole('button', { name: 'Print', exact: true })).toHaveAttribute(
    'aria-pressed',
    'true'
  );
  await send.click();
  await expect(multi.getByRole('status')).toHaveText('Answered: PDF, Print');
  expect(await answers(page)).toEqual([
    {
      roomId: '!book:example.test',
      eventId: '$structured-multi',
      answer: { kind: 'choices', indices: [1, 3] },
    },
  ]);
});

test('other text and skip go through the same owner route', async ({ page }) => {
  await page.goto(route);
  const fallback = card(page, 'Text fallback');
  await fallback.getByRole('textbox', { name: 'Your own answer' }).fill('  MySQL  ');
  await fallback.getByRole('textbox', { name: 'Your own answer' }).press('Enter');
  await expect(fallback.getByRole('status')).toHaveText('Answered: MySQL');
  const single = card(page, 'Single choice');
  await single.getByRole('button', { name: 'Skip', exact: true }).click();
  await expect(single.getByRole('status')).toHaveText('Skipped');
  expect(await answers(page)).toEqual([
    {
      roomId: '!book:example.test',
      eventId: '$fallback-card',
      answer: { kind: 'other', text: 'MySQL' },
    },
    { roomId: '!book:example.test', eventId: '$structured-single', answer: { kind: 'skip' } },
  ]);
});

test('read-only, settled, expired and replied cards offer no buttons', async ({ page }) => {
  await page.goto(route);
  const foreign = card(page, 'Someone else');
  await expect(foreign.getByText('This question is for someone else.')).toBeVisible();
  await expect(foreign.getByRole('listitem')).toHaveCount(3);
  await expect(foreign.getByRole('button')).toHaveCount(0);
  await expect(card(page, 'Settled').getByRole('status')).toHaveText('Answered: Blue');
  await expect(card(page, 'Expired').getByRole('status')).toHaveText('No answer (expired)');
  await expect(card(page, 'Answered by reply').getByRole('status')).toHaveText(
    'Answered: Red, Green'
  );
  for (const name of ['Settled', 'Expired', 'Answered by reply']) {
    await expect(card(page, name).getByRole('button')).toHaveCount(0);
  }
  const open = card(page, 'Open question');
  await expect(open.getByRole('textbox', { name: 'Your own answer' })).toBeVisible();
  await expect(open.getByRole('button', { name: 'Skip' })).toHaveCount(0);
});

test('a failed answer shows an error and keeps the card answerable', async ({ page }) => {
  await page.goto(`${route}&questionError`);
  const single = card(page, 'Single choice');
  await single.getByRole('button', { name: 'Blue', exact: true }).click();
  await expect(single.getByText('Your answer could not be sent. Try again.')).toBeVisible();
  await expect(single.getByRole('button', { name: 'Blue', exact: true })).toBeEnabled();
});
