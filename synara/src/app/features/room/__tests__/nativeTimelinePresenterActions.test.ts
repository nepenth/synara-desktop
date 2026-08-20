import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const presenter = readFileSync('src/app/features/room/NativeTimelinePresenter.tsx', 'utf8');
const htmlCss = readFileSync('src/app/features/room/nativeTimelineHtml.css.ts', 'utf8');

const hexChannel = (hex: string, index: number): number => {
  const value = parseInt(hex.slice(1 + index * 2, 3 + index * 2), 16) / 255;
  return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
};

const relativeLuminance = (hex: string): number =>
  0.2126 * hexChannel(hex, 0) + 0.7152 * hexChannel(hex, 1) + 0.0722 * hexChannel(hex, 2);

const contrastRatio = (foreground: string, background: string): number => {
  const [higher, lower] = [relativeLuminance(foreground), relativeLuminance(background)].sort(
    (left, right) => right - left,
  );
  return (higher + 0.05) / (lower + 0.05);
};

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
    /shouldShowJumpToLatest\(readyState\.selectedPosition\.kind, atLiveBottom\)/,
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

test('native timeline message rows use a full-width surface, not a text-only chip', () => {
  const messageRowCss = htmlCss.slice(
    htmlCss.indexOf('export const MessageRow'),
    htmlCss.indexOf('export const MessageBody'),
  );
  const messageBodyCss = htmlCss.slice(
    htmlCss.indexOf('export const MessageBody'),
    htmlCss.indexOf('export const FormattedBody'),
  );

  assert.match(messageRowCss, /export const MessageRow = recipe\(/);
  assert.match(messageRowCss, /backgroundColor: color\.SurfaceVariant\.ContainerHover/);
  assert.match(messageRowCss, /backgroundColor: color\.SurfaceVariant\.ContainerActive/);
  assert.match(messageRowCss, /borderRadius: config\.radii\.R400/);
  assert.match(messageRowCss, /marginLeft: config\.space\.S400/);
  assert.match(messageRowCss, /marginRight: config\.space\.S400/);
  assert.match(messageRowCss, /color: color\.SurfaceVariant\.OnContainer/);
  assert.doesNotMatch(messageRowCss, /['"]transparent['"]/);
  assert.match(messageBodyCss, /background: 'transparent'/);
  assert.match(messageBodyCss, /color: color\.SurfaceVariant\.OnContainer/);

  assert.match(presenter, /htmlCss\.MessageRow\(\{/);
  assert.match(presenter, /hasMessageSurface/);
  assert.match(presenter, /groupsNext/);
  assert.match(presenter, /htmlCss\.MessageActionSurface/);
  assert.doesNotMatch(presenter, /htmlCss\.MessageBody[\s\S]{0,80}backgroundColor/);
});

test('message surface stays readable against OnContainer in light and dark', () => {
  const lightChat = '#FFFFFF';
  const lightPanel = '#F2F3F5';
  const lightHover = '#E8EAED';
  const lightOn = '#060607';
  const darkChat = '#313338';
  const darkPanel = '#383A40';
  const darkHover = '#404249';
  const darkOn = '#F2F3F5';
  const colors = readFileSync('src/colors.css.ts', 'utf8');

  assert.match(colors, /ContainerHover: '#F2F3F5'/);
  assert.match(colors, /ContainerHover: '#383A40'/);
  assert.ok(relativeLuminance(lightPanel) < relativeLuminance(lightChat));
  assert.ok(relativeLuminance(darkPanel) > relativeLuminance(darkChat));
  assert.ok(contrastRatio(lightOn, lightPanel) >= 7);
  assert.ok(contrastRatio(lightOn, lightHover) >= 7);
  assert.ok(contrastRatio(darkOn, darkPanel) >= 7);
  assert.ok(contrastRatio(darkOn, darkHover) >= 7);
});
