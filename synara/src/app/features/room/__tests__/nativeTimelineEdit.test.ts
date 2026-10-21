import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  dropNativeEditDraft,
  isNativeEditUnchanged,
  isNativeTextEditable,
  keepNativeEditDraft,
  nativeEditInitialHtml,
  nativeEditMsgType,
  takeNativeEditDraft,
} from '../nativeTimelineEdit';

test('an edit keeps the message type (D76)', () => {
  assert.equal(nativeEditMsgType('emote'), 'm.emote');
  assert.equal(nativeEditMsgType('notice'), 'm.notice');
  assert.equal(nativeEditMsgType('text'), 'm.text');
  assert.equal(nativeEditMsgType(undefined), 'm.text');
  assert.equal(isNativeTextEditable('emote'), true);
  assert.equal(isNativeTextEditable('image'), false);
  assert.equal(isNativeTextEditable('file'), false);
});

test('the editor starts from the message HTML without a reply fallback', () => {
  assert.equal(
    nativeEditInitialHtml('<mx-reply><blockquote>quoted</blockquote></mx-reply><b>new</b>'),
    '<b>new</b>'
  );
  assert.equal(nativeEditInitialHtml('<p>kept</p>'), '<p>kept</p>');
  assert.equal(nativeEditInitialHtml(undefined), undefined);
  assert.equal(nativeEditInitialHtml('<mx-reply>only</mx-reply>'), undefined);
});

test('saving an unchanged message sends nothing; a formatting-only change is an edit', () => {
  const plain = { body: 'hello' };
  assert.equal(
    isNativeEditUnchanged(plain, { body: 'hello', html: 'hello', htmlDiffers: false }),
    true
  );
  assert.equal(
    isNativeEditUnchanged(plain, { body: 'hello', html: '<b>hello</b>', htmlDiffers: true }),
    false
  );
  const rich = { body: 'hello', formattedBody: '<b>hello</b>' };
  assert.equal(
    isNativeEditUnchanged(rich, { body: 'hello', html: '<b>hello</b>', htmlDiffers: true }),
    true
  );
  assert.equal(
    isNativeEditUnchanged(rich, { body: 'hello', html: 'hello', htmlDiffers: false }),
    false
  );
  assert.equal(
    isNativeEditUnchanged(rich, { body: 'hi', html: '<b>hi</b>', htmlDiffers: true }),
    false
  );
});

test('a draft survives its row scrolling away and is dropped on close', () => {
  keepNativeEditDraft('!r', '$e', ['draft']);
  assert.deepEqual(takeNativeEditDraft('!r', '$e'), ['draft']);
  assert.equal(takeNativeEditDraft('!r', '$e'), undefined);
  keepNativeEditDraft('!r', '$e', ['again']);
  dropNativeEditDraft('!r', '$e');
  assert.equal(takeNativeEditDraft('!r', '$e'), undefined);
  // Bounded: old drafts give way.
  for (let index = 0; index < 25; index += 1) keepNativeEditDraft('!r', `$${index}`, index);
  assert.equal(takeNativeEditDraft('!r', '$0'), undefined);
  assert.equal(takeNativeEditDraft('!r', '$24'), 24);
});

test('editing uses the Slate composer and matrix_edit_message, not a raw HTML form', () => {
  const presenter = readFileSync('src/app/features/room/NativeTimelinePresenter.tsx', 'utf8');
  const editor = readFileSync('src/app/features/room/NativeTimelineMessageEditor.tsx', 'utf8');
  assert.doesNotMatch(
    presenter,
    /Edit message HTML body|org\.matrix\.custom\.html|<textarea\s+value=\{editBody\}/
  );
  assert.doesNotMatch(presenter, /editTextWithNativeTimelineAction/);
  assert.match(presenter, /<NativeTimelineMessageEditor/);
  // The editor lives in the row, so a click outside the menu cannot discard it.
  assert.match(presenter, /const \[editingEventId, setEditingEventId\] = useState<string>\(\)/);
  assert.match(
    presenter,
    /onEdit: editableInPlace \? \(\) => onEditingChange\(eventId\) : undefined/
  );
  assert.match(editor, /<CustomEditor/);
  assert.match(editor, /editMessageWithNativeDesktopOwner\(\{/);
  assert.match(editor, /msgType: nativeEditMsgType\(messageType\)/);
  assert.match(editor, /mentionUserIds: Array\.from\(mentions\.users\)/);
  assert.match(editor, /mentionRoom: mentions\.room/);
  assert.match(editor, /role="alert"/);
  // Core builds the replacement with the msgtype and mentions.
  const core = readFileSync('../crates/synara-core/src/app/send/text.rs', 'utf8');
  assert.match(core, /\("m\.emote", Some\(html\)\) => RoomMessageEventContent::emote_html/);
  assert.match(core, /content\.make_replacement\(ReplacementMetadata::new\(event_id, mentions\)\)/);
});
