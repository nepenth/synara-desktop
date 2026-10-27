import assert from 'node:assert/strict';
import test from 'node:test';
import { inlineGifLink, messageInlineGif } from '../gifLinks';

test('Klipy and GIPHY media links play inline, kept exactly as sent', () => {
  const klipy = 'https://static1.klipy.com/ii/abc/def/md.gif?x=1';
  assert.deepEqual(inlineGifLink(klipy), { url: klipy, host: 'static1.klipy.com' });
  assert.ok(inlineGifLink('https://media3.giphy.com/media/abc/giphy.gif'));
  assert.ok(inlineGifLink('https://i.giphy.com/abc.webp'));
});

test('other hosts, non-images and unsafe links stay ordinary links', () => {
  assert.equal(inlineGifLink('https://example.org/cat.gif'), undefined);
  assert.equal(inlineGifLink('https://static.klipy.com.evil.example/cat.gif'), undefined);
  assert.equal(inlineGifLink('http://static.klipy.com/cat.gif'), undefined);
  assert.equal(inlineGifLink('https://static.klipy.com/cat.mp4'), undefined);
  assert.equal(inlineGifLink('https://klipy.com/gifs/cat'), undefined);
});

test('only a message that is nothing but the link becomes a GIF', () => {
  const url = 'https://static.klipy.com/ii/abc/md.gif';
  assert.ok(messageInlineGif(`  ${url}\n`));
  assert.equal(messageInlineGif(`look ${url}`), undefined);
  assert.equal(messageInlineGif(''), undefined);
});
