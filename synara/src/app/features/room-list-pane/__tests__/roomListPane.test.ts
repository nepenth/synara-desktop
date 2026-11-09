import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  roomListFocusTarget,
  roomListShortcut,
  roomListToggleBadge,
  roomListToggleLabel,
} from '../roomListPane';

test('the toggle badge follows the rail rule, only while the list is hidden', () => {
  assert.deepEqual(roomListToggleBadge(false, true, 3), { kind: 'none' });
  assert.deepEqual(roomListToggleBadge(true, false, 0), { kind: 'none' });
  assert.deepEqual(roomListToggleBadge(true, true, 0), { kind: 'dot' });
  assert.deepEqual(roomListToggleBadge(true, true, 4), { kind: 'count', count: 4 });
});

test('the toggle names its action, the hidden unread and its shortcut', () => {
  assert.equal(roomListToggleLabel(false, { kind: 'none' }), 'Hide room list');
  assert.equal(roomListToggleLabel(true, { kind: 'none' }), 'Show room list');
  assert.equal(roomListToggleLabel(true, { kind: 'dot' }), 'Show room list, unread messages');
  assert.equal(
    roomListToggleLabel(true, { kind: 'count', count: 1 }),
    'Show room list, 1 mention or direct message'
  );
  assert.equal(
    roomListToggleLabel(true, { kind: 'count', count: 5 }),
    'Show room list, 5 mentions and direct messages'
  );
  assert.deepEqual(roomListShortcut(true), { label: '⌘⇧D', ariaKeyShortcuts: 'Meta+Shift+D' });
  assert.deepEqual(roomListShortcut(false), {
    label: 'Ctrl+Shift+D',
    ariaKeyShortcuts: 'Control+Shift+D',
  });
});

test('revealing the list focuses the open room, else the search field', () => {
  const pane = (matches: Record<string, string>) =>
    ({
      querySelector: (selector: string) => matches[selector] ?? null,
    }) as unknown as ParentNode;
  const room = '[aria-current="page"]';
  const search = '[data-testid="nav-search-field"]';
  assert.equal(roomListFocusTarget(pane({ [room]: 'room', [search]: 'search' })), 'room');
  assert.equal(roomListFocusTarget(pane({ [search]: 'search' })), 'search');
  assert.equal(roomListFocusTarget(pane({})), null);
});

test('every client pane can collapse, and the rail carries the toggle on desktop', () => {
  const router = readFileSync('src/app/pages/Router.tsx', 'utf8');
  assert.equal(router.match(/<PageRoot\s+collapsibleNav/g)?.length, 4);
  const rail = readFileSync('src/app/pages/client/SidebarNav.tsx', 'utf8');
  assert.ok(rail.indexOf('<RoomListTab />') < rail.indexOf('<HomeTab />'));
  const shortcuts = readFileSync(
    'src/app/features/desktop-keyboard/DesktopAppShortcuts.tsx',
    'utf8'
  );
  assert.ok(shortcuts.includes("case 'toggle-room-list':"));
});
