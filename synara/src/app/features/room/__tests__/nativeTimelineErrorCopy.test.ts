import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  nativeTimelineActionErrorMessage,
  nativeTimelineLoadErrorMessage,
  TIMELINE_ACTION_FAILED,
  TIMELINE_LOAD_FAILED,
} from '../nativeTimelineErrorCopy';
import { nativeTimelineCommandError } from '../nativeTimelineView';

const read = (path: string) => readFileSync(`src/app/features/room/${path}`, 'utf8');

test('timeline load errors use the Core diagnostic, never "native" wording (D12, D24)', () => {
  const roomGone = nativeTimelineCommandError({
    code: 'NotFound',
    message: 'The native Matrix timeline is not available.',
    diagnosticId: 'd0.3-timeline-room-not-found',
  });
  assert.match(nativeTimelineLoadErrorMessage(roomGone), /This room is not available any more/);
  // A room sync has not delivered yet is retryable, not gone.
  const notYetSynced = nativeTimelineCommandError({
    code: 'Unknown',
    message: 'The native Matrix timeline is unavailable.',
    diagnosticId: 'v-timeline-room-not-yet-synced',
  });
  assert.equal(
    nativeTimelineLoadErrorMessage(notYetSynced),
    'Synara is still loading this room. Try again in a moment.'
  );
  const noSession = { diagnosticId: 'd0.3-timeline-requires-session' };
  assert.match(nativeTimelineLoadErrorMessage(noSession), /Sign in again/);
  // An unknown Core failure keeps its code for a bug report.
  assert.equal(
    nativeTimelineLoadErrorMessage({
      message: 'The native Matrix timeline request is invalid.',
      diagnosticId: 'd0.3-timeline-invalid-room-id',
    }),
    `${TIMELINE_LOAD_FAILED} (Error code: d0.3-timeline-invalid-room-id)`
  );
  // Renderer-internal errors are replaced, not shown.
  assert.equal(
    nativeTimelineLoadErrorMessage(new Error('Native timeline stream lost synchronization.')),
    TIMELINE_LOAD_FAILED
  );
});

test('row action errors keep Core reasons and replace internal copy', () => {
  assert.equal(
    nativeTimelineActionErrorMessage(
      { diagnosticId: 'v-timeline-redact-forbidden' },
      TIMELINE_ACTION_FAILED.redact
    ),
    'You do not have permission to do that in this room.'
  );
  assert.equal(
    nativeTimelineActionErrorMessage(
      new Error('Native Matrix timeline action is unavailable.'),
      TIMELINE_ACTION_FAILED.pin
    ),
    TIMELINE_ACTION_FAILED.pin
  );
  assert.equal(
    nativeTimelineActionErrorMessage(new Error('The event was already redacted.'), 'x'),
    'The event was already redacted.'
  );
  for (const copy of Object.values(TIMELINE_ACTION_FAILED)) {
    assert.doesNotMatch(copy, /\bnative\b/i);
  }
});

test('the presenter shows no "native" copy and styles action errors as alerts (D24, D95)', () => {
  const presenter = read('NativeTimelinePresenter.tsx');
  assert.doesNotMatch(presenter, /'Native [^']*'/);
  // No user-facing literal names the implementation.
  assert.doesNotMatch(presenter, /Opening native timeline|The native timeline is unavailable/);
  assert.doesNotMatch(presenter, /'[^'\n]*\bnative (timeline|matrix)\b[^'\n]*'/i);
  assert.doesNotMatch(presenter, /error instanceof Error \? error\.message/);
  assert.doesNotMatch(presenter, /\{actionError && <Text/);
  const notice = read('NativeTimelineNotice.tsx');
  assert.match(presenter, /<NativeTimelineNotice/);
  assert.match(notice, /role=\{tone === 'error' \? 'alert' : 'status'\}/);
  assert.match(notice, /className=\{htmlCss\.TimelineNotice\(\{ tone \}\)\}/);
  assert.match(notice, /aria-label="Dismiss"/);
  assert.match(read('nativeTimelineHtml.css.ts'), /TimelineNotice = recipe\(/);
  assert.doesNotMatch(read('nativeLinkUnfurl.ts'), /Native Matrix URL preview/);
  assert.doesNotMatch(read('NativeTimelineMarkdownPreview.tsx'), /error instanceof Error/);
  // The room row's notification icon has a spoken name, not the enum value.
  const roomNav = readFileSync('src/app/features/room-nav/RoomNavItem.tsx', 'utf8');
  assert.doesNotMatch(roomNav, /aria-label=\{notificationMode\}/);
  assert.match(
    roomNav,
    /\[RoomNotificationMode\.SpecialMessages\]: 'Notifications: mentions and keywords'/
  );
});

test('the error view retries and can leave a focused or restored position (D12)', () => {
  const presenter = read('NativeTimelinePresenter.tsx');
  const view = read('nativeTimelineView.ts');
  assert.match(presenter, /data-native-timeline-error="true"/);
  assert.match(presenter, /nativeTimelineLoadErrorMessage\(timelineState\.error\)/);
  assert.match(presenter, /onClick=\{reload\}/);
  assert.match(presenter, /\{threadRootId \? 'Back to room' : 'Go to latest'\}/);
  // A failed focused open falls back to the live room with a notice.
  assert.match(
    presenter,
    /timelineState\.status === 'error' && input\.position\.kind === 'focused'/
  );
  assert.match(presenter, /setNotice\(\{ tone: 'info', message: FOCUSED_OPEN_FAILED \}\)/);
  // A stream gap keeps the rows it has and fetches a snapshot at once.
  const gap = view.slice(
    view.indexOf('if (applied.gap) {'),
    view.indexOf('if (applied.gap) {') + 900
  );
  assert.doesNotMatch(gap, /status: 'error'/);
  assert.match(gap, /resyncAfterGap\(\)/);
  assert.match(
    view,
    /if \(!adopted && !disposed && streamIdRef\.current === streamId\) setStreamGap\('failed'\)/
  );
  assert.match(
    view,
    /\}, \[acceptSnapshot, beginOpen, finishOpen, nativeRequest, reloadToken\]\);/
  );
  assert.match(presenter, /streamGap === 'failed'/);
  assert.match(presenter, /action=\{\{ label: 'Reload', onClick: reload \}\}/);
});
