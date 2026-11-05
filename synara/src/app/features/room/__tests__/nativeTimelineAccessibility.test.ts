import assert from 'node:assert/strict';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

const read = (path: string) => readFileSync(`src/app/${path}`, 'utf8');
const presenter = read('features/room/NativeTimelinePresenter.tsx');
const timelineCss = read('features/room/nativeTimelineHtml.css.ts');

const tsxFiles = (dir: string): string[] =>
  readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === '__tests__' ? [] : tsxFiles(path);
    return path.endsWith('.tsx') ? [path] : [];
  });

test('the timeline is a labelled region with a polite arrivals log (D58)', () => {
  assert.match(
    presenter,
    /role="region"\s+aria-label=\{threadRootId \? 'Thread messages' : 'Messages'\}/
  );
  assert.match(
    presenter,
    /<div role="log" aria-live="polite" className=\{htmlCss\.VisuallyHidden\}>/
  );
  assert.match(presenter, /nativeTimelineArrivals\(announcedTailRef\.current, rows, ownUserId\)/);
  assert.match(read('features/room/RoomViewHeader.tsx'), /<Text\s+as="h1"/);
  assert.match(read('features/room/Room.tsx'), /<Box as="main"/);
  assert.match(read('components/page/Page.tsx'), /as=\{label \? 'nav' : 'div'\}/);
  // Room-list panes name the landmark from the translated section noun.
  for (const [path, pattern] of [
    // Home is the one list of rooms and DMs (NAV-12); `/direct/` routes share it.
    ['pages/client/home/Home.tsx', /<PageNav label=\{roomsLabel\}>/],
    ['pages/client/inbox/Inbox.tsx', /<PageNav label="Activity">/],
    ['pages/client/explore/Explore.tsx', /<PageNav label="Explore">/],
    ['pages/client/space/Space.tsx', /<PageNav label=\{roomsLabel\}>/],
  ] as const) {
    assert.match(read(path), pattern, path);
  }
  assert.match(read('pages/client/home/Home.tsx'), /t\('modernization\.nav\.rooms', 'Rooms'\)/);
  assert.match(
    read('pages/client/home/Home.tsx'),
    /t\('modernization\.nav\.direct_messages', 'Direct messages'\)/
  );
  assert.match(read('pages/client/SidebarNav.tsx'), /<Sidebar as="nav" aria-label="Spaces">/);
  // Wave 3 rail: places only. Home, the spaces, then Activity, Approvals and
  // the account; create/join/explore are in the list's "+" menu and search is
  // the list's search field (and ⌘K).
  const rail = read('pages/client/SidebarNav.tsx');
  assert.doesNotMatch(rail, /DirectTab|ExploreTab|CreateTab|SearchTab/);
  const order = [
    '<HomeTab />',
    '<SpaceTabs',
    '<InboxTab />',
    '<ApprovalsTab />',
    '<SettingsTab />',
  ];
  const at = order.map((tag) => rail.indexOf(tag));
  assert.ok(
    at.every((index, i) => index >= 0 && (i === 0 || index > at[i - 1])),
    String(at)
  );
  const addMenu = read('pages/client/home/Home.tsx');
  for (const key of ['create_room', 'join_room', 'new_message', 'explore_rooms', 'create_space']) {
    assert.match(addMenu, new RegExp(`modernization\\.nav\\.${key}`), key);
  }
  assert.match(read('components/nav/NavSearchField.tsx'), /searchModalAtom/);
});

test('navigation rows mark the current page, never aria-selected on a plain div (D58)', () => {
  const navItem = read('components/nav/NavItem.tsx');
  assert.match(navItem, /data-selected=\{selected \|\| undefined\}/);
  const offenders = tsxFiles('src/app').filter((path) =>
    /<NavItem\b[^>]*aria-selected=/.test(readFileSync(path, 'utf8'))
  );
  assert.deepEqual(offenders, []);
  // Home and Direct rows come from RoomNavItem, which carries aria-current
  // itself. NavSearchField opens the global search dialog, not a page.
  assert.match(read('components/nav/NavSearchField.tsx'), /aria-haspopup="dialog"/);
  for (const path of [
    'pages/client/explore/Explore.tsx',
    'pages/client/inbox/Inbox.tsx',
    'pages/client/space/Space.tsx',
  ]) {
    assert.match(read(path), /aria-current=\{[^}]+ \? 'page' : undefined\}/, path);
  }
  assert.match(read('features/room-nav/RoomNavItem.tsx'), /selected=\{selected\}/);
  assert.match(read('../index.css'), /\[data-selected='true'\],/);
  assert.match(read('styles/Depth.css.ts'), /\[data-selected=true\]/);
  assert.doesNotMatch(read('pages/client/inbox/Invites.tsx'), /aria-selected/);
});

test('the message menu is a menu with menu items and an expanded state (D63)', () => {
  const actions = presenter.slice(
    presenter.indexOf('const NativeTimelineRowActions = ({'),
    presenter.indexOf('const NativeTimelineUnsentChrome = ({')
  );
  const menuItems = actions.match(/<MenuItem\b/g)?.length ?? 0;
  assert.ok(menuItems >= 10);
  assert.equal(actions.match(/role="menuitem"/g)?.length, menuItems);
  assert.match(actions, /role="menu"\s+aria-label="Message actions"\s+aria-orientation="vertical"/);
  assert.match(actions, /<Line size="300" role="separator"/);
  assert.match(actions, /aria-expanded=\{forwarding\}/);
  assert.match(actions, /aria-expanded=\{reporting\}/);
  assert.match(presenter, /role="toolbar" aria-label="Message tools"/);
  assert.match(
    presenter,
    /aria-label="Add reaction"\s+aria-haspopup="dialog"\s+aria-expanded=\{emojiBoardOpen\}/
  );
  assert.doesNotMatch(presenter, /aria-pressed=\{emojiBoardOpen\}/);
  assert.match(presenter, /isKeyForward: \(event: KeyboardEvent\) =>\s+event\.key === 'ArrowDown'/);
});

test('presenter dialogs and menus use the dialog wrapper and return focus (B8a follow-up)', () => {
  assert.match(presenter, /import \{ Modal \} from '\.\.\/\.\.\/components\/modal-dialog';/);
  assert.doesNotMatch(presenter, /^\s+Modal,$/m);
  assert.doesNotMatch(presenter, /returnFocusOnDeactivate/);
  assert.match(presenter, /returnFocusOrFallback\(\(\) => surfaceRef\.current\)/);
  assert.match(presenter, /setReturnFocus: returnFocusFromMenu/);
  assert.match(presenter, /setReturnFocus=\{returnFocusToSurface\}/);
  assert.match(presenter, /<Modal variant="Surface" size="300" aria-label="Reactions">/);
  assert.match(
    read('components/emoji-board/EmojiBoard.tsx'),
    /\.\.\.\(setReturnFocus \? \{ setReturnFocus \} : \{\}\)/
  );
});

test('motion respects prefers-reduced-motion (D64)', () => {
  assert.match(presenter, /const behavior = preferredScrollBehavior\(\);/);
  assert.doesNotMatch(presenter, /behavior: 'smooth'/);
  assert.match(read('utils/motion.ts'), /\(prefers-reduced-motion: reduce\)/);
  for (const path of [
    'pages/client/sidebar/UnverifiedTab.css.ts',
    'components/typing-indicator/TypingIndicator.css.ts',
    'features/room/RoomViewTyping.css.ts',
  ]) {
    assert.match(read(path), /'\(prefers-reduced-motion: reduce\)': \{\s+animation: 'none'/, path);
  }
  assert.match(
    read('../index.css'),
    /@media \(prefers-reduced-motion: reduce\) \{\s+\[role='dialog'\],[\s\S]*?animation: none !important;/
  );
});

test('rail buttons and the message row show keyboard focus (V10)', () => {
  assert.match(
    timelineCss,
    /globalStyle\(`\$\{MessageActionRail\} button:focus-visible`, \{\s+outline:/
  );
  assert.match(
    timelineCss,
    /globalStyle\(`\$\{MessageActionSurface\}:focus-visible`, \{\s+outline:/
  );
});

test('settings controls have names (B8a follow-up)', () => {
  assert.match(
    read('features/settings/notifications/Notifications.tsx'),
    /<Input\s+aria-label="New keyword"/
  );
  assert.match(
    read('pages/client/home/Home.tsx'),
    /aria-label=\{t\('modernization\.nav\.home_options', 'Home options'\)\}/
  );
  assert.match(read('pages/client/space/Space.tsx'), /aria-label=\{`\$\{spaceName\} options`\}/);
  // Every folds Switch is named: by SettingTile when it is the tile's `after`,
  // otherwise by its own aria-label.
  const unnamed: string[] = [];
  for (const path of tsxFiles('src/app')) {
    const text = readFileSync(path, 'utf8');
    if (!/import \{[^}]*\bSwitch\b[^}]*\} from 'folds'/.test(text)) continue;
    for (const match of text.matchAll(/<Switch\b[^>]*?\/>/gs)) {
      const before = text.slice(Math.max(0, (match.index ?? 0) - 40), match.index);
      if (/aria-label/.test(match[0]) || /after=\{\s*$/.test(before)) continue;
      unnamed.push(`${path}:${text.slice(0, match.index).split('\n').length}`);
    }
  }
  assert.deepEqual(unnamed, []);
});

test('settings that did nothing in the timeline now apply (D16)', () => {
  assert.match(presenter, /useSetting\(settingsAtom, 'mediaAutoLoad'\)/);
  assert.match(presenter, /data-native-timeline-media-deferred=\{kind\}/);
  assert.match(presenter, /preload=\{mediaAutoLoad \? undefined : 'none'\}/);
  assert.match(presenter, /useSetting\(settingsAtom, 'legacyUsernameColor'\)/);
  assert.match(presenter, /\{ color: colorMXID\(rowSenderId\(row\) \?\? ''\) \}/);
  assert.match(presenter, /useSetting\(settingsAtom, 'showHiddenEvents'\)/);
});

test('room lists show loading until their first readback (D29)', () => {
  const roomList = read('state/room-list/roomList.ts');
  assert.match(roomList, /export const useNativeRoomListLoaded/);
  assert.match(
    roomList,
    /if \(loadedSessionGeneration !== session\.sessionGeneration\) setLoaded\(false\)/
  );
  for (const [path, key, label] of [
    ['pages/client/home/Home.tsx', 'loading_rooms', 'Loading rooms…'],
    ['pages/client/space/Space.tsx', 'loading_space', 'Loading space…'],
  ]) {
    assert.match(
      read(path),
      new RegExp(
        `roomListLoaded \\? \\([\\s\\S]*?<NavLoading\\s+label=\\{t\\('modernization\\.nav\\.${key}', '${label}'\\)\\}\\s*/>`
      ),
      path
    );
  }
  assert.match(presenter, /<Text size="T300">Loading messages…<\/Text>/);
  assert.match(read('components/nav/NavEmptyLayout.tsx'), /role="status"/);
});
