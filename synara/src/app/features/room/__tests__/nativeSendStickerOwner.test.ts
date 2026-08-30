import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const roomInput = readFileSync('src/app/features/room/RoomInput.tsx', 'utf8');
const emojiBoard = readFileSync('src/app/components/emoji-board/EmojiBoard.tsx', 'utf8');
const imagePackContent = readFileSync(
  'src/app/components/image-pack-view/ImagePackContent.tsx',
  'utf8'
);
const imageTile = readFileSync('src/app/components/image-pack-view/ImageTile.tsx', 'utf8');
const packMeta = readFileSync('src/app/components/image-pack-view/PackMeta.tsx', 'utf8');
const roomSettings = readFileSync('src/app/features/room-settings/RoomSettings.tsx', 'utf8');
const spaceSettings = readFileSync('src/app/features/space-settings/SpaceSettings.tsx', 'utf8');
const nativePresenter = readFileSync('src/app/features/room/NativeTimelinePresenter.tsx', 'utf8');

test('desktop composer exposes emoji and reactions without an outgoing sticker path', () => {
  for (const removedSurface of [
    'nativeSendSticker',
    'handleStickerSelect',
    'onStickerSelect',
    'EmojiBoardTab.Sticker',
    "'m.sticker'",
  ]) {
    assert.equal(roomInput.includes(removedSurface), false, `found ${removedSurface}`);
  }
  assert.equal(emojiBoard.includes('ImageUsage.Sticker'), false);
  assert.equal(emojiBoard.includes('EmojiType.Sticker'), false);
  assert.match(roomInput, /onCustomEmojiSelect=\{handleEmoticonSelect\}/);
});

test('image-pack creation is emoji-only while existing Matrix usage metadata is preserved', () => {
  assert.match(imagePackContent, /usage: \[ImageUsage\.Emoticon\]/);
  assert.match(imagePackContent, /pack: savedMeta\?\.content \?\? imagePack\.meta\.content/);
  assert.match(imageTile, /\.\.\.existingContent/);
  assert.match(packMeta, /\.\.\.meta\.content/);
  assert.equal(imageTile.includes('usage: [ImageUsage.Emoticon]'), false);
  assert.equal(roomSettings.includes('Emojis & Stickers'), false);
  assert.equal(spaceSettings.includes('Emojis & Stickers'), false);
  assert.match(roomSettings, /Custom Emoji/);
  assert.match(spaceSettings, /Custom Emoji/);
});

test('incoming Matrix stickers remain renderable after outgoing surfaces are removed', () => {
  assert.match(nativePresenter, /case 'sticker'/);
  assert.match(nativePresenter, /<NativeTimelineMedia media=\{row\.media\} sticker/);
});
