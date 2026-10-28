import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { isTypedGifCommand } from '../typedGifCommand';

test('typed /gif with or without a search opens the picker', () => {
  assert.equal(isTypedGifCommand('/gif'), true);
  assert.equal(isTypedGifCommand('/gif high five'), true);
  assert.equal(isTypedGifCommand('/gif\nhigh five'), true);
});

test('other text that starts with /gif is sent as a message', () => {
  assert.equal(isTypedGifCommand('/gifs are fun'), false);
  assert.equal(isTypedGifCommand('see /gif'), false);
  assert.equal(isTypedGifCommand('/GIF'), false);
  assert.equal(isTypedGifCommand(''), false);
});

test('the composer opens the picker for typed /gif and for Enter on /gif', () => {
  const roomInput = readFileSync('src/app/features/room/RoomInput.tsx', 'utf8');
  assert.match(
    roomInput,
    /getBeginCommand\(editor\) \?\?\s*\(isTypedGifCommand\(plainText\) \? Command\.Gif : undefined\)/
  );
  assert.match(
    roomInput,
    /autocompleteQuery\.prefix === AutocompletePrefix\.Command &&\s*autocompleteQuery\.text === Command\.Gif/
  );
});
