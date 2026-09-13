import { expect, test, type Page } from '@playwright/test';

type Evidence = {
  reads: { account: number; discovery: boolean }[];
  commits: { account: number; bodies: string[]; pending: number }[];
  summaryCommits: number;
  detailCommits: number;
  decisionStarted: boolean;
  decisionCompleted: boolean;
  readCompleted: boolean;
};
const evidence = (page: Page) =>
  page.evaluate(() => (window as unknown as { lifecycleEvidence: Evidence }).lifecycleEvidence);
async function visibility(page: Page, state: 'visible' | 'hidden') {
  await page.evaluate((value) => {
    Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => value });
    document.dispatchEvent(new Event('visibilitychange'));
  }, state);
}
test.beforeEach(async ({ page }) => {
  await page.clock.install();
  await page.goto('/e2e/approvals-harness/lifecycle.html');
  await expect(page.getByTestId('request')).toHaveText('Private account 1 request: pending');
});

test('visibility events disable discovery while hidden and restore it only on the approval route', async ({
  page,
}) => {
  await expect.poll(async () => (await evidence(page)).reads.at(-1)?.discovery).toBe(true);
  await visibility(page, 'hidden');
  await expect.poll(async () => (await evidence(page)).reads.at(-1)?.discovery).toBe(false);
  await visibility(page, 'visible');
  await expect.poll(async () => (await evidence(page)).reads.at(-1)?.discovery).toBe(true);
  await page.getByRole('button', { name: 'Leave approvals' }).click();
  await expect.poll(async () => (await evidence(page)).reads.at(-1)?.discovery).toBe(false);
  const count = (await evidence(page)).reads.length;
  await visibility(page, 'hidden');
  await visibility(page, 'visible');
  await expect.poll(async () => (await evidence(page)).reads.length).toBeGreaterThan(count);
  expect((await evidence(page)).reads.slice(count).every((read) => !read.discovery)).toBe(true);
  await page.getByRole('button', { name: 'Show approvals' }).click();
  await expect.poll(async () => (await evidence(page)).reads.at(-1)?.discovery).toBe(true);
});

test('account replacement never commits private old items and ignores actual late native decisions', async ({
  page,
}) => {
  await page.getByRole('button', { name: 'Begin delayed decision' }).click();
  await expect.poll(async () => (await evidence(page)).decisionStarted).toBe(true);
  await page.getByRole('button', { name: 'Replace account' }).click();
  await expect(page.getByTestId('request')).toHaveText('Private account 2 request: pending');
  const before = await evidence(page);
  const account2Commits = before.commits.filter((commit) => commit.account === 2);
  expect(account2Commits.length).toBeGreaterThan(0);
  expect(account2Commits[0].bodies).toEqual([]);
  expect(
    account2Commits.every((commit) => !commit.bodies.includes('Private account 1 request'))
  ).toBe(true);
  await page.getByRole('button', { name: 'Complete old decision' }).click();
  await expect.poll(async () => (await evidence(page)).decisionCompleted).toBe(true);
  await expect(page.getByTestId('request')).toHaveText('Private account 2 request: pending');
  expect((await evidence(page)).reads.length).toBe(before.reads.length);
});

test('an old account list reply cannot replace the new account snapshot', async ({ page }) => {
  await page.getByRole('button', { name: 'Hold next read' }).click();
  await page.getByRole('button', { name: 'Replace account' }).click();
  await expect(page.getByTestId('request')).toHaveText('Private account 2 request: pending');
  await page.getByRole('button', { name: 'Complete old read' }).click();
  await expect.poll(async () => (await evidence(page)).readCompleted).toBe(true);
  await expect(page.getByTestId('request')).toHaveText('Private account 2 request: pending');
  expect(
    (await evidence(page)).commits
      .filter((commit) => commit.account === 2)
      .every((commit) => !commit.bodies.includes('Private account 1 request'))
  ).toBe(true);
});

test('pending age ticks update detail consumers without rerendering the summary rail', async ({
  page,
}) => {
  const before = await evidence(page);
  await page.clock.fastForward(1500);
  await expect
    .poll(async () => (await evidence(page)).detailCommits)
    .toBeGreaterThan(before.detailCommits);
  expect((await evidence(page)).summaryCommits).toBe(before.summaryCommits);
});
