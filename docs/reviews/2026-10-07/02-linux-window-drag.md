# Linux window drag

Branch: `feature/2026-10-07-client-session-and-chrome`.
Baseline: HEAD `6f964d3f`, tag `v2.1.46`.
Workstation: Pop!_OS, kernel `7.1.5-76070105-generic`, COSMIC on Wayland (`XDG_SESSION_TYPE=wayland`, `XDG_CURRENT_DESKTOP=COSMIC`, `WAYLAND_DISPLAY=wayland-1`). Installed client is Synara 2.1.46.
Scope: moving the borderless Linux window from the in-app title strip, and keeping that window undecorated across relaunch. Session restore, the connection banner, and unsent local echo are other tasks.

Resolved crates on this branch: Tauri `2.12.1` (`@tauri-apps/api` `2.12.1`), `tauri-plugin-window-state` `2.5.0`, `tao` `0.37.1`, `wry` `0.57.0`.

## Problem statement

On the Linux desktop client the user can stretch the window from its edges and cannot drag it to a new position. macOS and Windows are supposed to keep their native titlebars. Linux is supposed to stay borderless and move from the in-app strip.

The running client (process start 2026-10-07 08:42:13) restored `~/.config/com.whylandcreative.synara.desktop/.window-state.json` (mtime 2026-10-07 08:42:12). The plugin writes that file on `RunEvent::Exit`, so this copy is the snapshot the process restored, not a live rewrite at startup. The `main` record is 1648×928, `x`/`y`/`prev_x`/`prev_y` all 0, `maximized: false`, `decorated: true`, `fullscreen: false`, `visible: true`. Size has changed from the builder's 1280×900 logical default. Position has never left the origin. There are no window-drag log lines. No title-bar commit exists in the last seven days. The last title-bar change is `9e69a7d8` (2026-09-06), which observes native maximize state.

## Root cause

Two mechanisms are both in the code. They fail in different ways. Movement fails because the drag request is asynchronous. Decorations fail because window-state puts the native decorated flag back.

### Movement: the drag request runs after the button-press serial is gone

`DesktopTitleBar` renders only when `isSynaraDesktop() && isLinuxOS()`. The bar and the inner grow box set a bare `data-tauri-drag-region`. The strip is 40px (`toRem(40)` in `DesktopTitleBar.css.ts`). Minimize, maximize, and close are `IconButton`s and call `desktop_window_minimize`, `desktop_window_toggle_maximize`, and `desktop_window_close`. Application source has no `startDragging` and no `getCurrentWindow().startDragging()`. `core:window:allow-start-dragging` is already granted in `src-tauri/capabilities/main.json`. The missing call in the repo is not a missing permission.

Tauri 2.12.1 injects `src/window/scripts/drag.js`. On Linux, a primary `mousedown` whose event target is a bare drag-region element calls `window.__TAURI_INTERNALS__.invoke('plugin:window|start_dragging')` when `detail === 1`. A bare attribute matches only `composedPath[0]`, so the empty inner grow box is the hit target and the buttons are not: `BUTTON` with no drag attribute stops the walk. That invoke is the same async command as `getCurrentWindow().startDragging()`.

`tao` 0.37.1 `Window::drag_window` enqueues `WindowRequest::DragWindow`. The GTK loop later calls `gtk_window_begin_move_drag` with button 1 and timestamp 0 (`GDK_CURRENT_TIME`). On WebKitGTK/Wayland, `begin_move_drag` has to run inside the button-press handler, using that event's serial. The IPC round trip plus the queued request both return from the button-press handler first, and the compositor ignores the move. `wry` 0.57.0's WebKit button-press hook only synthesizes back/forward (buttons 8 and 9). It does not start a window move.

Edge resize still works because the compositor resizes the frame. It does not go through `start_resize_dragging`. The saved geometry is the record of that split: width and height changed, x and y stayed 0.

A React `onMouseDown` that calls `getCurrentWindow().startDragging()` uses that same `plugin:window|start_dragging` invoke. It does not satisfy the move requirement on this COSMIC Wayland session.

### Decorations: default window-state flags call `set_decorations(true)`

`src-tauri/src/lib.rs` applies `.decorations(false)` only under `#[cfg(target_os = "linux")]`. The comment says Linux uses the in-app strip. The same setup registers `tauri_plugin_window_state::Builder::default()` with no `with_state_flags` and no `skip_initial_state`.

In window-state 2.5.0, `StateFlags::default()` is `StateFlags::all()`, which includes `DECORATIONS` (`1 << 4`). `Builder::default()` keeps those flags. On window ready, `restore_state` loads `.window-state.json`. The saved `main` record is not `WindowState::default()` (default size is 0×0; this file is 1648×928), so the restore branch runs and, because `DECORATIONS` is set, calls `set_decorations(state.decorated)`. The file says `decorated: true`, so that call is `set_decorations(true)`. It runs after the Linux builder's `decorations(false)`.

`tao`'s `set_decorations` only enqueues `WindowRequest::Decorations`. A read of `is_decorated()` on the next line of `setup` can still be false. The GTK loop then applies `gtk_window_set_decorated(true)`. On exit, `update_state` copies `is_decorated()` back into the file, which is why the snapshot stays `decorated: true`. No other product code sets decorations on.

On Wayland, tao's window constructor does this when decorations start false: it calls `set_decorated(false)`, then `set_titlebar` with an empty `EventBox` so the compositor does not draw server-side decorations. `WindowRequest::Decorations(true)` only calls `set_decorated(true)`. It does not install a header bar with a move handler. Showing that empty `EventBox` again is not a draggable native titlebar. The restored flag fights `decorations(false)` and still leaves the window unmovable.

### How to prove it

Decorations, on a launch that still has this JSON, after the window is shown and the GTK loop has processed the queued request: `is_decorated()` is true. The builder had set it false. The exit snapshot already records that outcome. A read inline at the end of `WebviewWindowBuilder::build()` is too early to disprove the restore.

Movement, on this workstation, with the current build: a primary-button drag on the inner grow box does not change `outer_position`. An edge resize does change size. A log line that `start_dragging` was invoked shows the command ran. It does not show that the compositor accepted a move. The position change is the proof.

After the fix, repeat both on this same session. `is_decorated()` stays false across relaunch while the existing file still contains `"decorated": true`. A primary drag on the strip changes x/y and leaves size unchanged.

## Requirements

### FR-DRAG-1 — Primary drag on the Linux title strip moves the window

On the Linux desktop client, a primary-button press on the title strip's drag surface, followed by pointer movement and release, moves the window. Size stays constant for that gesture. The drag surface is the current inner grow box (bare `data-tauri-drag-region`) and any padding of the strip whose event target is the strip itself. The minimize, maximize, and close controls are not drag surfaces.

The move starts inside the GTK button-press handler for that press, with that event's button and timestamp, so the Wayland serial is still valid. `plugin:window|start_dragging`, including a React call to `getCurrentWindow().startDragging()`, and `tao`'s queued `WindowRequest::DragWindow`, do not meet this requirement on COSMIC Wayland.

### FR-DRAG-2 — Window controls keep their current jobs

The three controls still run the existing commands:

- Minimize calls `desktop_window_minimize`.
- Maximize / Restore calls `desktop_window_toggle_maximize`.
- Close calls `desktop_window_close`, which hides to the tray. It does not call `Window::close()`.

Their accessible names stay Minimize, Maximize or Restore, and Close. A press on a control activates the control and does not start a move.

### FR-DRAG-3 — Edge resize still works

Dragging a window edge or corner still resizes the window, down to the existing minimum inner size (960×720). Resize stays on the compositor frame path. This requirement does not add `startResizeDragging` handles in the page.

### FR-DRAG-4 — Relaunch leaves Linux undecorated

On Linux, window-state must not restore `DECORATIONS`. `Builder::default()`'s `StateFlags::all()` includes that flag; the Linux registration has to drop it and keep size, position, maximized, visible, and fullscreen.

A relaunch with the current `.window-state.json` (`decorated: true`, non-zero size) does not call `set_decorations(true)`. After the window is shown, `is_decorated()` is false, and there is no OS titlebar and no empty client-side titlebar from tao's Wayland `EventBox`. The user does not have to delete the state file. Size and position from a later successful move still restore.

macOS and Windows keep `tauri_plugin_window_state::Builder::default()` and the current window builder. `decorations(false)` stays inside `#[cfg(target_os = "linux")]`.

### FR-DRAG-5 — Double-click maximize does not regress

Double-click on the drag surface still toggles maximize / restore. Today Tauri's `drag.js` does that on Linux with `plugin:window|internal_toggle_maximize` when `detail === 2`. `DesktopTitleBar` has no `onDoubleClick`; the existing test locks that. Keep a single owner. If the synchronous press handler stops the event before `drag.js` sees the second click, that handler performs the same toggle.

The in-app Maximize / Restore icon and label still follow `observeNativeMaximizedState` after that toggle and after a window-manager maximize.

### FR-DRAG-6 — The strip stays Linux-only

`useDesktopTitleBarVisible` stays `isSynaraDesktop() && isLinuxOS()`. macOS and Windows do not render `DesktopTitleBar`. Room and home headers that already carry `data-tauri-drag-region` keep that attribute; the existing title-bar source test requires it. Making those headers move the window is not part of this fix.

## Acceptance criteria

A reviewer checks the manual items on this Pop!_OS COSMIC Wayland workstation. Source and unit tests cover the rest. There is no headless drag test in this environment.

1. Primary-button drag on the empty grow region of the Linux title strip moves the window. Width and height stay the same across that gesture. `outer_position` (or the next saved `x`/`y`) differs from the pre-drag position.
2. Minimize, Maximize/Restore, and Close still work. Close hides to the tray and the window can be shown again. Pressing those buttons does not move the window.
3. Dragging an edge or corner still resizes the window and still respects the 960×720 minimum.
4. Quit and launch again without deleting `.window-state.json`. The window has the in-app strip only. `is_decorated()` is false after show. The file may still contain `"decorated": true`; that field is not applied. A position saved from criterion 1 is restored.
5. The non-Linux `WebviewWindowBuilder` chain is unchanged: no `decorations(false)`, no `TitleBarStyle::Overlay`, no `hidden_title(true)` outside the existing Linux cfg. `DesktopTitleBar` still returns null unless the runtime is the Linux desktop app.
6. Double-click on the drag surface toggles maximized and restored. The in-app button name and icon switch between Maximize and Restore. `DesktopTitleBar.tsx` still has no `onDoubleClick`.

## Testing requirements

Unit-test any pure helper the implementer adds. Do not add a test that claims to drag a Wayland window from CI or from this agent's shell.

- If Linux window-state flags are a function, a Rust unit test asserts `DECORATIONS` is clear and size, position, maximized, visible, and fullscreen remain set. Place it next to that function.
- If a helper decides that a press starts a move (primary button, drag surface, not a window control), test that helper with those inputs. A button target does not start a move. A double-click classification, if the helper owns it, toggles maximize and does not start a second move owner.
- Extend `synara/src/app/features/desktop-titlebar/__tests__/desktopTitleBar.test.ts` only for the contract that remains: Linux-only strip, the three commands, no `onDoubleClick`, and `core:window:allow-start-dragging` still granted. A source match on `startDragging` is not evidence the window moved.

Manual check on this workstation, with a physical pointer, against the acceptance list above. Record the before and after position. The current failure is the baseline: edges resize, the strip does not move, saved `x` and `y` stay 0.

## Allowed paths

- `src-tauri/src/lib.rs` — Linux window-state flags via `with_state_flags`, and a call from `setup` into the drag hook. Leave the macOS menu block and the shared builder chain alone. `decorations(false)` stays Linux-only.
- A new focused module under `src-tauri/src/` (for example `desktop_window_drag.rs`) — the Linux button-press move, using the event serial. `desktop_spellcheck.rs` and `desktop_webview_performance.rs` already use `with_webview`; follow that hook style. Do not put the drag handler inside spellcheck or performance.
- `src-tauri/src/desktop.rs` — only if the hook needs a small helper beside the existing window commands. `desktop_window_close` still hides to the tray.
- `synara/src/app/features/desktop-titlebar/DesktopTitleBar.tsx` — only to keep the drag surface and the three controls distinguishable. Do not add `onDoubleClick`.
- `synara/src/app/features/desktop-titlebar/DesktopTitleBar.css.ts` — only to keep the drag surface a stable hit target. Height stays `toRem(40)`.
- `synara/src/app/features/desktop-titlebar/__tests__/desktopTitleBar.test.ts` and a Rust unit test next to a new flags or hit-test helper.
- `src-tauri/capabilities/main.json`, `src-tauri/build.rs`, `src-tauri/permissions/autogenerated/`, and `src-tauri/gen/schemas/` — only if a new command is unavoidable. A new async command that calls `window.start_dragging()` does not meet FR-DRAG-1. Prefer no new IPC. Leave `core:window:allow-start-dragging` in place.

## Non-goals

- Do not redesign the title strip, window controls, or the rest of the chrome.
- Do not change session restore, sync, the connection banner, mark-read, or unsent local echo.
- Do not change the macOS or Windows titlebar, and do not give those platforms the Linux strip.
- Do not remove `data-tauri-drag-region` from room, home, or drawer headers, and do not take on making those headers drag.
- Do not upgrade Tauri, tao, wry, or window-state as the fix.
- Do not delete or hand-edit `.window-state.json` as the fix.
- Do not add a headless or synthetic-pointer drag test. This environment cannot begin a COSMIC Wayland move from a unit test.
