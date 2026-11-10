import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  APP_SHORTCUT_ACTIONS,
  APP_SHORTCUT_SCOPES,
  APP_SHORTCUTS,
  appShortcut,
  appShortcutAction,
  appShortcutHelp,
  appShortcutHint,
  bindingAriaKeyShortcuts,
  bindingLabel,
  menuAccelerator,
  nextPageZoom,
  PAGE_ZOOM_STEPS,
  shortcutBinding,
  type AppShortcutPlatform,
  type KeyBinding,
} from '../appShortcuts';
import { cycleFocusRegion } from '../focusRegions';

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
const platforms: [string, AppShortcutPlatform][] = [
  ['macOS', mac],
  ['Linux', linux],
  ['Windows', windows],
];

/** The key event a binding describes. */
const pressOf = (binding: KeyBinding, keyValue = binding.keys[0]) =>
  key(keyValue, {
    ctrlKey: !!binding.ctrl,
    metaKey: !!binding.meta,
    altKey: !!binding.alt,
    shiftKey: binding.shift === true,
  });

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
  // Shift adds "unread" (Slack's ⌥⇧↑/↓).
  assert.equal(
    appShortcutAction(key('ArrowDown', { altKey: true, shiftKey: true }), mac),
    'next-unread-room'
  );
  assert.equal(
    appShortcutAction(key('ArrowUp', { altKey: true, shiftKey: true }), windows),
    'previous-unread-room'
  );
  assert.equal(
    appShortcutAction(key('ArrowDown', { altKey: true, ctrlKey: true }), linux),
    undefined
  );
  // A shifted settings or zoom-out key is not the shortcut.
  assert.equal(appShortcutAction(key(',', { metaKey: true, shiftKey: true }), mac), undefined);
  assert.equal(appShortcutAction(key('-', { ctrlKey: true, shiftKey: true }), linux), undefined);
});

test('Ctrl+Q quits only on Linux, and composition never triggers a shortcut', () => {
  assert.equal(appShortcutAction(key('q', { ctrlKey: true }), linux), 'quit');
  assert.equal(appShortcutAction(key('Q', { ctrlKey: true }), linux), 'quit');
  assert.equal(appShortcutAction(key('q', { ctrlKey: true }), windows), undefined);
  assert.equal(appShortcutAction(key('q', { metaKey: true }), mac), undefined);
  assert.equal(appShortcutAction(key('q', { ctrlKey: true, shiftKey: true }), linux), undefined);
  assert.equal(
    appShortcutAction({ ...key('=', { ctrlKey: true }), isComposing: true }, linux),
    undefined
  );
});

test('Cmd/Ctrl+Shift+D toggles the room list, and nothing else takes it', () => {
  assert.equal(
    appShortcutAction(key('d', { metaKey: true, shiftKey: true }), mac),
    'toggle-room-list'
  );
  // Chromium reports the shifted letter.
  assert.equal(
    appShortcutAction(key('D', { metaKey: true, shiftKey: true }), mac),
    'toggle-room-list'
  );
  assert.equal(
    appShortcutAction(key('D', { ctrlKey: true, shiftKey: true }), linux),
    'toggle-room-list'
  );
  assert.equal(
    appShortcutAction(key('D', { ctrlKey: true, shiftKey: true }), windows),
    'toggle-room-list'
  );
  // Without Shift, with Alt, or with Ctrl on macOS (Slate's delete-forward is Ctrl+D).
  assert.equal(appShortcutAction(key('d', { metaKey: true }), mac), undefined);
  assert.equal(appShortcutAction(key('d', { ctrlKey: true }), linux), undefined);
  assert.equal(
    appShortcutAction(key('D', { metaKey: true, shiftKey: true, altKey: true }), mac),
    undefined
  );
  assert.equal(appShortcutAction(key('D', { ctrlKey: true, shiftKey: true }), mac), undefined);
  assert.equal(
    appShortcutAction({ ...key('D', { metaKey: true, shiftKey: true }), isComposing: true }, mac),
    undefined
  );
});

test('search, help, focus cycling and Escape come from the registry', () => {
  assert.equal(appShortcutAction(key('k', { metaKey: true }), mac), 'search');
  assert.equal(appShortcutAction(key('K', { ctrlKey: true }), linux), 'search');
  assert.equal(appShortcutAction(key('k', { ctrlKey: true }), mac), undefined);
  assert.equal(appShortcutAction(key('k', { ctrlKey: true, shiftKey: true }), windows), undefined);

  assert.equal(appShortcutAction(key('/', { metaKey: true }), mac), 'shortcut-help');
  assert.equal(appShortcutAction(key('/', { ctrlKey: true }), windows), 'shortcut-help');
  // Layouts that type `/` with Shift still reach it; `?` (US Shift+/) does not.
  assert.equal(
    appShortcutAction(key('/', { ctrlKey: true, shiftKey: true }), linux),
    'shortcut-help'
  );
  assert.equal(appShortcutAction(key('?', { metaKey: true, shiftKey: true }), mac), undefined);

  assert.equal(appShortcutAction(key('F6'), mac), 'focus-next-region');
  assert.equal(appShortcutAction(key('F6', { shiftKey: true }), linux), 'focus-previous-region');
  assert.equal(appShortcutAction(key('F6', { ctrlKey: true }), windows), undefined);

  assert.equal(appShortcutAction(key('Escape'), linux), 'close-thread');
  assert.equal(appShortcutAction(key('Escape', { metaKey: true }), mac), undefined);
});

test('the registry is one table: unique ids, every binding reaches its own action', () => {
  assert.equal(new Set(APP_SHORTCUT_ACTIONS).size, APP_SHORTCUTS.length);
  const scopes = new Set(APP_SHORTCUT_SCOPES.map(({ scope }) => scope));
  for (const [name, platform] of platforms) {
    for (const shortcut of APP_SHORTCUTS) {
      assert.ok(scopes.has(shortcut.scope), shortcut.id);
      const binding = shortcutBinding(shortcut, platform);
      if (!binding) continue;
      for (const keyValue of binding.keys) {
        // Two entries with the same binding would shadow one another.
        assert.equal(
          appShortcutAction(pressOf(binding, keyValue), platform),
          shortcut.id,
          `${name} ${shortcut.id} ${keyValue}`
        );
      }
      if (name !== 'macOS') {
        // No Super-key bindings on Windows or Linux.
        assert.ok(!binding.meta, `${shortcut.id} uses Super on ${name}`);
      }
    }
  }
  // Every shortcut exists on macOS and Windows/Linux except Linux's Quit.
  for (const shortcut of APP_SHORTCUTS) {
    if (shortcut.id === 'quit') continue;
    assert.ok(shortcut.mac && shortcut.windowsLinux, shortcut.id);
  }
  assert.equal(shortcutBinding(appShortcut('quit'), windows), undefined);
  assert.throws(() => appShortcut('nope' as never));
});

test('no app shortcut takes a composer formatting key or the type-to-focus paste', () => {
  const editorKeys = readFileSync('src/app/components/editor/keyboard.ts', 'utf8');
  const roomInput = readFileSync('src/app/features/room/RoomInput.tsx', 'utf8');
  const roomView = readFileSync('src/app/features/room/RoomView.tsx', 'utf8');
  const hotkeys = new Set(
    [editorKeys, roomInput, roomView].flatMap((text) =>
      Array.from(text.matchAll(/["'](mod\+[^"']+)["']/g), (match) => match[1])
    )
  );
  // Bold, italic, headings 1–3, lists, quote, code block, exit formatting, send, paste.
  for (const expected of ['mod+b', 'mod+1', 'mod+3', 'mod+7', 'mod+e', 'mod+enter', 'mod+v']) {
    assert.ok(hotkeys.has(expected), expected);
  }
  for (const hotkey of hotkeys) {
    const parts = hotkey.split('+');
    const keyName = parts.at(-1)!;
    const value = keyName === 'enter' ? 'Enter' : keyName;
    const shiftKey = parts.includes('shift');
    const altKey = parts.includes('alt');
    for (const [name, platform] of platforms) {
      const evt = key(value, {
        metaKey: platform.macOS,
        ctrlKey: !platform.macOS,
        shiftKey,
        altKey,
      });
      assert.equal(appShortcutAction(evt, platform), undefined, `${hotkey} on ${name}`);
    }
  }
});

test('bindings are written ⌘⇧D on macOS and Ctrl+Shift+D elsewhere', () => {
  const label = (id: Parameters<typeof appShortcut>[0], platform: AppShortcutPlatform) =>
    bindingLabel(shortcutBinding(appShortcut(id), platform)!, platform.macOS);
  assert.equal(label('toggle-room-list', mac), '⌘⇧D');
  assert.equal(label('toggle-room-list', linux), 'Ctrl+Shift+D');
  assert.equal(label('next-unread-room', mac), '⌥⇧↓');
  assert.equal(label('previous-unread-room', windows), 'Alt+Shift+↑');
  assert.equal(label('search', mac), '⌘K');
  assert.equal(label('shortcut-help', windows), 'Ctrl+/');
  assert.equal(label('zoom-in', mac), '⌘=');
  assert.equal(label('focus-previous-region', linux), 'Shift+F6');
  assert.equal(label('close-thread', mac), 'Esc');
  assert.equal(
    bindingAriaKeyShortcuts(appShortcut('next-unread-room').mac!),
    'Alt+Shift+ArrowDown'
  );
  assert.equal(bindingAriaKeyShortcuts(appShortcut('search').windowsLinux!), 'Control+K');
});

test('the help sheet lists every bound shortcut once, by group, in sentence case', () => {
  for (const [name, platform] of platforms) {
    const groups = appShortcutHelp(platform);
    assert.deepEqual(
      groups.map((group) => group.label),
      ['General', 'Navigation', 'Rooms and threads', 'View'],
      name
    );
    const ids = groups.flatMap((group) => group.items.map((item) => item.id));
    assert.equal(new Set(ids).size, ids.length);
    assert.equal(ids.includes('quit'), platform.linux, name);
    assert.ok(ids.includes('shortcut-help'));
    // One row per action: no label twice, and no binding dropped.
    const items = groups.flatMap((group) => group.items);
    const labels = items.map((item) => item.label);
    assert.equal(new Set(labels).size, labels.length, name);
    assert.equal(
      items.flatMap((item) => item.keys).length,
      APP_SHORTCUTS.filter((shortcut) => shortcutBinding(shortcut, platform)).length,
      name
    );
    for (const text of [
      ...groups.map((group) => group.label),
      ...groups.flatMap((group) => group.items.map((item) => item.label)),
    ]) {
      // Capital first letter; later capitals only in the product name.
      assert.match(text.replace('Synara', 'synara'), /^[A-Z][^A-Z]*$/, text);
    }
  }
  const navigation = appShortcutHelp(mac).find((group) => group.scope === 'navigation')!;
  assert.deepEqual(
    navigation.items.map((item) => [item.label, item.keys]),
    [
      ['Open home', ['⌃1']],
      ['Open activity', ['⌃2']],
      ['Open approvals', ['⌃3']],
      ['Open space switcher', ['⌘⇧S']],
      ['Previous room', ['⌥↑']],
      ['Next room', ['⌥↓']],
      ['Previous unread room', ['⌥⇧↑']],
      ['Next unread room', ['⌥⇧↓']],
      ['Move focus to the next area', ['F6']],
      ['Move focus to the previous area', ['⇧F6']],
    ]
  );
  // "Open settings" is one row, under General, with the place key beside ⌘,.
  for (const [platform, keys, aria] of [
    [mac, ['⌘,', '⌃4'], 'Meta+, Control+4'],
    [linux, ['Ctrl+,', 'Alt+4'], 'Control+, Alt+4'],
  ] as const) {
    const general = appShortcutHelp(platform).find((group) => group.scope === 'general')!;
    const settings = general.items.find((item) => item.id === 'settings')!;
    assert.deepEqual([settings.label, settings.keys], ['Open settings', keys]);
    assert.equal(settings.ariaKeyShortcuts, aria);
  }
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

test('F6 cycles list, timeline, composer and thread, skipping areas not on screen', () => {
  const all = ['room-list', 'timeline', 'composer', 'thread'] as const;
  assert.equal(cycleFocusRegion(all, 'room-list', 'next'), 'timeline');
  assert.equal(cycleFocusRegion(all, 'composer', 'next'), 'thread');
  assert.equal(cycleFocusRegion(all, 'thread', 'next'), 'room-list');
  assert.equal(cycleFocusRegion(all, 'room-list', 'previous'), 'thread');
  // No thread pane, and the list hidden.
  assert.equal(cycleFocusRegion(['timeline', 'composer'], 'composer', 'next'), 'timeline');
  assert.equal(cycleFocusRegion(['timeline', 'composer'], 'timeline', 'previous'), 'composer');
  // From outside every area.
  assert.equal(cycleFocusRegion(all, undefined, 'next'), 'room-list');
  assert.equal(cycleFocusRegion(all, undefined, 'previous'), 'thread');
  assert.equal(cycleFocusRegion(['room-list'], 'room-list', 'next'), 'room-list');
  assert.equal(cycleFocusRegion([], undefined, 'next'), undefined);
});

test('one dispatcher: the shortcuts are mounted for the signed-in app, no other window listener handles them, and the macOS menu matches the registry', () => {
  const features = readFileSync('src/app/pages/client/ClientNonUIFeatures.tsx', 'utf8');
  assert.ok(features.includes('<DesktopAppShortcuts />'));

  // ⌘K and Escape-closes-thread moved into the registry.
  const search = readFileSync('src/app/features/search/Search.tsx', 'utf8');
  assert.ok(!search.includes("'mod+k'"));
  assert.ok(!search.includes('useKeyDown'));
  const thread = readFileSync('src/app/features/room/ThreadSidePanel.tsx', 'utf8');
  assert.ok(!thread.includes("addEventListener('keydown'"));
  assert.ok(thread.includes("useAppShortcutHandler(\n    'close-thread'"));
  const dispatcher = readFileSync(
    'src/app/features/desktop-keyboard/DesktopAppShortcuts.tsx',
    'utf8'
  );
  assert.ok(!dispatcher.includes("data-room-nav-link]')"), 'room nav reads list data');
  assert.ok(dispatcher.includes('currentRoomListNavSource()'));

  const menu = readFileSync('../src-tauri/src/menu.rs', 'utf8');
  assert.ok(menu.includes('"synara://app-shortcut"'));
  const items = Array.from(
    menu.matchAll(/\(\s*"synara_[a-z_]+",\s*"[^"]*",\s*"([^"]+)",\s*"([a-z-]+)",?\s*\)/g),
    (match) => ({ accelerator: match[1], action: match[2] })
  );
  assert.deepEqual(
    items.map((item) => item.action).sort(),
    APP_SHORTCUTS.filter((shortcut) => shortcut.menu)
      .map((shortcut) => shortcut.id)
      .sort()
  );
  for (const item of items) {
    const shortcut = APP_SHORTCUTS.find((candidate) => candidate.id === item.action);
    assert.ok(shortcut, item.action);
    assert.equal(menuAccelerator(shortcut), item.accelerator, item.action);
  }

  // Quit stays on the Linux title strip, which asks the registry.
  const titleBar = readFileSync('src/app/features/desktop-titlebar/DesktopTitleBar.tsx', 'utf8');
  assert.ok(titleBar.includes("invokeDesktopWithAvailability('desktop_quit')"));
  assert.ok(titleBar.includes("appShortcutAction(evt, { macOS: false, linux: true }) !== 'quit'"));
  assert.ok(appShortcut('quit').external);
});

test('the space switcher is Cmd/Ctrl+Shift+S, apart from the composer strikethrough on Cmd/Ctrl+S', () => {
  assert.equal(
    appShortcutAction(key('S', { metaKey: true, shiftKey: true }), mac),
    'open-space-switcher'
  );
  for (const platform of [linux, windows]) {
    assert.equal(
      appShortcutAction(key('S', { ctrlKey: true, shiftKey: true }), platform),
      'open-space-switcher'
    );
    assert.equal(appShortcutAction(key('s', { ctrlKey: true }), platform), undefined);
  }
  assert.equal(appShortcutAction(key('s', { metaKey: true }), mac), undefined);
  // Handled by the list header that owns the switcher, through the dispatcher.
  const header = readFileSync('src/app/features/nav-pane/NavPaneHeader.tsx', 'utf8');
  assert.ok(header.includes("useAppShortcutHandler(\n    'open-space-switcher'"));
  assert.deepEqual(appShortcutHint('open-space-switcher', windows), {
    label: 'Ctrl+Shift+S',
    ariaKeyShortcuts: 'Control+Shift+S',
  });
});

test('the places are Control+1-4 on macOS and Alt+1-4 elsewhere, clear of the composer', () => {
  const places = ['go-home', 'go-activity', 'go-approvals', 'go-settings'] as const;
  places.forEach((id, index) => {
    const digit = String(index + 1);
    const shortcut = appShortcut(id);
    assert.deepEqual(shortcut.mac, { keys: [digit], ctrl: true });
    assert.deepEqual(shortcut.windowsLinux, { keys: [digit], alt: true });
  });
});
