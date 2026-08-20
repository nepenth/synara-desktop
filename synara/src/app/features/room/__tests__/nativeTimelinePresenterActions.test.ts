import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const presenter = readFileSync('src/app/features/room/NativeTimelinePresenter.tsx', 'utf8');

test('native timeline rows show timestamps and formatted HTML without the legacy event graph', () => {
  assert.match(presenter, /rowOriginServerTs/);
  assert.match(presenter, /<Time\s/);
  assert.match(presenter, /NativeFormattedBody/);
  assert.match(presenter, /htmlCss\.MessageBody/);
  assert.match(presenter, /NativeTimelineSenderAvatar/);
  assert.match(presenter, /followingLiveRef/);
  assert.doesNotMatch(presenter, /dangerouslySetInnerHTML/);
});

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

test('native timeline navigation uses contextual controls and edge pagination', () => {
  assert.match(presenter, /scrollEl\.scrollTop <= 96/);
  assert.match(presenter, /distanceFromBottom <= 96/);
  assert.match(presenter, /aria-label="Jump to latest"/);
  assert.match(presenter, /Icons\.ChevronBottom/);
  assert.match(
    presenter,
    /shouldShowJumpToLatest\(readyState\.selectedPosition\.kind, atLiveBottom\)/
  );
  assert.match(presenter, /const jumpToLatest = \(\) =>/);
  assert.match(presenter, /followingLiveRef\.current = true/);
  assert.match(presenter, /onClick=\{jumpToLatest\}/);
  assert.doesNotMatch(presenter, /snapshot\.position\.kind !== 'live_bottom'/);
  assert.match(presenter, /aria-label="Loading older messages"/);
  assert.match(presenter, /aria-label="Loading newer messages"/);

  assert.doesNotMatch(presenter, />\s*Mark read\s*</);
  assert.doesNotMatch(presenter, />\s*Mark unread\s*</);
  assert.doesNotMatch(presenter, />\s*Load older messages\s*</);
  assert.doesNotMatch(presenter, />\s*Load newer messages\s*</);
});

test('room read state stays a single contextual overflow action', () => {
  const header = readFileSync('src/app/features/room/RoomViewHeader.tsx', 'utf8');

  assert.match(header, /aria-label="More Options"/);
  assert.match(header, /unread \? 'Mark as Read' : 'Mark as Unread'/);
  assert.match(header, /unread \? Icons\.CheckTwice : Icons\.MessageUnread/);
});
