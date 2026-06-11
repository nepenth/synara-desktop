import assert from 'node:assert/strict';
import test from 'node:test';
import { createBoundedLruMap, createBoundedLruSet } from '../boundedLru';

test('createBoundedLruSet evicts least-recently-used ids when over capacity', () => {
  const cache = createBoundedLruSet(3);

  cache.add('a');
  cache.add('b');
  cache.add('c');
  assert.equal(cache.size, 3);
  assert.equal(cache.has('a'), true);

  cache.add('d');
  assert.equal(cache.size, 3);
  assert.equal(cache.has('a'), false);
  assert.equal(cache.has('b'), true);
  assert.equal(cache.has('c'), true);
  assert.equal(cache.has('d'), true);
});

test('createBoundedLruSet dedupes ids and refreshes recency on duplicate add', () => {
  const cache = createBoundedLruSet(3);

  cache.add('a');
  cache.add('b');
  cache.add('c');
  cache.add('a');

  cache.add('d');
  assert.equal(cache.size, 3);
  assert.equal(cache.has('a'), true);
  assert.equal(cache.has('b'), false);
  assert.equal(cache.has('c'), true);
  assert.equal(cache.has('d'), true);
});

test('createBoundedLruSet bounds large approval id streams', () => {
  const cache = createBoundedLruSet(500);

  for (let index = 0; index < 10_000; index += 1) {
    cache.add(`$event-${index}`);
  }

  assert.equal(cache.size, 500);
  assert.equal(cache.has('$event-0'), false);
  assert.equal(cache.has('$event-9499'), false);
  assert.equal(cache.has('$event-9500'), true);
  assert.equal(cache.has('$event-9999'), true);
});

test('createBoundedLruMap evicts least-recently-used room entries when over capacity', () => {
  const cache = createBoundedLruMap<string, number>(3);

  cache.set('room-a', 1);
  cache.set('room-b', 2);
  cache.set('room-c', 3);
  assert.equal(cache.size, 3);
  assert.equal(cache.get('room-a'), 1);

  cache.set('room-d', 4);
  assert.equal(cache.size, 3);
  assert.equal(cache.get('room-a'), 1);
  assert.equal(cache.get('room-b'), undefined);
  assert.equal(cache.get('room-c'), 3);
  assert.equal(cache.get('room-d'), 4);
});

test('createBoundedLruMap dedupes room ids and refreshes recency on duplicate set', () => {
  const cache = createBoundedLruMap<string, number>(3);

  cache.set('room-a', 1);
  cache.set('room-b', 2);
  cache.set('room-c', 3);
  cache.set('room-a', 10);

  cache.set('room-d', 4);
  assert.equal(cache.size, 3);
  assert.equal(cache.get('room-a'), 10);
  assert.equal(cache.get('room-b'), undefined);
  assert.equal(cache.get('room-c'), 3);
  assert.equal(cache.get('room-d'), 4);
});

test('createBoundedLruMap bounds large unread room streams', () => {
  const cache = createBoundedLruMap<string, number>(200);

  for (let index = 0; index < 1_000; index += 1) {
    cache.set(`!room-${index}:example.org`, index);
  }

  assert.equal(cache.size, 200);
  assert.equal(cache.get('!room-0:example.org'), undefined);
  assert.equal(cache.get('!room-799:example.org'), undefined);
  assert.equal(cache.get('!room-800:example.org'), 800);
  assert.equal(cache.get('!room-999:example.org'), 999);
});

test('createBoundedLruSet and createBoundedLruMap clear all entries', () => {
  const eventIds = createBoundedLruSet(2);
  const unreadRooms = createBoundedLruMap<string, number>(2);

  eventIds.add('a');
  unreadRooms.set('room-a', 1);

  eventIds.clear();
  unreadRooms.clear();

  assert.equal(eventIds.size, 0);
  assert.equal(unreadRooms.size, 0);
  assert.equal(eventIds.has('a'), false);
  assert.equal(unreadRooms.get('room-a'), undefined);
});
