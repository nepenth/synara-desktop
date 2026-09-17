import assert from 'node:assert/strict';
import test from 'node:test';
import {
  MessageSearchTypeFilter,
  filterMessageSearchGroups,
  isAttachmentListingType,
  isMessageSearchResultInDateRange,
  isMessageSearchResultForType,
  lastNDaysDateRange,
  listingDateRangeToTimestamps,
  parseSenderFilter,
  resolveMessageSearchListingDateRange,
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
  assert.equal(
    isMessageSearchResultForType(
      { event: { content: { msgtype: 'm.video' } } },
      MessageSearchTypeFilter.Media
    ),
    true
  );
  assert.equal(
    isMessageSearchResultForType(
      { event: { content: { msgtype: 'm.text' } } },
      MessageSearchTypeFilter.Media
    ),
    false
  );
  assert.equal(
    isMessageSearchResultForType(
      { event: { content: { msgtype: 'm.file', filename: 'notes.pdf' } } },
      MessageSearchTypeFilter.Files
    ),
    true
  );
  assert.equal(isAttachmentListingType(MessageSearchTypeFilter.Media), true);
  assert.equal(isAttachmentListingType(MessageSearchTypeFilter.Audio), false);
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

test('native hits with msgtype and dates filter media in range', () => {
  const inRange = new Date('2026-05-07T12:00:00.000').getTime();
  const outOfRange = new Date('2026-05-01T12:00:00.000').getTime();
  const groups = filterMessageSearchGroups(
    [
      {
        roomId: '!r:example.org',
        items: [
          {
            event: {
              origin_server_ts: inRange,
              content: { msgtype: 'm.image', body: 'pic.png' },
            },
          },
          {
            event: {
              origin_server_ts: inRange,
              content: { msgtype: 'm.file', body: 'notes.pdf' },
            },
          },
          {
            event: {
              origin_server_ts: outOfRange,
              content: { msgtype: 'm.video', body: 'clip.mp4' },
            },
          },
          {
            event: {
              origin_server_ts: inRange,
              content: { msgtype: 'm.text', body: 'hello' },
            },
          },
        ],
      },
    ],
    { type: MessageSearchTypeFilter.Media, fromDate: '2026-05-07', toDate: '2026-05-07' }
  );

  assert.equal(groups.length, 1);
  assert.equal(groups[0].items.length, 1);
  assert.equal(groups[0].items[0].event.content?.msgtype, 'm.image');
});

test('listing date range defaults to the last 7 local days', () => {
  const now = new Date('2026-05-14T15:00:00');
  const range = resolveMessageSearchListingDateRange(undefined, undefined, now);
  assert.deepEqual(range, lastNDaysDateRange(7, now.getTime()));
  const timestamps = listingDateRangeToTimestamps(range.fromDate, range.toDate);
  assert.ok(timestamps);
  assert.equal(timestamps.fromTs, new Date(`${range.fromDate}T00:00:00.000`).getTime());
  assert.equal(timestamps.toTs, new Date(`${range.toDate}T23:59:59.999`).getTime());
});
