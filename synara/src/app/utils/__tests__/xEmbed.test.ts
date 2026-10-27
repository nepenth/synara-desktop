import assert from 'node:assert/strict';
import test from 'node:test';
import { fetchXPost, parseXPost, xStatusId, xSyndicationToken } from '../xEmbed';

const videoPost = {
  __typename: 'Tweet',
  id_str: '1700000000000000001',
  text: 'Launch day &amp; more https://t.co/abc https://t.co/media',
  display_text_range: [0, 38],
  entities: { urls: [{ url: 'https://t.co/abc', expanded_url: 'https://example.org/launch' }] },
  created_at: '2026-10-01T12:00:00.000Z',
  user: {
    name: 'Example',
    screen_name: 'example',
    profile_image_url_https: 'https://pbs.twimg.com/profile_images/1/a_normal.jpg',
  },
  mediaDetails: [
    {
      type: 'video',
      media_url_https: 'https://pbs.twimg.com/ext_tw_video_thumb/1/pu/img/poster.jpg',
      original_info: { width: 1280, height: 720 },
      video_info: {
        aspect_ratio: [16, 9],
        variants: [
          { content_type: 'application/x-mpegURL', url: 'https://video.twimg.com/a.m3u8' },
          { bitrate: 832000, content_type: 'video/mp4', url: 'https://video.twimg.com/low.mp4' },
          { bitrate: 2176000, content_type: 'video/mp4', url: 'https://video.twimg.com/high.mp4' },
          { bitrate: 9000000, content_type: 'video/mp4', url: 'https://evil.example/x.mp4' },
        ],
      },
    },
  ],
};

test('status links from x.com and twitter.com give the post id', () => {
  assert.equal(
    xStatusId('https://x.com/example/status/1700000000000000001'),
    '1700000000000000001'
  );
  assert.equal(xStatusId('https://twitter.com/example/status/20?s=20'), '20');
  assert.equal(xStatusId('https://mobile.x.com/example/status/20/video/1'), '20');
  assert.equal(xStatusId('https://x.com/i/web/status/20'), '20');
  assert.equal(xStatusId('https://x.com/example'), undefined);
  assert.equal(xStatusId('https://x.com.evil.example/a/status/20'), undefined);
  assert.equal(xStatusId('https://x.com/a/status/20abc'), undefined);
});

test('the token matches the one X embeds derive', () => {
  assert.equal(xSyndicationToken('20'), '6dq1a2xwd93');
});

test('a video post keeps the best MP4 from X and its poster', () => {
  const post = parseXPost(videoPost, 'https://x.com/example/status/1700000000000000001');
  assert.ok(post);
  assert.equal(post.authorHandle, 'example');
  assert.equal(post.text, 'Launch day & more https://example.org/launch');
  assert.deepEqual(post.media, [
    {
      kind: 'video',
      src: 'https://video.twimg.com/high.mp4',
      poster: 'https://pbs.twimg.com/ext_tw_video_thumb/1/pu/img/poster.jpg',
      width: 1280,
      height: 720,
    },
  ]);
});

test('deleted posts and media off X CDNs are dropped', () => {
  assert.equal(parseXPost({ __typename: 'TweetTombstone' }, 'https://x.com/a/status/1'), undefined);
  const offCdn = parseXPost(
    {
      ...videoPost,
      user: { ...videoPost.user, profile_image_url_https: 'https://evil.example/a.jpg' },
      mediaDetails: [{ type: 'photo', media_url_https: 'https://evil.example/p.jpg' }],
    },
    'https://x.com/a/status/1'
  );
  assert.ok(offCdn);
  assert.equal(offCdn.avatarUrl, undefined);
  assert.deepEqual(offCdn.media, []);
});

test('fetchXPost sends only the id and token to the native command', async () => {
  const calls: Array<[string, Record<string, unknown> | undefined]> = [];
  const post = await fetchXPost(
    'https://x.com/example/status/1700000000000000001',
    async (command, args) => {
      calls.push([command, args]);
      return { available: true, value: JSON.stringify(videoPost) };
    }
  );
  assert.equal(post?.id, '1700000000000000001');
  assert.deepEqual(calls, [
    [
      'desktop_x_post',
      { id: '1700000000000000001', token: xSyndicationToken('1700000000000000001') },
    ],
  ]);
  assert.equal(
    await fetchXPost('https://x.com/example/status/1700000000000000002', async () => ({
      available: true,
      value: 'not json',
    })),
    undefined
  );
});
