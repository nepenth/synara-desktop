# Grok 4.6 High source review: window-depth

Read-only review; runtime evidence is separate. Session `01a0795e-0fde-7c53-b4d3-a6e134288581`.

I’ll finish the remaining source checks—drag-region hit targets, iOS collection callers, and Linux layout—then return the verdict.**ACCEPT**

Head `0a895fce3b565e9c141536aea23c91d68d0999db` matches the stated contract: native macOS chrome, Linux-only custom strip with the missing drag ACL, splash remaining-height layout, quiet iOS collection rows, and restrained Settings cards. No P1 defects in the changed source.

## Verdict

The original macOS failure mode is addressed in code by restoring the OS titlebar (Overlay/`hidden_title` removed in `src-tauri/src/lib.rs`) and deleting the 28px sidebar spacer. Linux stays borderless, keeps a persistent strip outside `ClientConfigLoader`, and gains `core:window:allow-start-dragging` — the permission Tauri 2.11.2 leaves out of `core:default` (`start_dragging` is default-deny; `internal_toggle_maximize` is default-allow). That is a minimal, main-webview-only grant.

Splash layout is structurally sound: `#root` is a column flex container; the strip is `flex-shrink: 0`; `SplashScreen` is `flex: 1 1 0; min-height: 0; overflow-y: auto`. That is the right fix for the prior 40px footer clip. iOS room rows use `SynaraSurfaceDepthRole.roomRow` → `.collection` (no drop shadow, faint edge; Increased Contrast shares the existing layered boundary). Settings sequence cards use the shared fold/inner-edge tokens with a `prefers-contrast: more` outline.

## P2

**P2 — Linux maximize chrome state drifts after Tauri-owned double-click**  
- **File:** `synara/src/app/features/desktop-titlebar/DesktopTitleBar.tsx`  
- **Lines:** 39–47 (state), 73–78 (label/icon), 59 (drag region)  
- **Trigger:** Double-click the Linux strip (Tauri `plugin:window|internal_toggle_maximize` from `drag.js`), or maximize via the window manager, then read the in-app Maximize/Restore button.  
- **Why:** React `maximized` updates only on `desktop_window_toggle_maximize`. Removing `onDoubleClick` correctly avoids a second owner, but nothing resyncs icon/`aria-label`. The button still toggles the real native flag, so this is label/state drift, not a dead control.  
- **Repair:** On show, and on Tauri resize/scale events, set `maximized` from `is_maximized` (or equivalent). Keep double-click on Tauri.

## Checks that did not fail

- **Hit targets:** Tauri 2.11.2 `data-tauri-drag-region` is self-only unless `"deep"`. Bare attr on the strip Box does not capture descendant button clicks; `BUTTON` in the composed path blocks drag. Inner empty `DragRegion` is the drag surface. Window controls stay on `IconButton` click handlers.
- **mac/Linux layout:** Custom strip is `isSynaraDesktop() && isLinuxOS()`; macOS/Windows keep native decorations. No second macOS strip. Overlay spacer is gone.
- **ACL:** One added permission, `windows: ["main"]`. No extra window commands.
- **a11y/theme:** Collection keeps a measurable Increased Contrast boundary; Settings contrast path uses `--synara-depth-contrast-edge` and drops decorative fill/shadow. `quietInteractiveSurface` adds hover/selected elevation, not a new text color.
- **Scope:** Changes stay on window chrome, splash flex, Settings cards/nav, and iOS collection metrics.

## Proof limitations (not code defects)

- Native macOS titlebar drag is **not confirmed**. CUA did not move the window; `CGPreflightPostEventAccess` was false. That is an input-automation boundary, not evidence of a remaining product drag bug, and not proof of success. Physical-pointer confirmation remains.
- This review did not inspect screenshots. Chromium splash geometry (y=40 / height=680 / bottom=720), 27 iOS tests, and 9 desktop checks are taken as reported.
- CI is pending. Linux native drag was not exercised here.
- The user’s originally reported drag path stays Failed until a physical pointer relocates a non-fullscreen window.
