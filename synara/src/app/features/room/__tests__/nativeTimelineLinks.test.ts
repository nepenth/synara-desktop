import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  isNativeCustomEmojiSource,
  isNativeOpenableHref,
  nativeCustomEmojiContentUri,
  nativeFormattedBodyMentionsUser,
  nativeLinkHostMismatch,
  parseNativePermalink,
} from '../nativeTimelineLinks';
import { projectNativeFormattedBody } from '../nativeTimelinePresentationProjection';

test('matrix.to permalinks resolve to users, rooms and events (D13)', () => {
  assert.deepEqual(parseNativePermalink('https://matrix.to/#/@alice:example.org'), {
    kind: 'user',
    userId: '@alice:example.org',
  });
  assert.deepEqual(parseNativePermalink('https://matrix.to/#/%40alice%3Aexample.org'), {
    kind: 'user',
    userId: '@alice:example.org',
  });
  assert.deepEqual(
    parseNativePermalink('https://matrix.to/#/!room:example.org/$event?via=example.org'),
    {
      kind: 'room',
      roomIdOrAlias: '!room:example.org',
      eventId: '$event',
      viaServers: ['example.org'],
    }
  );
  assert.deepEqual(parseNativePermalink('https://matrix.to/#/#lobby:example.org'), {
    kind: 'room',
    roomIdOrAlias: '#lobby:example.org',
    viaServers: undefined,
  });
  assert.equal(parseNativePermalink('https://example.org/#/@alice:example.org'), undefined);
});

test('a link whose text is a different web address names its real host', () => {
  assert.equal(
    nativeLinkHostMismatch('https://bank.example', 'https://evil.example/x'),
    'evil.example'
  );
  assert.equal(
    nativeLinkHostMismatch('bank.example/login', 'https://evil.example'),
    'evil.example'
  );
  assert.equal(
    nativeLinkHostMismatch('https://www.example.org/a', 'https://example.org/b'),
    undefined
  );
  assert.equal(nativeLinkHostMismatch('the docs', 'https://evil.example'), undefined);
  assert.equal(nativeLinkHostMismatch('mail me', 'mailto:a@example.org'), undefined);
});

test('only mxc custom emoji load, through a small Core thumbnail', () => {
  assert.equal(isNativeCustomEmojiSource('mxc://example.org/AbC_123-x'), true);
  assert.equal(isNativeCustomEmojiSource('https://example.org/e.png'), false);
  assert.equal(isNativeCustomEmojiSource('mxc://example.org/a/../b'), false);
  assert.equal(isNativeCustomEmojiSource(undefined), false);
  assert.match(
    nativeCustomEmojiContentUri('mxc://example.org/party'),
    /^thumbnail\/\d+x\d+\/scale\/mxc:\/\/example\.org\/party$/
  );
  const projected = projectNativeFormattedBody(
    'Hi <img data-mx-emoticon src="mxc://example.org/party" alt=":party:"> there'
  );
  assert.deepEqual(projected?.customEmoji, [':party:']);
  assert.equal(projected?.resourceOwningElements, 0);
});

test('ftp and magnet links show their address; web and mail links open', () => {
  assert.equal(isNativeOpenableHref('https://example.org'), true);
  assert.equal(isNativeOpenableHref('mailto:a@example.org'), true);
  assert.equal(isNativeOpenableHref('ftp://example.org/file'), false);
  assert.equal(isNativeOpenableHref('magnet:?xt=urn:btih:abc'), false);
});

test('mentions of the signed-in user are recognised for the row highlight', () => {
  const html = '<a href="https://matrix.to/#/@me:example.org">Me</a> look';
  assert.equal(nativeFormattedBodyMentionsUser(html, '@me:example.org'), true);
  assert.equal(
    nativeFormattedBodyMentionsUser(
      '<a href="https://matrix.to/#/%40me%3Aexample.org">Me</a>',
      '@me:example.org'
    ),
    true
  );
  assert.equal(nativeFormattedBodyMentionsUser(html, '@you:example.org'), false);
  assert.equal(nativeFormattedBodyMentionsUser(undefined, '@me:example.org'), false);
});

test('formatted bodies navigate permalinks in app and show link destinations', () => {
  const body = readFileSync('src/app/features/room/nativeTimelineFormattedBody.tsx', 'utf8');
  assert.match(body, /\[IN_APP_LINK_ATTRIBUTE\]: ''/);
  assert.match(body, /onPermalink\(permalink, event\.currentTarget\)/);
  assert.match(body, /customHtmlCss\.Mention\(\{/);
  assert.match(body, /title=\{href\}/);
  assert.match(body, /\(link to \$\{mismatchHost\}\)/);
  assert.match(body, /data-native-unopenable-link="true"/);
  const appLinks = readFileSync('src/app/utils/appLinks.ts', 'utf8');
  assert.match(appLinks, /anchor\.hasAttribute\(IN_APP_LINK_ATTRIBUTE\)/);
  const room = readFileSync('src/app/features/room/Room.tsx', 'utf8');
  assert.match(room, /<NativePermalinkProvider value=\{permalinkHandler\}>/);
  const presenter = readFileSync('src/app/features/room/NativeTimelinePresenter.tsx', 'utf8');
  assert.match(presenter, /htmlCss\.MentionsMeRow/);
});
