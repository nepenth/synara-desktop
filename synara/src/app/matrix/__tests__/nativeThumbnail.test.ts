import assert from 'node:assert/strict';
import test from 'node:test';
import {
  NATIVE_AVATAR_THUMBNAIL_PX,
  nativeThumbnailContentUri,
  stripNativeThumbnail,
} from '../nativeThumbnail';
import { isNativeMediaContentUri } from '../media';
import { nativeTimelineInlineImageContentUri } from '../../features/room/nativeTimelineView';

test('native thumbnails wrap mxc and timeline handles with device-scaled sizes', () => {
  assert.equal(
    nativeThumbnailContentUri('mxc://example.org/avatar', 96, 96, 'crop', 1),
    'thumbnail/96x96/crop/mxc://example.org/avatar'
  );
  assert.equal(
    nativeThumbnailContentUri('timeline-media-ab', 480, 480, 'scale', 2),
    'thumbnail/960x960/scale/timeline-media-ab'
  );
  assert.equal(
    nativeThumbnailContentUri(' mxc://example.org/avatar ', 10_000, 10_000, 'crop', 3),
    'thumbnail/4096x4096/crop/mxc://example.org/avatar'
  );
  assert.equal(NATIVE_AVATAR_THUMBNAIL_PX, 96);
});

test('native thumbnails leave other sources and invalid sizes unchanged', () => {
  const wrapped = 'thumbnail/96x96/crop/mxc://example.org/avatar';
  assert.equal(nativeThumbnailContentUri(wrapped, 48, 48, 'crop', 1), wrapped);
  assert.equal(
    nativeThumbnailContentUri('https://example.org/a.png', 96, 96, 'crop', 1),
    'https://example.org/a.png'
  );
  assert.equal(
    nativeThumbnailContentUri('mxc://example.org/avatar', 0, 96, 'crop', 1),
    'mxc://example.org/avatar'
  );
  assert.equal(
    nativeThumbnailContentUri('mxc://example.org/avatar', Number.NaN, 96, 'crop', 1),
    'mxc://example.org/avatar'
  );
});

test('thumbnail wrappers are native media and strip back to the original', () => {
  const wrapped = 'thumbnail/96x96/crop/mxc://example.org/avatar';
  assert.equal(isNativeMediaContentUri(wrapped), true);
  assert.equal(stripNativeThumbnail(wrapped), 'mxc://example.org/avatar');
  assert.equal(
    stripNativeThumbnail('thumbnail/960x960/scale/timeline-media-ab'),
    'timeline-media-ab'
  );
  assert.equal(stripNativeThumbnail('mxc://example.org/avatar'), 'mxc://example.org/avatar');
});

test('inline timeline images use a thumbnail only when the original exceeds the box', () => {
  const handle = { handleId: 'timeline-media-ab', mimeType: 'image/png' };
  assert.equal(
    nativeTimelineInlineImageContentUri({ ...handle, width: 4000, height: 3000 }, 480, 2),
    'thumbnail/960x960/scale/timeline-media-ab'
  );
  assert.equal(
    nativeTimelineInlineImageContentUri(handle, 480, 1),
    'thumbnail/480x480/scale/timeline-media-ab',
    'unknown dimensions may be large'
  );
  assert.equal(
    nativeTimelineInlineImageContentUri({ ...handle, width: 640, height: 480 }, 480, 2),
    'timeline-media-ab',
    'already within the device box'
  );
  assert.equal(
    nativeTimelineInlineImageContentUri(
      { handleId: 'timeline-media-ab', mimeType: 'image/gif', width: 4000, height: 3000 },
      480,
      2
    ),
    'timeline-media-ab',
    'GIFs keep the animated original'
  );
});
