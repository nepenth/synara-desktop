import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';
import { mapNativeSearchResult } from '../nativeMessageSearchMap';

const source = readFileSync(
  join(process.cwd(), 'src/app/features/message-search/useMessageSearch.ts'),
  'utf8'
);

test('native message search invokes matrix_message_search and never mx.search', () => {
  assert.match(source, /isNativeMatrixSession\(\)/);
  assert.match(source, /matrix_message_search/);
  assert.match(source, /invokeDesktopWithAvailability/);
  const nativeBranch = source.slice(
    source.indexOf('if (nativeSession)'),
    source.indexOf('const limit = 20')
  );
  assert.match(nativeBranch, /matrix_message_search/);
  assert.equal(nativeBranch.includes('mx.search'), false);
  assert.equal(nativeBranch.includes('leftover-unavailable'), false);
});

test('mapNativeSearchResult preserves msgType instead of hardcoding m.text', () => {
  const mapped = mapNativeSearchResult({
    highlights: [],
    groups: [
      {
        roomId: '!r:example.org',
        items: [
          {
            rank: 1,
            eventId: '$img',
            sender: '@a:example.org',
            originServerTs: 1_700_000_000_000,
            body: 'pic.png',
            roomId: '!r:example.org',
            msgType: 'm.image',
          },
          {
            rank: 0.5,
            eventId: '$txt',
            sender: '@a:example.org',
            originServerTs: 1_700_000_000_001,
            body: 'hello',
            roomId: '!r:example.org',
          },
        ],
      },
    ],
  });

  assert.equal(mapped.groups[0].items[0].event.content.msgtype, 'm.image');
  assert.equal(mapped.groups[0].items[0].event.origin_server_ts, 1_700_000_000_000);
  assert.equal(mapped.groups[0].items[1].event.content.msgtype, 'm.text');
});
