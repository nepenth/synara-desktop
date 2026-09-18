import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { ImageUsage } from '../../plugins/custom-emoji';
import { imagePackListFingerprint, sameImagePackList } from '../useImagePacks';

const pack = (id: string, shortcode: string) => ({
  id,
  getImages: (usage: ImageUsage) => (usage === ImageUsage.Emoticon ? [{ shortcode }] : []),
});

test('imagePackListFingerprint is order and shortcode sensitive', () => {
  assert.equal(
    imagePackListFingerprint([pack('a', 'thumb')]),
    imagePackListFingerprint([pack('a', 'thumb')])
  );
  assert.equal(sameImagePackList([pack('a', 'thumb')], [pack('a', 'thumb')]), true);
  assert.equal(sameImagePackList([pack('a', 'thumb')], [pack('a', 'heart')]), false);
  assert.equal(sameImagePackList([pack('a', 'thumb')], [pack('b', 'thumb')]), false);
});

test('room image pack effect depends on joined room ids, not array identity', () => {
  const source = readFileSync('src/app/hooks/useImagePacks.ts', 'utf8');
  const roomsHook = source.slice(
    source.indexOf('export const useRoomsImagePacks'),
    source.indexOf('export const useRelevantImagePacks')
  );

  assert.match(roomsHook, /\[roomKey, refreshToken\]/);
  assert.doesNotMatch(roomsHook, /\[roomKey, roomIds, refreshToken\]/);
  assert.match(roomsHook, /sameImagePackList\(prev, all\)/);
});

test('message reaction pickers reuse a stable empty image-pack room list', () => {
  const message = readFileSync('src/app/features/room/message/Message.tsx', 'utf8');
  const editor = readFileSync('src/app/features/room/message/MessageEditor.tsx', 'utf8');
  assert.match(message, /const EMPTY_IMAGE_PACK_ROOMS: string\[\] = \[\]/);
  assert.match(message, /imagePackRooms \?\? EMPTY_IMAGE_PACK_ROOMS/);
  assert.doesNotMatch(message, /imagePackRooms \?\? \[\]/);
  assert.match(editor, /const EMPTY_IMAGE_PACK_ROOMS: string\[\] = \[\]/);
  assert.doesNotMatch(editor, /imagePackRooms \?\? \[\]/);
  assert.doesNotMatch(editor, /imagePackRooms \|\| \[\]/);
});
