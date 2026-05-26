import assert from 'node:assert/strict';
import test from 'node:test';
import {
  MessageSearchTypeFilter,
  filterMessageSearchGroups,
  isMessageSearchResultInDateRange,
  isMessageSearchResultForType,
  parseSenderFilter,
} from '../messageSearchFilters';

test('parseSenderFilter normalizes comma-separated Matrix IDs', () => {
  assert.deepEqual(parseSenderFilter(' @alice:example.org, @bob:example.org ,, '), [
    '@alice:example.org',
    '@bob:example.org',
  ]);
  assert.equal(parseSenderFilter('   '), undefined);
});

test('isMessageSearchResultForType matches richer attachment filters', () => {
  assert.equal(
    isMessageSearchResultForType(
      { event: { content: { msgtype: 'm.image' } } },
      MessageSearchTypeFilter.Media
    ),
    true
  );
  assert.equal(
    isMessageSearchResultForType(
      { event: { content: { msgtype: 'm.text', body: 'see https://example.org' } } },
      MessageSearchTypeFilter.Links
    ),
    true
  );
  assert.equal(
    isMessageSearchResultForType(
      { event: { content: { 'm.poll': { question: {} } } } },
      MessageSearchTypeFilter.Polls
    ),
    true
  );
  assert.equal(
    isMessageSearchResultForType(
      { event: { content: { msgtype: 'm.audio' } } },
      MessageSearchTypeFilter.Files
    ),
    false
  );
});

test('isMessageSearchResultInDateRange applies inclusive day bounds', () => {
  assert.equal(
    isMessageSearchResultInDateRange(
      { event: { origin_server_ts: new Date('2026-05-07T12:00:00.000').getTime() } },
      '2026-05-07',
      '2026-05-07'
    ),
    true
  );
  assert.equal(
    isMessageSearchResultInDateRange(
      { event: { origin_server_ts: new Date('2026-05-06T23:59:59.999').getTime() } },
      '2026-05-07',
      undefined
    ),
    false
  );
});

test('filterMessageSearchGroups removes empty groups after type/date filtering', () => {
  const groups = filterMessageSearchGroups(
    [
      {
        roomId: '!a:example.org',
        items: [
          {
            event: {
              origin_server_ts: new Date('2026-05-07T12:00:00.000').getTime(),
              content: { msgtype: 'm.audio' },
            },
          },
        ],
      },
      {
        roomId: '!b:example.org',
        items: [
          {
            event: {
              origin_server_ts: new Date('2026-05-08T12:00:00.000').getTime(),
              content: { msgtype: 'm.text', body: 'plain text' },
            },
          },
        ],
      },
    ],
    { type: MessageSearchTypeFilter.Audio, fromDate: '2026-05-07', toDate: '2026-05-07' }
  );

  assert.equal(groups.length, 1);
  assert.equal(groups[0].roomId, '!a:example.org');
});
