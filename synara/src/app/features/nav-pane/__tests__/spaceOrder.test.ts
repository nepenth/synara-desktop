import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import type { ISidebarFolder, SidebarItems } from '../../../hooks/useSidebarItems';
import {
  SWITCHER_FILTER_THRESHOLD,
  addSpacesToFolder,
  folderLabel,
  groupSpacesIntoFolder,
  moveSpaceItem,
  removeSpaceFromFolder,
  renameFolder,
  reorderSpaceItems,
  spaceCount,
  switcherRows,
  ungroupFolder,
} from '../spaceOrder';
import { otherSpacesUnread } from '../switcherUnread';

// The space switcher keeps the rail's drag rules (SpaceTabs), ported case by
// case, except drop-onto-to-create-folder, which moved to Manage spaces (D1).

const folder = (id: string, content: string[], name?: string): ISidebarFolder =>
  name ? { id, name, content } : { id, content };
const newId = () => 'new-folder';

test('reorder: a top-level space above or below another', () => {
  const items: SidebarItems = ['a', 'b', 'c'];
  assert.deepEqual(reorderSpaceItems(items, 'c', 'a', 'reorder-above', newId).items, [
    'c',
    'a',
    'b',
  ]);
  assert.deepEqual(reorderSpaceItems(items, 'a', 'c', 'reorder-below', newId).items, [
    'b',
    'c',
    'a',
  ]);
});

test('reorder: a closed folder moves as a whole', () => {
  const f = folder('f', ['x', 'y']);
  const items: SidebarItems = ['a', f, 'b'];
  assert.deepEqual(reorderSpaceItems(items, { folder: f }, 'b', 'reorder-below', newId).items, [
    'a',
    'b',
    f,
  ]);
  assert.deepEqual(reorderSpaceItems(items, 'b', { folder: f }, 'reorder-above', newId).items, [
    'a',
    'b',
    f,
  ]);
});

test('reorder: a space dragged out of a folder, and an emptied folder is dropped', () => {
  const f = folder('f', ['x', 'y']);
  const edit = reorderSpaceItems(
    ['a', f],
    { folder: f, spaceId: 'x' },
    'a',
    'reorder-above',
    newId
  );
  assert.deepEqual(edit.items, ['x', 'a', folder('f', ['y'])]);
  assert.deepEqual(edit.removedFolderIds, []);

  const single = folder('s', ['x']);
  const emptied = reorderSpaceItems(
    ['a', single],
    { folder: single, spaceId: 'x' },
    'a',
    'reorder-below',
    newId
  );
  assert.deepEqual(emptied.items, ['a', 'x']);
  assert.deepEqual(emptied.removedFolderIds, ['s']);
});

test('reorder: a space dropped beside a space inside an open folder joins it there', () => {
  const f = folder('f', ['x', 'y']);
  assert.deepEqual(
    reorderSpaceItems(['a', f], 'a', { folder: f, spaceId: 'y' }, 'reorder-above', newId).items,
    [folder('f', ['x', 'a', 'y'])]
  );
  assert.deepEqual(
    reorderSpaceItems(['a', f], 'a', { folder: f, spaceId: 'y' }, 'reorder-below', newId).items,
    [folder('f', ['x', 'y', 'a'])]
  );
});

test('reorder: within one folder', () => {
  const f = folder('f', ['x', 'y', 'z']);
  assert.deepEqual(
    reorderSpaceItems(
      [f],
      { folder: f, spaceId: 'z' },
      { folder: f, spaceId: 'x' },
      'reorder-above',
      newId
    ).items,
    [folder('f', ['z', 'x', 'y'])]
  );
});

test('reorder: a space placed beside its own folder leaves it', () => {
  const f = folder('f', ['x', 'y']);
  assert.deepEqual(
    reorderSpaceItems(['a', f], { folder: f, spaceId: 'x' }, { folder: f }, 'reorder-below', newId)
      .items,
    ['a', folder('f', ['y']), 'x']
  );
  const single = folder('s', ['x']);
  const edit = reorderSpaceItems(
    [single],
    { folder: single, spaceId: 'x' },
    { folder: single },
    'reorder-above',
    newId
  );
  assert.deepEqual(edit.items, ['x']);
  assert.deepEqual(edit.removedFolderIds, ['s']);
});

test('make-child still folds in for callers that allow it (the switcher blocks it)', () => {
  assert.deepEqual(reorderSpaceItems(['a', 'b'], 'b', 'a', 'make-child', newId).items, [
    folder('new-folder', ['a', 'b']),
  ]);
  const switcher = readFileSync('src/app/features/nav-pane/SpaceSwitcher.tsx', 'utf8');
  assert.match(switcher, /const block: Instruction\['type'\]\[\] = \['reparent', 'make-child'\]/);
  assert.match(switcher, /instructionType === 'make-child'\) return/);
});

test('move up and down, at the top level and inside a folder', () => {
  const f = folder('f', ['x', 'y']);
  const items: SidebarItems = ['a', f, 'b'];
  assert.deepEqual(moveSpaceItem(items, 'b', -1), ['a', 'b', f]);
  assert.deepEqual(moveSpaceItem(items, 'f', -1), [f, 'a', 'b']);
  assert.equal(moveSpaceItem(items, 'a', -1), items);
  assert.equal(moveSpaceItem(items, 'b', 1), items);
  assert.deepEqual(moveSpaceItem(items, 'y', -1), ['a', folder('f', ['y', 'x']), 'b']);
  assert.deepEqual(moveSpaceItem(items, 'y', 1), items);
});

test('Manage spaces: group, add, take out, ungroup and rename', () => {
  const grouped = groupSpacesIntoFolder(['a', 'b', 'c', 'd'], ['c', 'b'], 'g', ' Work ');
  assert.deepEqual(grouped, ['a', folder('g', ['b', 'c'], 'Work'), 'd']);
  // Spaces already in a folder stay where they are.
  assert.deepEqual(groupSpacesIntoFolder([folder('f', ['x'])], ['x'], 'g'), [folder('f', ['x'])]);
  assert.deepEqual(groupSpacesIntoFolder(['a'], ['a'], 'g', '  '), [folder('g', ['a'])]);

  assert.deepEqual(addSpacesToFolder(['a', folder('f', ['x']), 'b'], ['b', 'a'], 'f'), [
    folder('f', ['x', 'a', 'b']),
  ]);
  assert.deepEqual(removeSpaceFromFolder(['a', folder('f', ['x', 'y'])], 'x'), [
    'a',
    folder('f', ['y']),
    'x',
  ]);
  assert.deepEqual(removeSpaceFromFolder([folder('f', ['x'])], 'x'), ['x']);
  assert.deepEqual(ungroupFolder(['a', folder('f', ['x', 'y']), 'b'], 'f'), ['a', 'x', 'y', 'b']);
  assert.deepEqual(renameFolder([folder('f', ['x'])], 'f', 'Clients'), [
    folder('f', ['x'], 'Clients'),
  ]);
  assert.deepEqual(renameFolder([folder('f', ['x'], 'Old')], 'f', ''), [folder('f', ['x'])]);
});

test('switcher rows: folders are collapsible groups; a filter flattens to matches', () => {
  const f = folder('f', ['x', 'y']);
  const names: Record<string, string> = { a: 'Alpha', x: 'Café', y: 'Yarn', b: 'Beta' };
  const nameOf = (id: string) => names[id] ?? '';
  assert.deepEqual(switcherRows(['a', f, 'b'], new Set(), '', nameOf), [
    { kind: 'space', spaceId: 'a', depth: 0 },
    { kind: 'folder', folder: f, open: false },
    { kind: 'space', spaceId: 'b', depth: 0 },
  ]);
  assert.deepEqual(switcherRows(['a', f], new Set(['f']), '', nameOf), [
    { kind: 'space', spaceId: 'a', depth: 0 },
    { kind: 'folder', folder: f, open: true },
    { kind: 'space', spaceId: 'x', depth: 1, folder: f },
    { kind: 'space', spaceId: 'y', depth: 1, folder: f },
  ]);
  // Accent- and case-insensitive; closed folders are searched too.
  assert.deepEqual(switcherRows(['a', f, 'b'], new Set(), ' CAFE', nameOf), [
    { kind: 'space', spaceId: 'x', depth: 0 },
  ]);
  assert.deepEqual(switcherRows(['a', f, 'b'], new Set(), 'zzz', nameOf), []);
});

test('folder labels, space counts and the filter threshold', () => {
  const nameOf = (id: string) => ({ x: 'Xeno', y: 'Yarn' })[id] ?? '';
  assert.equal(folderLabel(folder('f', ['x', 'y']), nameOf), 'Xeno, Yarn');
  assert.equal(folderLabel(folder('f', ['x'], 'Work'), nameOf), 'Work');
  assert.equal(folderLabel(folder('f', ['q']), nameOf), 'Folder');
  assert.equal(spaceCount(['a', folder('f', ['x', 'y']), 'b']), 4);
  assert.equal(SWITCHER_FILTER_THRESHOLD, 8);
});

test('the chevron dot: unread in a space other than the open one', () => {
  const unread = new Map([
    ['a', { total: 3, highlight: 0 }],
    ['x', { total: 1, highlight: 2 }],
  ]);
  const items: SidebarItems = ['a', folder('f', ['x', 'y']), 'b'];
  assert.deepEqual(otherSpacesUnread(items, undefined, unread), { unread: true, attention: 2 });
  assert.deepEqual(otherSpacesUnread(items, 'x', unread), { unread: true, attention: 0 });
  assert.deepEqual(otherSpacesUnread(['a'], 'a', unread), { unread: false, attention: 0 });
  assert.deepEqual(otherSpacesUnread(['b'], undefined, unread), { unread: false, attention: 0 });
});

test('layout writes revert at once and show the failure (the rail rule)', () => {
  const source = readFileSync('src/app/features/nav-pane/useSpaceOrder.ts', 'utf8');
  assert.match(source, /localEchoItems\(previous\);\s*setSaveError\(failure\);/);
  assert.match(source, /if \(sequence !== saveSequence\.current\) return;/);
  assert.doesNotMatch(source, /void saveSynaraSpacesContent/);
  const switcher = readFileSync('src/app/features/nav-pane/SpaceSwitcher.tsx', 'utf8');
  assert.match(switcher, /role="alert"/);
  assert.match(switcher, /SWITCHER_FILTER_THRESHOLD/);
  // Keyboard: rows rove with the arrow keys and open options with Shift+F10.
  assert.match(switcher, /listNavigationIndex\(evt\.key/);
  assert.match(switcher, /evt\.key === 'F10' && evt\.shiftKey/);
});
