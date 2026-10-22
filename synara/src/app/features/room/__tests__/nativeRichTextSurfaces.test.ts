import assert from 'node:assert/strict';
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';
import { projectNativeFormattedBody } from '../nativeTimelinePresentationProjection';

// D84: inbox, pins, both searches, the unfurl card, room topics, image-pack
// attributions and reply previews draw message text with the timeline's
// renderer. The Cinny-era parser and its separate sanitizer surface are gone.

const read = (path: string) => readFileSync(join('src/app', path), 'utf8');

const sourceFiles = (dir: string): string[] =>
  readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === '__tests__' ? [] : sourceFiles(path);
    return /\.(ts|tsx)$/.test(name) ? [path] : [];
  });

test('the compatibility HTML renderer is deleted and nothing imports it', () => {
  assert.equal(existsSync('src/app/plugins/react-custom-html-parser.tsx'), false);
  assert.equal(existsSync('src/app/components/message/RenderBody.tsx'), false);
  assert.equal(existsSync('src/app/hooks/useMentionClickHandler.ts'), false);
  for (const file of [...sourceFiles('src'), ...sourceFiles('e2e')]) {
    const source = readFileSync(file, 'utf8');
    assert.doesNotMatch(source, /react-custom-html-parser/, file);
    assert.doesNotMatch(source, /getReactCustomHtmlParser|LINKIFY_OPTS|scaleSystemEmoji/, file);
  }
});

test('message surfaces outside the timeline render text with the native entry point', () => {
  const content = read('components/RenderMessageContent.tsx');
  assert.match(
    content,
    /import \{ NativeMessageText \} from '\.\.\/features\/room\/nativeTimelineFormattedBody'/
  );
  assert.match(
    content,
    /<NativeMessageText body=\{body\} formattedBody=\{customBody\} highlightRegex=\{highlightRegex\} \/>/
  );
  assert.doesNotMatch(content, /htmlReactParserOptions|linkifyOpts|sanitizeCustomHtml/);

  for (const surface of [
    'pages/client/inbox/Notifications.tsx',
    'features/room/room-pin-menu/RoomPinMenu.tsx',
    'features/message-search/SearchResultGroup.tsx',
  ]) {
    const source = read(surface);
    assert.match(source, /<RenderMessageContent/, surface);
    assert.doesNotMatch(
      source,
      /htmlReactParserOptions|linkifyOpts|useMentionClickHandler/,
      surface
    );
  }
  // The inbox and search live outside a room view, so they bring their own
  // in-app permalink handler; the pin menu sits inside the room's.
  for (const surface of [
    'pages/client/inbox/Notifications.tsx',
    'features/message-search/SearchResultGroup.tsx',
  ]) {
    assert.match(read(surface), /<NativeRoomPermalinks roomId=\{props\.room\.roomId\}>/, surface);
  }
  assert.match(
    read('features/room/Room.tsx'),
    /<NativePermalinkProvider value=\{permalinkHandler\}>/
  );
  assert.match(
    read('features/message-search/SearchResultGroup.tsx'),
    /makeHighlightRegex\(highlights\)/
  );
});

test('topics, attributions, plain bodies and highlights use the shared helpers', () => {
  for (const surface of [
    'components/room-topic-viewer/RoomTopicViewer.tsx',
    'features/common-settings/general/RoomProfile.tsx',
    'components/image-pack-view/PackMeta.tsx',
    'features/room/nativeLinkUnfurlCard.tsx',
  ]) {
    const source = read(surface);
    assert.match(source, /<NativeLinkifiedText text=\{/, surface);
    assert.doesNotMatch(source, /from 'linkify-react'/, surface);
  }
  for (const surface of [
    'features/search/Search.tsx',
    'features/add-existing/AddExisting.tsx',
    'components/invite-user-prompt/InviteUserPrompt.tsx',
    'components/create-room/AdditionalCreatorInput.tsx',
  ]) {
    assert.match(read(surface), /from '\.\.\/\.\.\/utils\/highlightText'/, surface);
  }
  assert.doesNotMatch(read('components/message/Reply.tsx'), /scaleSystemEmoji/);
});

test('plain-text links go through the formatted anchor and only open what the desktop can', () => {
  const body = read('features/room/nativeTimelineFormattedBody.tsx');
  assert.match(body, /url: \(value\) => isNativeOpenableHref\(value\)/);
  assert.match(body, /return renderNativeAnchor\(href, content, content, context\);/);
  // Search highlighting leaves inline code exactly as sent.
  assert.match(body, /domNode\.parent\.name === 'code'/);
});

test('the shared renderer keeps the native sanitizer: inert custom emoji, no remote images', () => {
  const projection = projectNativeFormattedBody(
    '<p><img data-mx-emoticon src="mxc://example.test/party" alt=":party:">' +
      '<img data-mx-emoticon src="https://evil.example/e.png" alt=":evil:">' +
      '<img src="https://tracker.example/pixel.png" alt="tracker">' +
      '<a href="javascript:alert(1)">x</a><script>bad()</script></p>'
  );
  assert.ok(projection);
  assert.equal(projection.resourceOwningElements, 0);
  assert.match(
    projection.sanitizedHtml,
    /<span data-mx-emoticon="mxc:\/\/example\.test\/party">:party:<\/span>/
  );
  assert.doesNotMatch(
    projection.sanitizedHtml,
    /evil\.example|tracker\.example|javascript:|<script/
  );
  assert.deepEqual(projection.customEmoji, [':party:']);
});
