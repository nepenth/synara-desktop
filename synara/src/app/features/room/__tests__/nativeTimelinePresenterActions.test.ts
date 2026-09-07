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
    (left, right) => right - left
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

test('message, poll, and sticker rows share Core relation and reaction presentation', () => {
  for (const [start, end] of [
    ["case 'message'", "case 'membership'"],
    ["case 'poll'", "case 'call'"],
    ["case 'sticker'", "case 'pagination'"],
  ]) {
    const branch = presenter.slice(presenter.indexOf(start), presenter.indexOf(end));
    assert.match(branch, /NativeTimelineReplySurface reply=\{row\.reply\}/);
    assert.match(branch, /threadRoot=\{row\.threadRoot\}/);
    assert.match(branch, /thread=\{row\.thread\}/);
    assert.match(branch, /NativeTimelineReactionPills/);
    assert.match(branch, /reactions=\{row\.reactions\}/);
  }
  assert.match(presenter, /nativeThreadFocusEventId\(thread\) \?\? threadRoot/);
  assert.match(presenter, /variant=\{reaction\.own \? 'Primary' : 'Secondary'\}/);
});

test('poll and call actions consume Core capabilities with accessible pending controls', () => {
  assert.match(presenter, /NativeTimelinePollAnswers/);
  assert.match(presenter, /maximumSelections=\{Math\.max\(0, row\.maxSelections \?\? 1\)\}/);
  assert.match(presenter, /nativePollSubmission/);
  assert.match(presenter, /toggleNativePollSelection/);
  assert.match(presenter, /aria-pressed=\{selected\}/);
  assert.match(presenter, /disabled=\{!canVote \|\| closed \|\| submitting\}/);
  assert.match(presenter, /disabled=\{declinePending\}/);
  assert.match(presenter, /callDeclineWithNativeTimelineOwner/);
  assert.doesNotMatch(presenter, /sendEvent\(['"]m\.poll\.response/);
  assert.doesNotMatch(presenter, /sendEvent\(['"]m\.rtc/);
});

test('report and forward await exact native readback and expose safe accessible controls', () => {
  assert.match(presenter, /aria-label="Optional report reason"/);
  assert.match(presenter, /homeserver administrators/);
  assert.match(presenter, /autoFocus/);
  assert.match(presenter, /pendingProductAction/);
  assert.match(presenter, /The forwarded copy will not be protected by room encryption/);
  assert.match(presenter, /disabled=\{pendingProductAction !== undefined\}/);
  assert.match(presenter, /await reportWithNativeTimelineAction/);
  assert.match(presenter, /await forwardTextWithNativeTimelineAction/);
  assert.match(presenter, /await forwardMediaWithNativeTimelineAction/);
  assert.match(presenter, /nativeForwardEncryptionDecision/);
  assert.match(presenter, /sendForward\(forwardConfirm\.roomId, true\)/);
  assert.match(presenter, /confirmedEncryptionDowngrade/);
  assert.match(presenter, /Room encryption status is unavailable\. Forwarding was not started\./);
  assert.match(presenter, /maxLength=\{512\}/);
});

test('transient action surfaces use persistent locks and preserve editable arrow keys', () => {
  assert.match(presenter, /const nativeTimelineActionsInFlight = new Set<string>\(\)/);
  assert.match(
    presenter,
    /nativeTimelineActionFlightKey\(\s*sessionGeneration,\s*roomId,\s*eventId,\s*`reaction:\$\{key\}`/
  );
  assert.match(presenter, /nativePollFlights\.prepare\(actionKey, answerIds\)/);
  assert.match(presenter, /nativePollFlights\.settleDispatch\(actionKey, true\)/);
  assert.match(presenter, /nativePollFlights\.observeProjection/);
  assert.match(presenter, /That reaction is already in progress/);
  assert.match(presenter, /target\.isContentEditable/);
  assert.match(presenter, /target\.tagName === 'INPUT'/);
  assert.match(presenter, /target\.tagName === 'TEXTAREA'/);
  assert.match(presenter, /!isTimelineActionEditableTarget\(event\.target\)/);
});

test('redacted and undecryptable rows retain Core-projected event actions', () => {
  for (const [start, end] of [
    ["case 'redacted'", "case 'encrypted_unavailable'"],
    ["case 'encrypted_unavailable'", "case 'other'"],
  ]) {
    const branch = presenter.slice(presenter.indexOf(start), presenter.indexOf(end));
    assert.match(branch, /NativeTimelineRowActionSurface/);
    assert.match(branch, /capabilities/);
    assert.match(branch, /eventId/);
  }
});

test('Core-classified approval prompts cannot use the generic desktop reaction route', () => {
  assert.match(
    presenter,
    /approvalOwnsReactionActions = row\.kind === 'message' && Boolean\(row\.isAgentApproval\)/
  );
  assert.match(
    presenter,
    /approvalOwnsReactionActions && capabilities \? \{ \.\.\.capabilities, react: false \} : capabilities/
  );
  assert.match(presenter, /if \(!eventId \|\| !genericReactionCapabilities\?\.react\) return/);
  assert.match(presenter, /capabilities: genericReactionCapabilities/);
  assert.match(presenter, /enabled=\{Boolean\(genericReactionCapabilities\?\.react\)\}/);
  assert.match(presenter, /disabled=\{!enabled\}/);
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
  assert.match(presenter, /const jumpToLatest = useCallback\(\(\) =>/);
  assert.match(presenter, /followingLiveRef\.current = true/);
  assert.match(presenter, /onClick=\{jumpToLatest\}/);
  assert.doesNotMatch(presenter, /snapshot\.position\.kind !== 'live_bottom'/);
  assert.match(presenter, /aria-label="Loading older messages"/);
  assert.match(presenter, /aria-label="Loading newer messages"/);

  assert.doesNotMatch(presenter, />\s*Mark read\s*</);
  assert.doesNotMatch(presenter, />\s*Mark unread\s*</);
});

test('native live tail marks the open stream read through the native owner', () => {
  assert.match(presenter, /action: 'mark_read'/);
  assert.match(presenter, /intent: 'automatic_visibility'/);
  assert.match(presenter, /observedLiveTailEventId: liveTailReadTarget/);
  assert.match(presenter, /selectedPosition\.kind === 'live_bottom'/);
  assert.match(presenter, /capabilities\.markRead/);
  assert.doesNotMatch(presenter, /markAsReadInBackground/);
  assert.doesNotMatch(presenter, /sendReadReceipt/);
  assert.doesNotMatch(presenter, /setRoomReadMarkers/);
});

test('native live-tail receipts persist until event-driven painted-bottom proof and cancel cleanly', () => {
  assert.match(presenter, /requestAnimationFrame\(markPaintedTailRead\)/);
  assert.match(
    presenter,
    /scrollEl\.scrollHeight - scrollEl\.scrollTop - scrollEl\.clientHeight <= 8/
  );
  assert.match(presenter, /document\.visibilityState === 'visible'/);
  assert.match(presenter, /document\.hasFocus\(\)/);
  assert.match(presenter, /cancelAnimationFrame\(animationFrame\)/);
  assert.match(presenter, /liveTailMarkGenerationRef\.current !== generation/);
  assert.match(presenter, /new ResizeObserver\(requestPaintCheck\)/);
  assert.match(presenter, /new MutationObserver\(\(\) =>/);
  assert.match(presenter, /scrollEl\.addEventListener\('scroll', requestPaintCheck/);
  assert.doesNotMatch(presenter, /paintAttempts|paintAttempts < \d/);
  assert.doesNotMatch(presenter, /attributes: true/);
});

test('read target comes from the unfiltered native snapshot while paint uses the filtered UI', () => {
  assert.match(
    presenter,
    /latestNativeReadEventId\(readyState\?\.snapshot\.rows\.map\(rowEventId\) \?\? \[\]\)/
  );
  assert.match(presenter, /const rows = useMemo\(\(\) => \{/);
  assert.match(presenter, /hideMembershipEvents && row\.kind === 'membership'/);
});

test('blur before paint can reattach while blur after submission cannot duplicate the receipt', () => {
  const effectStart = presenter.indexOf('if (!liveTailMarkReadKey)');
  const paintProof = presenter.indexOf('if (!paintedAtBottom) return;', effectStart);
  const submittedKeyWrite = presenter.indexOf(
    'liveTailSubmittedKeyRef.current = liveTailMarkReadKey',
    effectStart
  );
  assert.ok(effectStart >= 0);
  assert.ok(paintProof > effectStart);
  assert.ok(submittedKeyWrite > paintProof);
  assert.match(presenter, /if \(liveTailSubmittedKeyRef\.current === liveTailMarkReadKey\) return/);
  assert.match(presenter, /window\.addEventListener\('focus', updateDocumentActive\)/);
  assert.doesNotMatch(
    presenter.slice(effectStart, paintProof),
    /SubmittedKeyRef\.current = liveTailMarkReadKey/
  );
});

test('room read state stays a single contextual overflow action', () => {
  const header = readFileSync('src/app/features/room/RoomViewHeader.tsx', 'utf8');

  assert.match(header, /aria-label="More Options"/);
  assert.match(header, /unread \? 'Mark as Read' : 'Mark as Unread'/);
  assert.match(header, /unread \? Icons\.CheckTwice : Icons\.MessageUnread/);
});

test('native timeline honors hide membership, hide activity receipts, and message spacing', () => {
  assert.match(presenter, /hideMembershipEvents && row\.kind === 'membership'/);
  assert.match(presenter, /hideNickAvatarEvents && row\.kind === 'state'/);
  assert.match(presenter, /hideActivity,/);
  assert.match(presenter, /nativeLiveReadTarget/);
  assert.doesNotMatch(presenter, /if \(!hideActivity\) \{/);
  assert.match(presenter, /messageSpacing=\{messageSpacing\}/);
});

test('native timeline message rows sit on chat chrome and highlight on hover', () => {
  const messageRowCss = htmlCss.slice(
    htmlCss.indexOf('export const MessageRow'),
    htmlCss.indexOf('export const MessageBody')
  );
  const messageBodyCss = htmlCss.slice(
    htmlCss.indexOf('export const MessageBody'),
    htmlCss.indexOf('export const FormattedBody')
  );

  assert.match(messageRowCss, /export const MessageRow = recipe\(/);
  assert.match(messageRowCss, /backgroundColor: color\.SurfaceVariant\.ContainerHover/);
  assert.match(messageRowCss, /borderRadius: config\.radii\.R400/);
  assert.match(messageRowCss, /MessageActionSurface\}:hover/);
  assert.match(messageBodyCss, /background: 'transparent'/);
  assert.match(messageBodyCss, /color: 'var\(--synara-message-foreground\)'/);

  assert.match(presenter, /htmlCss\.MessageRow\(\{/);
  assert.match(presenter, /hasMessageSurface/);
  assert.match(presenter, /groupsNext/);
  assert.match(presenter, /htmlCss\.MessageActionSurface/);
  assert.match(
    readFileSync('src/app/features/room/NativeTimelinePresenter.tsx', 'utf8'),
    /showActionRail = hasActionMenu && actionsActive/
  );
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

test('formatted messages use a readable measure, body size, and layered tables', () => {
  const messageBodyCss = htmlCss.slice(
    htmlCss.indexOf('export const MessageBody'),
    htmlCss.indexOf('export const CodePanel')
  );

  assert.match(messageBodyCss, /maxWidth: toRem\(672\)/);
  assert.match(messageBodyCss, /fontSize: toRem\(16\)/);
  assert.match(messageBodyCss, /lineHeight: 1\.55/);
  assert.match(htmlCss, /borderCollapse: 'separate'/);
  assert.match(htmlCss, /background: 'var\(--synara-rich-text-table-even\)'/);
  assert.match(htmlCss, /background: 'var\(--synara-rich-text-table-odd\)'/);
  assert.match(htmlCss, /background: 'var\(--synara-rich-text-table-header\)'/);
  assert.match(htmlCss, /tbody tr:nth-child\(even\) td/);
});
