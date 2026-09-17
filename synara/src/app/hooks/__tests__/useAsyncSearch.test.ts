import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { shouldPreserveQueryOnListChange, stabilizeListIdentity } from '../useAsyncSearch';

test('shouldPreserveQueryOnListChange keeps results only while a query is active', () => {
  assert.equal(shouldPreserveQueryOnListChange('thumb'), true);
  assert.equal(shouldPreserveQueryOnListChange(''), false);
  assert.equal(shouldPreserveQueryOnListChange(undefined), false);
});

test('stabilizeListIdentity reuses the previous array when item identities match', () => {
  const first = { id: 1 };
  const second = { id: 2 };
  const previous = [first, second];

  assert.equal(stabilizeListIdentity(previous, [first, second]), previous);
  const next = [first];
  assert.equal(stabilizeListIdentity(previous, next), next);
  assert.equal(stabilizeListIdentity(undefined, next), next);
});

test('emoji board renders only search results while a query result is present', () => {
  const source = readFileSync('src/app/components/emoji-board/EmojiBoard.tsx', 'utf8');

  assert.match(source, /stabilizeListIdentity\(searchListRef\.current, nextSearchList\)/);
  assert.match(source, /count: result \? 0 : groups\.length/);
  assert.match(source, /result \? \([\s\S]*SEARCH_GROUP_ID[\s\S]*\) : \([\s\S]*vItems\.map/);
  assert.doesNotMatch(
    source,
    /searchedItems && \([\s\S]*SEARCH_GROUP_ID[\s\S]*vItems\.map/,
    'search results must not render above the virtualized catalog'
  );
});
