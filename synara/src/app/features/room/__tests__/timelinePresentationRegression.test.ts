import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

test('composer leading action uses one circular affordance', () => {
  const input = readFileSync('src/app/features/room/RoomInput.tsx', 'utf8');
  const action = input.slice(
    input.indexOf('ref={composerToolsBtnRef}'),
    input.indexOf('</IconButton>', input.indexOf('ref={composerToolsBtnRef}'))
  );

  assert.match(action, /variant="Surface"/);
  assert.match(action, /radii="Pill"/);
  assert.match(action, /Icons\.Plus/);
  assert.doesNotMatch(action, /PlusCircle/);
});

test('native timeline owner hydrates unresolved sender profiles without blocking diffs', () => {
  const live = readFileSync('../crates/synara-core/src/app/timeline/live.rs', 'utf8');

  assert.match(live, /let hydrate_sender_profiles = items/);
  assert.match(live, /timeline\.fetch_members\(\)\.await/);
  assert.match(live, /tokio::select!/);
  assert.match(live, /\(\) = &mut member_hydration, if !members_hydrated/);
});

test('desktop grouped timestamps use horizontal trackpad movement without stealing selection', () => {
  const presenter = readFileSync('src/app/features/room/NativeTimelinePresenter.tsx', 'utf8');
  const timelineCss = readFileSync('src/app/features/room/nativeTimelineHtml.css.ts', 'utf8');

  assert.match(presenter, /GroupedTimestampReveal/);
  assert.match(presenter, /groupedTimestampOffset/);
  assert.match(presenter, /onWheel/);
  assert.match(presenter, /event\.deltaMode/);
  assert.match(presenter, /event\.deltaX/);
  assert.match(presenter, /wheelResetTimer/);
  assert.match(presenter, /event\.pointerType === 'mouse'/);
  assert.match(timelineCss, /touchAction: 'pan-y'/);
  assert.match(timelineCss, /overscrollBehaviorX: 'contain'/);
  assert.doesNotMatch(presenter, /swipeStartX/);
  assert.doesNotMatch(presenter, /grouped \? \(\s*originServerTs \? \(\s*<Time/);
});

test('native media rendering keeps Matrix filename and caption fields distinct', () => {
  const contract = readFileSync('src/app/features/room/nativeTimelineView.ts', 'utf8');
  const presenter = readFileSync('src/app/features/room/NativeTimelinePresenter.tsx', 'utf8');

  assert.match(contract, /mediaFilename\?: string/);
  assert.match(contract, /mediaCaption\?: string/);
  assert.match(presenter, /filename=\{row\.mediaFilename\}/);
  assert.match(presenter, /caption=\{row\.mediaCaption\}/);
  assert.match(presenter, /formattedCaption=\{row\.formattedBody\}/);
  assert.doesNotMatch(presenter, /<NativeTimelineMedia[\s\S]{0,240}body=\{row\.body\}/);
});
