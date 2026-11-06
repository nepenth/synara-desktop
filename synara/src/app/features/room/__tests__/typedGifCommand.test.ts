import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { isTypedGifCommand, typedCommandName } from '../typedGifCommand';

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

test('any command can be typed out instead of picked from the list', () => {
  assert.equal(typedCommandName('/remove @puck:example.org'), 'remove');
  assert.equal(typedCommandName('/kick @a:b -r spam'), 'kick');
  assert.equal(typedCommandName('/me waves'), 'me');
  assert.equal(typedCommandName('/leave'), 'leave');
  // Not a command: a path, an escaped slash, text before it.
  assert.equal(typedCommandName('/usr/bin is missing'), undefined);
  assert.equal(typedCommandName('//kick is a command'), undefined);
  assert.equal(typedCommandName('please /kick them'), undefined);
});

test('the composer opens the picker for typed /gif and for Enter on /gif', () => {
  const roomInput = readFileSync('src/app/features/room/RoomInput.tsx', 'utf8');
  assert.match(
    roomInput,
    /getBeginCommand\(editor\) \?\?\s*typedCommandName\(plainText\)/
  );
  assert.match(
    roomInput,
    /autocompleteQuery\.prefix === AutocompletePrefix\.Command &&\s*autocompleteQuery\.text === Command\.Gif/
  );
});
