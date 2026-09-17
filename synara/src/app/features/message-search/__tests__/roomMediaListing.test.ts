import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';
import { isRoomAttachmentListingEnabled, listingQueryRange } from '../roomMediaListing';

const listingSource = readFileSync(
  join(process.cwd(), 'src/app/features/message-search/roomMediaListing.ts'),
  'utf8'
);
const messageSearchSource = readFileSync(
  join(process.cwd(), 'src/app/features/message-search/MessageSearch.tsx'),
  'utf8'
);
const hookSource = readFileSync(
  join(process.cwd(), 'src/app/features/message-search/useMessageSearch.ts'),
  'utf8'
);

test('empty-term listing is scoped to one room and media/files', () => {
  assert.equal(
    isRoomAttachmentListingEnabled({
      term: '',
      type: 'media',
      rooms: ['!r:example.org'],
    }),
    true
  );
  assert.equal(
    isRoomAttachmentListingEnabled({
      term: 'keyword',
      type: 'media',
      rooms: ['!r:example.org'],
    }),
    false
  );
  assert.equal(
    isRoomAttachmentListingEnabled({
      type: 'files',
      rooms: ['!a:example.org', '!b:example.org'],
    }),
    false
  );
  assert.equal(
    isRoomAttachmentListingEnabled({
      type: 'audio',
      rooms: ['!r:example.org'],
    }),
    false
  );
});

test('listing query range defaults last 7 days and never uses mx.search', () => {
  const range = listingQueryRange(undefined, undefined, new Date('2026-05-14T15:00:00'));
  assert.ok(range);
  assert.ok(range.toTs >= range.fromTs);
  assert.match(listingSource, /listingKind/);
  assert.match(listingSource, /matrix_message_search/);
  assert.equal(listingSource.includes('mx.search'), false);
  assert.equal(listingSource.includes('search_events'), false);
  assert.match(messageSearchSource, /listRoomAttachments/);
  assert.match(messageSearchSource, /Listing attachments in this room/);
  assert.equal(hookSource.includes('leftover-unavailable'), false);
});
