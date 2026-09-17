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

test('composer plus menu dismisses on outside click', () => {
  const input = readFileSync('src/app/features/room/RoomInput.tsx', 'utf8');
  const tools = input.slice(
    input.indexOf('anchor={composerToolsAnchor}'),
    input.indexOf('ref={composerToolsBtnRef}')
  );

  assert.match(tools, /<FocusTrap/);
  assert.match(tools, /clickOutsideDeactivates: true/);
  assert.match(tools, /onDeactivate: \(\) => setComposerToolsAnchor\(undefined\)/);
  assert.match(tools, /returnFocusOnDeactivate: false/);
  assert.match(input, /data-composer-tools-menu="true"/);
  assert.match(input, /document\.addEventListener\('pointerdown', onPointerDown, true\)/);
});

test('room overflow Invite is a quiet surface option, not an accent-selected Primary item', () => {
  const header = readFileSync('src/app/features/room/RoomViewHeader.tsx', 'utf8');
  const inviteStart = header.indexOf('onClick={handleInvite}');
  const invite = header.slice(inviteStart, header.indexOf('onClick={handleCopyLink}', inviteStart));

  assert.match(invite, /quietInteractiveSurface/);
  assert.match(invite, /disabled=\{!canInvite\}/);
  assert.doesNotMatch(invite, /variant="Primary"/);
  assert.doesNotMatch(invite, /fill="None"/);

  const nav = readFileSync('src/app/features/room-nav/RoomNavItem.tsx', 'utf8');
  const navInviteStart = nav.indexOf('onClick={handleInvite}');
  const navInvite = nav.slice(
    navInviteStart,
    nav.indexOf('onClick={handleCopyLink}', navInviteStart)
  );
  assert.match(navInvite, /quietInteractiveSurface/);
  assert.doesNotMatch(navInvite, /fill="None"/);
  assert.doesNotMatch(navInvite, /variant="Primary"/);

  const space = readFileSync('src/app/pages/client/space/Space.tsx', 'utf8');
  const spaceInviteStart = space.indexOf('onClick={handleInvite}');
  const spaceInvite = space.slice(
    spaceInviteStart,
    space.indexOf('onClick={handleCopyLink}', spaceInviteStart)
  );
  assert.match(spaceInvite, /quietInteractiveSurface/);
  assert.doesNotMatch(spaceInvite, /fill="None"/);
  assert.doesNotMatch(spaceInvite, /variant="Primary"/);

  const hierarchy = readFileSync('src/app/features/lobby/HierarchyItemMenu.tsx', 'utf8');
  const hierarchyInviteStart = hierarchy.indexOf('onClick={handleInvite}');
  const hierarchyInvite = hierarchy.slice(
    hierarchyInviteStart,
    hierarchy.indexOf('disabled={disabled || !room}', hierarchyInviteStart) + 80
  );
  assert.match(hierarchyInvite, /quietInteractiveSurface/);
  assert.doesNotMatch(hierarchyInvite, /fill="None"/);

  const lobby = readFileSync('src/app/features/lobby/LobbyHeader.tsx', 'utf8');
  const lobbyInviteStart = lobby.indexOf('onClick={handleInvite}');
  const lobbyInvite = lobby.slice(
    lobbyInviteStart,
    lobby.indexOf('onClick={handleRoomSettings}', lobbyInviteStart)
  );
  assert.match(lobbyInvite, /quietInteractiveSurface/);
  assert.doesNotMatch(lobbyInvite, /fill="None"/);
  assert.doesNotMatch(lobbyInvite, /variant="Primary"/);

  const tabs = readFileSync('src/app/pages/client/sidebar/SpaceTabs.tsx', 'utf8');
  const tabsInviteStart = tabs.indexOf('onClick={handleInvite}');
  const tabsInvite = tabs.slice(
    tabsInviteStart,
    tabs.indexOf('onClick={handleCopyLink}', tabsInviteStart)
  );
  assert.match(tabsInvite, /quietInteractiveSurface/);
  assert.doesNotMatch(tabsInvite, /fill="None"/);
  assert.doesNotMatch(tabsInvite, /variant="Primary"/);
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
