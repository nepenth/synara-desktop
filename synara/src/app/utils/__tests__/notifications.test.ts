import assert from 'node:assert/strict';
import test from 'node:test';
import { AccountDataEvent } from '../../../types/matrix/accountData';
import { clearUnreadAnchor } from '../notifications';
import { getThreadRootEventId } from '../room';

test('clearUnreadAnchor skips account-data writes when the room has no anchor', async () => {
  let writes = 0;
  const mx = {
    getAccountData: () => ({
      getContent: () => ({
        version: 1,
        anchors: {
          '!other:example.org': {
            eventId: '$other',
            ts: 1,
          },
        },
      }),
    }),
    setAccountData: async () => {
      writes += 1;
    },
  } as any;

  await clearUnreadAnchor(mx, '!room:example.org');

  assert.equal(writes, 0);
});

test('clearUnreadAnchor removes existing anchors with one account-data write', async () => {
  let writtenContent: unknown;
  const mx = {
    getAccountData: () => ({
      getContent: () => ({
        version: 1,
        anchors: {
          '!room:example.org': {
            eventId: '$event',
            ts: 1,
          },
          '!other:example.org': {
            eventId: '$other',
            ts: 2,
          },
        },
      }),
    }),
    setAccountData: async (eventType: string, content: unknown) => {
      assert.equal(eventType, AccountDataEvent.SynaraUnreadAnchor);
      writtenContent = content;
    },
  } as any;

  await clearUnreadAnchor(mx, '!room:example.org');

  assert.deepEqual(writtenContent, {
    version: 1,
    anchors: {
      '!other:example.org': {
        eventId: '$other',
        ts: 2,
      },
    },
  });
});

test('getThreadRootEventId returns thread root ids when available', () => {
  const threadRootEvent = getThreadRootEventId({
    getRelation: () => ({
      rel_type: 'm.thread',
      event_id: '$thread-root',
    }),
  } as any);
  assert.equal(threadRootEvent, '$thread-root');
});

test('getThreadRootEventId ignores non-thread relations', () => {
  const threadRootEvent = getThreadRootEventId({
    getRelation: () => ({
      rel_type: 'm.annotation',
      event_id: '$thread-root',
    }),
  } as any);
  assert.equal(threadRootEvent, undefined);
});
