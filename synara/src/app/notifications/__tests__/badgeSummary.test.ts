import test from 'node:test';
import assert from 'node:assert/strict';
import { getBadgeCount, summarizeBadgeCount, summarizeNotifications } from '../badgeSummary';

test('summarizeNotifications describes app and inbox notification counts', () => {
  assert.deepEqual(
    summarizeNotifications({
      unreadCounts: [{ total: 4, highlight: 2 }, { total: 3 }],
      laterActiveCount: 5,
      inviteCount: 2,
      agentApprovalCount: 1,
    }),
    {
      appBadgeCount: 10,
      inboxBadgeCount: 8,
      laterActiveCount: 5,
      inviteCount: 2,
      agentApprovalCount: 1,
      highlightCount: 2,
      unreadCount: 3,
    }
  );
});

test('summarizeBadgeCount combines highlights, unread totals, and Later items', () => {
  assert.deepEqual(
    summarizeBadgeCount(
      [{ total: 4, highlight: 2 }, { total: 3 }, { total: 9, highlight: 0 }, { total: 0 }],
      5
    ),
    {
      count: 10,
      laterActiveCount: 5,
      highlightCount: 2,
      unreadCount: 3,
    }
  );
});

test('summarizeBadgeCount clamps negative and fractional values', () => {
  assert.deepEqual(summarizeBadgeCount([{ total: -1, highlight: -2 }, { total: 3.9 }], 2.8), {
    count: 5,
    laterActiveCount: 2,
    highlightCount: 0,
    unreadCount: 3,
  });
});

test('summarizeNotifications clamps invite and agent approval counts', () => {
  assert.deepEqual(
    summarizeNotifications({
      unreadCounts: [{ total: -1, highlight: -2 }, { total: 3.9 }],
      laterActiveCount: 2.8,
      inviteCount: -1,
      agentApprovalCount: Number.NaN,
    }),
    {
      appBadgeCount: 5,
      inboxBadgeCount: 2,
      laterActiveCount: 2,
      inviteCount: 0,
      agentApprovalCount: 0,
      highlightCount: 0,
      unreadCount: 3,
    }
  );
});

test('getBadgeCount returns the aggregate count for platform adapters', () => {
  assert.equal(getBadgeCount([{ total: 1 }, { highlight: 4 }], 2), 7);
});
