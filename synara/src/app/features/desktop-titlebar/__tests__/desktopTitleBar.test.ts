import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { observeNativeMaximizedState } from '../nativeMaximizedState';

const source = (path: string) => readFileSync(path, 'utf8');
const settle = () => new Promise<void>((resolve) => setImmediate(resolve));

test('native maximize and restore update titlebar state without a custom button command', async () => {
  let maximized = true;
  let resize: (() => void) | undefined;
  let removed = 0;
  const states: boolean[] = [];
  const observer = observeNativeMaximizedState(
    {
      isMaximized: async () => maximized,
      onResized: async (handler) => {
        resize = handler;
        return () => {
          removed += 1;
          resize = undefined;
        };
      },
    },
    (state) => states.push(state)
  );
  await settle();
  assert.deepEqual(states, [true], 'initially maximized windows must show Restore');
  maximized = false;
  resize?.();
  await settle();
  maximized = true;
  resize?.();
  await settle();
  assert.deepEqual(states, [true, false, true]);
  observer.dispose();
  assert.equal(removed, 1);
  assert.equal(resize, undefined);
});

test('late native readbacks cannot overwrite a newer resize or a disposed titlebar', async () => {
  const pending: ((value: boolean) => void)[] = [];
  let resize: (() => void) | undefined;
  const states: boolean[] = [];
  const observer = observeNativeMaximizedState(
    {
      isMaximized: () => new Promise<boolean>((resolve) => pending.push(resolve)),
      onResized: async (handler) => {
        resize = handler;
        return () => {
          resize = undefined;
        };
      },
    },
    (state) => states.push(state)
  );
  await settle();
  resize?.();
  pending[1](true);
  await settle();
  pending[0](false);
  await settle();
  assert.deepEqual(states, [true]);
  resize?.();
  observer.dispose();
  pending[2](false);
  await settle();
  await observer.refresh();
  assert.deepEqual(states, [true]);
  assert.equal(pending.length, 3, 'disposed observations must not issue more queries');
});

test('unmount during native listener registration removes the late subscription', async () => {
  let finishRegistration: ((stop: () => void) => void) | undefined;
  let removed = 0;
  let reads = 0;
  const observer = observeNativeMaximizedState(
    {
      isMaximized: async () => {
        reads += 1;
        return true;
      },
      onResized: () =>
        new Promise((resolve) => {
          finishRegistration = resolve;
        }),
    },
    () => assert.fail('a disposed titlebar must not receive a result')
  );
  observer.dispose();
  finishRegistration?.(() => {
    removed += 1;
  });
  await settle();
  assert.equal(removed, 1);
  assert.equal(reads, 0);
});

test('failed native queries preserve confirmation and a later resize can refresh', async () => {
  let fail = false;
  let resize: (() => void) | undefined;
  const states: boolean[] = [];
  const observer = observeNativeMaximizedState(
    {
      isMaximized: async () => {
        if (fail) throw new Error('native window unavailable');
        return true;
      },
      onResized: async (handler) => {
        resize = handler;
        return () => undefined;
      },
    },
    (state) => states.push(state)
  );
  await settle();
  fail = true;
  resize?.();
  await settle();
  assert.deepEqual(states, [true]);
  fail = false;
  resize?.();
  await settle();
  assert.deepEqual(states, [true, true]);
  observer.dispose();
});

test('custom titlebar renders only on Linux desktop', () => {
  const titlebar = source('src/app/features/desktop-titlebar/DesktopTitleBar.tsx');

  assert.match(titlebar, /isSynaraDesktop\(\) && isLinuxOS\(\)/);
  assert.match(titlebar, /if \(!visible\) return null/);
  assert.match(titlebar, /data-tauri-drag-region/);
  assert.doesNotMatch(titlebar, /onDoubleClick/);
});

test('linux titlebar owns drag and the three window controls', () => {
  const titlebar = source('src/app/features/desktop-titlebar/DesktopTitleBar.tsx');

  assert.match(titlebar, /desktop_window_minimize/);
  assert.match(titlebar, /desktop_window_toggle_maximize/);
  assert.match(titlebar, /desktop_window_close/);
  assert.match(titlebar, /aria-label="Minimize"/);
  assert.match(titlebar, /aria-label="Close"/);
  assert.match(titlebar, /className=\{depthCss\.quietInteractiveSurface\}/);
});

test('native window chrome matches the in-app titlebar contract', () => {
  const lib = source('../src-tauri/src/lib.rs');
  const capabilities = JSON.parse(source('../src-tauri/capabilities/main.json')) as {
    permissions: string[];
  };

  // macOS keeps native chrome; Linux drops server decorations.
  assert.doesNotMatch(lib, /title_bar_style\(tauri::TitleBarStyle::Overlay\)/);
  assert.doesNotMatch(lib, /\.hidden_title\(true\)/);
  assert.match(lib, /#\[cfg\(target_os = "linux"\)\]/);
  assert.match(lib, /\.decorations\(false\)/);
  // Window-control commands are registered and ACL-granted.
  assert.match(lib, /desktop::desktop_window_minimize/);
  assert.match(lib, /desktop::desktop_window_toggle_maximize/);
  assert.match(lib, /desktop::desktop_window_close/);
  assert.match(lib, /matrix::auth::product::matrix_timeline_follow_live/);
  const build = source('../src-tauri/build.rs');
  assert.match(build, /"desktop_window_minimize"/);
  assert.match(build, /"desktop_window_toggle_maximize"/);
  assert.match(build, /"desktop_window_close"/);
  assert.match(build, /"matrix_timeline_follow_live"/);
  const desktop = source('../src-tauri/src/desktop.rs');
  assert.match(desktop, /pub fn desktop_window_close/);
  assert.match(desktop, /hide_main_window/);
  assert.doesNotMatch(
    desktop.split('pub fn desktop_window_close')[1]?.split('pub fn desktop_navigate')[0] ?? '',
    /window\.close\(\)/
  );
  for (const permission of [
    'core:window:allow-start-dragging',
    'allow-desktop-window-minimize',
    'allow-desktop-window-toggle-maximize',
    'allow-desktop-window-close',
    'allow-matrix-timeline-follow-live',
  ]) {
    assert.ok(
      capabilities.permissions.includes(permission),
      `${permission} must be granted to the main webview`
    );
  }
  const linuxSchema = source('../src-tauri/gen/schemas/linux-schema.json');
  for (const command of [
    'desktop_window_minimize',
    'desktop_window_toggle_maximize',
    'desktop_window_close',
    'matrix_timeline_follow_live',
  ]) {
    assert.match(linuxSchema, new RegExp(`allow-${command.replaceAll('_', '-')}`));
  }
});

test('native macOS chrome needs no sidebar spacer and headers retain optional drag', () => {
  const sidebar = source('src/app/pages/client/SidebarNav.tsx');
  const home = source('src/app/pages/client/home/Home.tsx');
  const header = source('src/app/features/room/RoomViewHeader.tsx');
  const sidePanel = source('src/app/features/room/RoomSidePanel.tsx');
  const members = source('src/app/features/room/MembersDrawer.tsx');

  assert.doesNotMatch(sidebar, /overlaySpacer/);
  for (const [name, component] of Object.entries({ home, header, sidePanel, members })) {
    assert.match(component, /data-tauri-drag-region/, `${name} header must drag the window`);
  }
});
