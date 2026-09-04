import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const source = (path: string) => readFileSync(path, 'utf8');

test('linux titlebar renders only on the linux desktop shell', () => {
  const titlebar = source('src/app/features/desktop-titlebar/DesktopTitleBar.tsx');

  assert.match(titlebar, /isSynaraDesktop\(\) && isLinuxOS\(\)/);
  assert.match(titlebar, /if \(!visible\) return null/);
  assert.match(titlebar, /data-tauri-drag-region/);
  assert.match(titlebar, /onDoubleClick=\{toggleMaximize\}/);
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

  // macOS overlays traffic lights; Linux drops server decorations.
  assert.match(lib, /#\[cfg\(target_os = "macos"\)\]/);
  assert.match(lib, /title_bar_style\(tauri::TitleBarStyle::Overlay\)/);
  assert.match(lib, /\.hidden_title\(true\)/);
  assert.match(lib, /#\[cfg\(target_os = "linux"\)\]/);
  assert.match(lib, /\.decorations\(false\)/);
  // Window-control commands are registered and ACL-granted.
  assert.match(lib, /desktop::desktop_window_minimize/);
  assert.match(lib, /desktop::desktop_window_toggle_maximize/);
  assert.match(lib, /desktop::desktop_window_close/);
  assert.match(lib, /matrix::auth::product::matrix_timeline_follow_live/);
  for (const permission of [
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
});

test('macos overlay keeps headers draggable and traffic lights clear', () => {
  const sidebar = source('src/app/pages/client/SidebarNav.tsx');
  const home = source('src/app/pages/client/home/Home.tsx');
  const header = source('src/app/features/room/RoomViewHeader.tsx');
  const sidePanel = source('src/app/features/room/RoomSidePanel.tsx');
  const members = source('src/app/features/room/MembersDrawer.tsx');

  assert.match(sidebar, /isSynaraDesktop\(\) && isMacOS\(\)/);
  assert.match(sidebar, /data-tauri-drag-region/);
  for (const [name, component] of Object.entries({ home, header, sidePanel, members })) {
    assert.match(component, /data-tauri-drag-region/, `${name} header must drag the window`);
  }
});
