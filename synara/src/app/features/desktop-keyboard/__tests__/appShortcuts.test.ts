import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  adjacentRoomLink,
  appShortcutAction,
  nextPageZoom,
  PAGE_ZOOM_STEPS,
} from '../appShortcuts';

const key = (
  value: string,
  modifiers: Partial<Record<'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey', boolean>> = {}
) => ({
  key: value,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  isComposing: false,
  ...modifiers,
});

const mac = { macOS: true, linux: false };
const linux = { macOS: false, linux: true };
const windows = { macOS: false, linux: false };

test('zoom, settings and room shortcuts use Cmd on macOS and Ctrl elsewhere', () => {
  assert.equal(appShortcutAction(key('=', { metaKey: true }), mac), 'zoom-in');
  assert.equal(appShortcutAction(key('+', { metaKey: true, shiftKey: true }), mac), 'zoom-in');
  assert.equal(appShortcutAction(key('-', { metaKey: true }), mac), 'zoom-out');
  assert.equal(appShortcutAction(key('0', { metaKey: true }), mac), 'zoom-reset');
  assert.equal(appShortcutAction(key(',', { metaKey: true }), mac), 'settings');
  // Ctrl on macOS is not the primary modifier.
  assert.equal(appShortcutAction(key('=', { ctrlKey: true }), mac), undefined);

  assert.equal(appShortcutAction(key('=', { ctrlKey: true }), linux), 'zoom-in');
  assert.equal(appShortcutAction(key(',', { ctrlKey: true }), windows), 'settings');
  assert.equal(appShortcutAction(key('ArrowUp', { altKey: true }), linux), 'previous-room');
  assert.equal(appShortcutAction(key('ArrowDown', { altKey: true }), mac), 'next-room');
  assert.equal(
    appShortcutAction(key('ArrowDown', { altKey: true, shiftKey: true }), mac),
    undefined
  );
});

test('Ctrl+Q quits only on Linux, and composition never triggers a shortcut', () => {
  assert.equal(appShortcutAction(key('q', { ctrlKey: true }), linux), 'quit');
  assert.equal(appShortcutAction(key('Q', { ctrlKey: true }), linux), 'quit');
  assert.equal(appShortcutAction(key('q', { ctrlKey: true }), windows), undefined);
  assert.equal(appShortcutAction(key('q', { ctrlKey: true, shiftKey: true }), linux), undefined);
  assert.equal(
    appShortcutAction({ ...key('=', { ctrlKey: true }), isComposing: true }, linux),
    undefined
  );
});

test('page zoom steps stay within the Appearance range', () => {
  assert.equal(nextPageZoom(100, 'zoom-in'), 110);
  assert.equal(nextPageZoom(100, 'zoom-out'), 90);
  assert.equal(nextPageZoom(115, 'zoom-in'), 125);
  assert.equal(nextPageZoom(115, 'zoom-out'), 110);
  assert.equal(nextPageZoom(200, 'zoom-in'), 200);
  assert.equal(nextPageZoom(75, 'zoom-out'), 75);
  assert.equal(nextPageZoom(150, 'zoom-reset'), 100);
  assert.equal(PAGE_ZOOM_STEPS[0], 75);
  assert.equal(PAGE_ZOOM_STEPS.at(-1), 200);
});

test('previous and next room follow sidebar order from the open room', () => {
  const link = (name: string, current = false) => ({
    name,
    getAttribute: (attribute: string) => (attribute === 'aria-current' && current ? 'page' : null),
  });
  const links = [link('a'), link('b', true), link('c')];
  assert.equal(adjacentRoomLink(links, 'next-room')?.name, 'c');
  assert.equal(adjacentRoomLink(links, 'previous-room')?.name, 'a');
  assert.equal(adjacentRoomLink([link('a', true)], 'previous-room'), undefined);
  assert.equal(adjacentRoomLink([link('a'), link('b')], 'next-room')?.name, 'a');
  assert.equal(adjacentRoomLink([link('a'), link('b')], 'previous-room')?.name, 'b');
  assert.equal(adjacentRoomLink([], 'next-room'), undefined);
});

test('the shortcuts are mounted for the signed-in app and the macOS menu sends the same actions', () => {
  const features = readFileSync('src/app/pages/client/ClientNonUIFeatures.tsx', 'utf8');
  assert.ok(features.includes('<DesktopAppShortcuts />'));
  const menu = readFileSync('../src-tauri/src/menu.rs', 'utf8');
  assert.ok(menu.includes('"synara://app-shortcut"'));
  for (const action of ['zoom-in', 'zoom-out', 'zoom-reset', 'settings']) {
    assert.ok(menu.includes(`"${action}"`), action);
  }
  const titleBar = readFileSync('src/app/features/desktop-titlebar/DesktopTitleBar.tsx', 'utf8');
  assert.ok(titleBar.includes("invokeDesktopWithAvailability('desktop_quit')"));
});
