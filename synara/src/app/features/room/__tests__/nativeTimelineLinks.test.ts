import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  isNativeCustomEmojiSource,
  isNativeOpenableHref,
  nativeCustomEmojiContentUri,
  nativeLinkHostMismatch,
  parseNativePermalink,
  shortLinkLabel,
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
  // A mention of you is marked on its pill, not across the whole message.
  assert.match(
    body,
    /highlight: permalink\.kind === 'user' && permalink\.userId === context\.ownUserId/
  );
  const presenter = readFileSync('src/app/features/room/NativeTimelinePresenter.tsx', 'utf8');
  assert.doesNotMatch(presenter, /MentionsMeRow/);
});

test('a long bare web address shows as its site and path, without the query', () => {
  const yahoo =
    'https://www.yahoo.com/entertainment/celebrity/article/patrick-stewart-star-of-x-men-and-star-trek-dies-at-86-195431969.html?guccounter=1&guce_referrer=aHR0cHM6Ly93d3cuZ29vZ2xlLmNvbS8';
  assert.equal(shortLinkLabel(yahoo, yahoo), 'yahoo.com/…/patrick-stewart-star-of-x-men-and-…');
  const tracked = 'https://example.com/news/story?utm_source=feed&utm_medium=rss&utm_campaign=x';
  assert.equal(shortLinkLabel(tracked, tracked), 'example.com/news/story…');
  const path = 'https://docs.example.org/guide/install/linux/arch/packages/troubleshooting';
  assert.equal(shortLinkLabel(path, path), 'docs.example.org/…/troubleshooting');
  // linkify links `www.` text with an http href.
  const www = 'www.example.com/a/b?session=0123456789abcdef0123456789abcdef';
  assert.equal(shortLinkLabel(www, `http://${www}`), 'example.com/a/b…');
});

test('the site is never cut and other links keep their text', () => {
  const longHost =
    'https://paypal.com.account-verify.secure-login.example-phishing-host.test/x?token=1';
  assert.equal(
    shortLinkLabel(longHost, longHost),
    'paypal.com.account-verify.secure-login.example-phishing-host.test/…'
  );
  const hugeHost = `https://${'a'.repeat(60)}.test/one/two/three`;
  assert.equal(shortLinkLabel(hugeHost, hugeHost), `${'a'.repeat(60)}.test/…`);
  // Short addresses, written words and links to a different address stay.
  assert.equal(shortLinkLabel('https://example.com/a?b=c', 'https://example.com/a?b=c'), undefined);
  const long = 'https://example.com/a/very/long/path/that/goes/on/and/on/for/a/while';
  assert.equal(shortLinkLabel('Read the full article here please, it is long', long), undefined);
  assert.equal(shortLinkLabel(long, 'https://example.com/other'), undefined);
  // Only plain web addresses: no credentials, no other schemes.
  const creds = 'https://user:secret@example.com/a/very/long/path/that/goes/on/and/on';
  assert.equal(shortLinkLabel(creds, creds), undefined);
  const mail = 'mailto:someone.with.a.very.long.address@example-company-domain.test';
  assert.equal(shortLinkLabel(mail, mail), undefined);
});

test('formatted bodies shorten only bare addresses, never search results', () => {
  const body = readFileSync('src/app/features/room/nativeTimelineFormattedBody.tsx', 'utf8');
  assert.match(
    body,
    /context\.shortenLinks && !context\.highlightRegex \? shortLinkLabel\(text, href\)/
  );
  assert.match(body, /\{shortLabel \?\? children\}/);
  assert.equal(body.match(/useSetting\(settingsAtom, 'shortenLinks'\)/g)?.length, 2);
});
