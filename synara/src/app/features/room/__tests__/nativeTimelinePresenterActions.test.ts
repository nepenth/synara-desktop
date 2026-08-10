import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const presenter = readFileSync('src/app/features/room/NativeTimelinePresenter.tsx', 'utf8');

test('native timeline rows retain hover/focus action access without restoring the legacy owner', () => {
  assert.match(presenter, /NativeTimelineRowActionSurface/);
  assert.match(presenter, /useHover\(\{ onHoverChange: setHovered \}\)/);
  assert.match(presenter, /useFocusWithin\(\{ onFocusWithinChange: setFocusWithin \}\)/);
  assert.match(presenter, /Icons\.SmilePlus/);
  assert.match(presenter, /Icons\.VerticalDots/);
  assert.match(presenter, /EmojiBoard/);
  assert.match(presenter, /aria-label="Add reaction"/);
  assert.match(presenter, /addToRecentEmoji=\{false\}/);
  assert.match(presenter, /onEmojiSelect/);
  assert.match(presenter, /onCustomEmojiSelect/);
  assert.match(presenter, /aria-label="More message actions"/);
  assert.match(presenter, /aria-haspopup="menu"/);
  assert.match(presenter, /data-native-timeline-action-menu="true"/);

  for (const action of [
    'Reply',
    'Reply in thread',
    'Edit',
    'Forward',
    'Redact',
    'Report',
    'Pin',
    'Save for later',
  ]) {
    assert.equal(presenter.includes(action), true, `missing ${action} action`);
  }

  for (const nativeOwner of [
    'toggleReactionWithNativeOwner',
    'setNativeComposerReplyDraft',
    'editTextWithNativeTimelineAction',
    'forwardTextWithNativeTimelineAction',
    'redactWithNativeTimelineAction',
    'reportWithNativeTimelineAction',
    'pinWithNativeTimelineAction',
    'upsertLaterWithNativeOwner',
  ]) {
    assert.match(presenter, new RegExp(nativeOwner));
  }

  assert.doesNotMatch(presenter, /matrix-js-sdk/);
  assert.doesNotMatch(presenter, /from ['"][^'"]*message\/Message['"]/);
  assert.doesNotMatch(presenter, /from ['"][^'"]*RoomTimeline['"]/);
});
