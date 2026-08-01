import assert from 'node:assert/strict';
import test from 'node:test';
import {
  clearCompletedLaterItems,
  completeLaterItem,
  getLaterDueSummary,
  getLaterItemId,
  getSortedLaterItems,
  normalizeLaterContent,
  putLaterItem,
  removeLaterItem,
  snoozeLaterItem,
} from '../later';

test('later items use stable room/event anchors', () => {
  assert.equal(getLaterItemId('!room:example.org', '$event'), '!room:example.org\n$event');
});

test('putLaterItem, removeLaterItem, and sorting preserve the latest queue state', () => {
  const now = 1_000;
  const saved = {
    id: 'saved',
    kind: 'saved' as const,
    roomId: '!room',
    eventId: '$saved',
    createdAt: now,
  };
  const reminder = {
    id: 'reminder',
    kind: 'reminder' as const,
    roomId: '!room',
    eventId: '$reminder',
    createdAt: now + 1,
    dueTs: now - 1,
  };

  const content = putLaterItem(putLaterItem(undefined, saved), reminder);
  assert.deepEqual(
    getSortedLaterItems(content, now).map((item) => item.id),
    ['reminder', 'saved']
  );
  assert.deepEqual(Object.keys(removeLaterItem(content, 'reminder').items ?? {}), ['saved']);
});

test('later item ids remain room/event anchors without plaintext fields', () => {
  const item = {
    id: getLaterItemId('!room:example.org', '$event'),
    kind: 'reminder' as const,
    roomId: '!room:example.org',
    eventId: '$event',
    createdAt: 1,
    dueTs: 2_000,
  };

  assert.equal(item.id, '!room:example.org\n$event');
  assert.equal('sender' in item, false);
  assert.equal('body' in item, false);
  assert.equal(item.dueTs, 2_000);
});

test('normalizeLaterContent strips legacy plaintext preview fields', () => {
  const content = normalizeLaterContent({
    items: {
      legacy: {
        id: 'legacy',
        kind: 'saved',
        roomId: '!room',
        eventId: '$event',
        createdAt: 1,
        body: 'do not keep this',
        sender: '@alice:example.org',
      } as any,
    },
  });

  assert.deepEqual(content.items?.legacy, {
    id: 'legacy',
    kind: 'saved',
    roomId: '!room',
    eventId: '$event',
    createdAt: 1,
  });
});

test('normalizeLaterContent drops malformed items and non-finite optional timestamps', () => {
  const content = normalizeLaterContent({
    version: 999,
    items: {
      valid: {
        id: 'valid',
        kind: 'reminder',
        roomId: '!room',
        eventId: '$event',
        createdAt: 1,
        dueTs: 2,
        remindedAt: Number.POSITIVE_INFINITY,
        completedAt: Number.NaN,
      } as any,
      badKind: {
        id: 'badKind',
        kind: 'todo',
        roomId: '!room',
        eventId: '$event',
        createdAt: 1,
      } as any,
      missingEvent: {
        id: 'missingEvent',
        kind: 'saved',
        roomId: '!room',
        createdAt: 1,
      } as any,
      badCreatedAt: {
        id: 'badCreatedAt',
        kind: 'saved',
        roomId: '!room',
        eventId: '$event',
        createdAt: '1',
      } as any,
    },
  });

  assert.equal(content.version, 1);
  assert.deepEqual(Object.keys(content.items ?? {}), ['valid']);
  assert.deepEqual(content.items?.valid, {
    id: 'valid',
    kind: 'reminder',
    roomId: '!room',
    eventId: '$event',
    createdAt: 1,
    dueTs: 2,
  });
});

test('later helpers complete, snooze, clear completed, and summarize due items', () => {
  const now = new Date('2026-05-07T12:00:00Z').getTime();
  const base = putLaterItem(
    putLaterItem(undefined, {
      id: 'saved',
      kind: 'saved',
      roomId: '!room',
      eventId: '$saved',
      createdAt: now,
    }),
    {
      id: 'reminder',
      kind: 'reminder',
      roomId: '!room',
      eventId: '$reminder',
      createdAt: now + 1,
      dueTs: now - 1,
    }
  );

  const snoozed = snoozeLaterItem(base, 'reminder', now + 60_000);
  assert.equal(snoozed.items?.reminder.dueTs, now + 60_000);
  assert.equal(snoozed.items?.reminder.remindedAt, undefined);

  const completed = completeLaterItem(snoozed, 'saved', now + 2);
  assert.equal(completed.items?.saved.completedAt, now + 2);
  assert.deepEqual(getLaterDueSummary(completed, now), {
    active: 1,
    completed: 1,
    overdue: 0,
    dueToday: 1,
  });
  assert.deepEqual(Object.keys(clearCompletedLaterItems(completed).items ?? {}), ['reminder']);
});
