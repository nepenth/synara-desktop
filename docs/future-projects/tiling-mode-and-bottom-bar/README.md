# Bottom-bar navigation, tiling mode and desktop integration

Status: in progress on the local branch `feature/tiling-bottom-bar` (started 2026-11-09, not pushed).
Pop-out windows (workstream P) are deferred at the user's request; D1–D6 proceed on the recommended option. Paths are relative to the repo root; `app/` is short
for `synara/src/app/`.

## What we are doing

1. Replace the 66px left rail with a bottom bar on the room-list column, and move spaces into a
   switcher on the list title (the pattern iOS already uses).
2. Add a runtime "tiling mode" that the Linux client enters when it runs under a tiling window
   manager such as Hyprland (Omarchy), with a setting to force it on or off.
3. Make narrow windows, keyboard navigation, pop-out room windows, deep links, a status command,
   notification actions and externally driven theming first-class.

There is no separate Omarchy build. One codebase, one Linux package.

## The consistency rule

Three layers. Every task below is tagged with the layer it belongs to.

| Layer | Applies to | What lives here |
| --- | --- | --- |
| **Universal** | macOS, Windows, Linux | Bottom bar and space switcher, width-driven layouts, keyboard navigation, pop-out windows, deep links, `synara status`, notification actions, the palette engine, font settings |
| **Tiling mode** | Linux, switched at runtime | No in-app title strip, tiling-friendly defaults |
| **Provider** | One platform or desktop | Omarchy palette reader, Waybar example, macOS reply action, Windows toast work |

Two consequences:

- Layout reacts to **window width**, never to "am I tiled". A 420px window looks the same on macOS
  and in a Hyprland tile. Tiling mode changes window chrome and defaults only.
- Anything the Rust side must know (counts, routes, actions) is owned by Rust, not pushed from the
  renderer, so it is the same for every window and every platform.

## What the code looks like today

Facts the plan depends on, from a survey on 2026-11-09 (`main` at 6b78d71e).

**Shell**
- Rail: `app/pages/client/SidebarNav.tsx`, tabs in `app/pages/client/sidebar/`. Only the rail offers
  the per-space unread overview, Activity and Approvals entry points, the unverified-device warning,
  the account avatar, and space drag-reorder, folders and context menus (`SpaceTabs.tsx`, 924 lines;
  order saved to `in.synara.spaces` account data).
- List column: `app/components/page/Page.tsx` (resizable 196–360px, `data-compact` under 232px).
  Home header in `app/pages/client/home/Home.tsx` (logo, server name, "+" and ⋮ menus); space header
  in `app/pages/client/space/Space.tsx`. Each route family wraps its own `PageRoot`/`PageNav`.
  `/approvals` has no list column.
- Breakpoints in `app/hooks/useScreenSize.ts`: Desktop > 1124, Tablet 751–1124, Mobile ≤ 750. Mobile
  is already a single-pane stack with Back buttons. Tablet forces threads full-width and drops the
  members drawer.
- **The main window's minimum size is 960×720** (`src-tauri/src/lib.rs`, window builder), so the
  Mobile layout is unreachable in a normal window today.
- Linux title strip: `app/features/desktop-titlebar/DesktopTitleBar.tsx`, 40px, mounted in
  `app/pages/App.tsx`. It also owns Ctrl+Q.
- Shortcuts are scattered. The nearest thing to a registry is
  `app/features/desktop-keyboard/appShortcuts.ts`. Previous/next room (Alt+↑/↓) clicks neighbouring
  DOM rows, so it only reaches rows the virtualized list has rendered. There is no next-unread jump.
- Theming: `app/pages/ThemeManager.tsx` already writes the surface ramp and accent as CSS variables on
  `body` at runtime. There is no interface-font or monospace-font setting.
- Settings: `app/state/settings.ts` (`SharedSettings`, `DesktopPlatformSettings`), plus the search
  index in `app/features/settings/settingsSearchIndex.ts`.

**Native**
- One window, label `main`. Session and client state live in the Rust process (`MatrixAuthState`,
  `synara_core::Core`); the webview holds derived UI state only, so a second window can attach.
- Main-only assumptions: the `synara-media:` protocol, `desktop_timeline_subscribe`, the room-list
  `emit_to`, file drop, menu, tray and `src-tauri/capabilities/main.json`.
- Desktop-environment detection exists twice (`desktop_integration.rs`, `desktop_shortcuts.rs`);
  neither knows Hyprland, Sway or Niri.
- No URL scheme on desktop (iOS registers `synara`). Single-instance ignores `argv`. `main.rs` parses
  no arguments.
- Notifications: Linux (notify-rust) and macOS have approval actions; no mark-read or reply. Windows
  is title and body only.
- Unread, highlight and approval totals are combined in the renderer
  (`app/pages/client/ClientNonUIFeatures.tsx`) and pushed to the tray. Rust keeps no aggregate.
- No file watching anywhere. Nothing sets the Wayland app id explicitly.

## Decisions needed before the navigation work starts

Each has a recommendation; the tasks assume it. On 2026-11-09 the user asked for the plan to be
executed without answering these one by one, so the recommended option is what gets built and each
stays open to change at the review build. D5 is moot while pop-out windows are deferred.

| # | Question | Recommendation |
| --- | --- | --- |
| D1 | Where do space reorder and folders go? | Drag to reorder inside the switcher menu; folders show as collapsible groups. Creating a folder moves to a "Manage spaces" dialog rather than drag-onto. |
| D2 | Approvals has no list column, so the bar would vanish there. | Give Approvals the Home list as its nav column so the bar is always present. |
| D3 | Bar items: icons only, or icons with labels? | Icons only with tooltips; labels appear when the column is wider than about 300px. |
| D4 | What does the account avatar open? | A small menu: Settings, Verify this device (when needed), Sign out. The unverified warning becomes a badge on the avatar. |
| D5 | Can one room be open in two windows? | No. Popping out a room moves it; opening it again focuses its window. |
| D6 | Shortcut bindings (see K2). | As proposed, after a conflict check against the composer. |

## Workstreams and tasks

Sizes: S is under a day, M is one to three days, L is more. "Done when" is the acceptance check.

### F. Foundations

**F1. One desktop-environment module** · Tiling mode · S
- Merge the two `is_wayland`/`is_kde` copies into `src-tauri/src/desktop_environment.rs`.
- Add `WindowEnvironment { Floating, Tiling }`. Tiling when any of `HYPRLAND_INSTANCE_SIGNATURE`,
  `SWAYSOCK`, `NIRI_SOCKET`, `I3SOCK` is set, or `XDG_CURRENT_DESKTOP` names Hyprland, sway, niri,
  i3, river or dwl. macOS and Windows always report Floating.
- Inject it through the init script next to `os` (the bridge's `desktopEnvironment` field is
  "unknown" today).
- New `DesktopPlatformSettings.tilingMode: 'auto' | 'on' | 'off'`, a `useTilingMode()` hook, a
  Settings tile (Linux only) and a search-index entry.
- Done when: unit tests cover the env matrix; on Hyprland the hook returns true; toggling the setting
  changes it without a restart.

**F2. Let the window be narrow** · Universal · S
- Lower the minimum window size to about 360×480 and adjust the "smaller than minimum" reset.
- Done when: the window resizes into the Mobile layout on all three platforms with no clipped or
  overlapping chrome on sign-in, Home, a room, Settings and Approvals. File what breaks into L1.

**F3. Shortcut registry** · Universal · M
- Grow `appShortcuts.ts` into the single table of `{ id, default binding per platform, label,
  scope }`. Move ⌘K, Esc-closes-thread and Ctrl+Q into it. The macOS menu reads its accelerators
  from the same table.
- Done when: no window-level `keydown` listener outside the registry handles an app shortcut;
  existing shortcut tests pass.

**F4. Rust owns the status aggregate** · Universal · M
- Add `DesktopStatus { unread, highlights, pending_approvals, invites, later_due, dnd }` in
  Rust, updated from the room-list presentation totals and the approvals inbox. Tray, badge and
  (later) the status file read from it. The renderer stops computing the total and subscribes.
- Done when: tray and Dock badges match today's values with the main window closed to tray.

### N. Navigation: bottom bar and space switcher

Universal. Supersedes the wave-3 rail ("Home, spaces, Activity, Approvals, account"); the other
review-2 decisions stand. Depends on D1–D4.

**N1. Shared list-column frame** · M
- `PageNav` gains `header` and `footer` slots. A new `NavPaneHeader` replaces `HomeHeader` and
  `SpaceHeader`: switcher title, "+" (Home) and ⋮.
- Done when: Home, Space, Explore and Activity lists render through the same frame with no visual
  change yet.

**N2. Bottom bar** · M
- `NavBottomBar` in the footer slot: Home, Activity, Approvals, account. Reuse the badge logic from
  `HomeTab`, `InboxTab`, `ApprovalsTab`, `UnverifiedTab`. Right-click "Mark as read" on Home stays.
- Done when: every rail badge state has an equivalent on the bar (unit tests ported from the tabs).

**N3. Space switcher** · L
- Title menu listing Home and every space with avatar and unread badge; folders as groups; a dot on
  the title chevron when another space has unread. Context menu per row (Mark as read, Invite, Copy
  link, Space settings, Unpin). Reorder per D1, still saved to `in.synara.spaces`.
- Fully keyboard operable; type-ahead filter when there are more than about eight spaces.
- Done when: a 20-space account with folders can do everything `SpaceTabs` allowed except
  drag-onto-to-create-folder, which lives in Manage spaces.

**N4. Approvals keeps the bar** · S — wrap `/approvals` in `PageRoot` with the Home list (D2).

**N5. List toggle moves to the content header** · S
- When the list is hidden, `RoomViewHeader` (and the other page headers) show the show-list button
  with the unread dot that `RoomListTab` carries today. Keep `ROOM_LIST_TOGGLE_ID` focus restore.

**N6. Remove the rail** · M
- Delete `SidebarNav`, the tab components and `components/sidebar/` once nothing imports them; drop
  `MobileFriendlyClientNav`. Fix docs links (`check:workflows`), unused locale keys and Playwright
  selectors.
- Done when: tsc, eslint, unit tests and the six Playwright suites pass.

**N7. Review build** — local macOS build for judging, with a 50+ room account. Not merged before
that.

### T. Tiling mode chrome

Tiling mode. Depends on F1.

**T1. Hide the title strip** · S
- `DesktopTitleBar` renders only when not in tiling mode. Ctrl+Q already moved in F3.
- Check that the native drag hit-reporter (`desktop_window_drag.rs`) is happy with zero rectangles.
- Done when: under Hyprland the content starts at the top edge; switching the setting shows and
  hides the strip live.

**T2. Stable window identity** · S
- Set the GTK program name and Wayland app id explicitly to `synara`; confirm with
  `hyprctl clients`. Document the class and example window rules in `docs/linux.md`.

**T3. Tiling defaults** · S
- In tiling mode: skip restoring window size and position (the compositor decides).

### L. Width-driven layouts

Universal. Depends on F2 and N.

**L1. Narrow audit and fixes** · M
- Walk every route at 380, 520, 700, 900 and 1100px. Known items: the Mobile list route gets the
  bottom bar instead of the rail; sign-in and Settings at 380px; modals wider than the window.
- Done when: a Playwright viewport matrix covers those widths for Home, room, thread, Settings and
  Approvals.

**L2. Tablet range** · M
- 751–1124px: allow the members drawer as an overlay; auto-hide the list when a thread opens and
  restore it on close.

**L3. Measure the window, not the page** · S
- Confirm `useScreenSize` behaves with page zoom and in pop-out windows (each webview measures its
  own body).

### K. Keyboard

Universal. Depends on F3.

**K1. Navigate from data, not the DOM** · M
- Previous/next room and the new next/previous unread walk the room-list snapshot, then scroll the
  virtualized list to the target. Closes the review-2 deferred item "keyboard pane toggle /
  next-unread".

**K2. Bindings** · M — proposed, subject to D6. No Super-key bindings on Linux.

| Action | macOS | Windows and Linux |
| --- | --- | --- |
| Home, Activity, Approvals, Settings | ⌘1–4 | Ctrl+1–4 |
| Space switcher | ⌘⇧S | Ctrl+Shift+S |
| Next / previous unread room | ⌥⇧↓ / ⌥⇧↑ | Alt+Shift+↓ / Alt+Shift+↑ |
| Cycle focus list → timeline → composer → thread | F6 / ⇧F6 | F6 / Shift+F6 |
| Shortcut help | ⌘/ | Ctrl+/ |

**K3. Shortcut help sheet** · S — generated from the registry.

**K4. Optional list keys** · S — j/k and g g / G when the list has focus, behind a setting that is
on by default in tiling mode.

### I. Desktop integration

**I1. Deep links** · Universal · M
- Add `tauri-plugin-deep-link`; register `synara://` (Info.plist, `.desktop`
  `MimeType=x-scheme-handler/synara`, Windows via the plugin). Reuse the iOS grammar: read the iOS
  parser first and share the route table in `synara-core` if it is not already there.
- Single-instance forwards `argv` to the running app, which navigates.
- Done when: `xdg-open synara://room/<id>` focuses the app on that room, cold and warm, on all three
  platforms. Closes "single instance and deep links" in `docs/windows-support-spike.md`.

**I2. `synara status`** · Universal · M · depends on F4
- The app writes `status.json` to its runtime directory on every `DesktopStatus` change.
- `main.rs` handles `synara status [--json|--waybar] [--follow]` before Tauri starts: it reads the
  file and exits without opening a window. `synara open <uri>` forwards a deep link.
- Provider: a Waybar module example (click runs `synara open synara://activity`) in `docs/linux.md`.
- Done when: the count in Waybar changes within a second of a new mention, with the window hidden.

**I3. Notification actions** · Universal core, provider front ends · L
- Shared action ids `message.mark-read` and `message.reply`, handled in Rust without needing the
  renderer (reuse the session-generation gate in `desktop_notification_responses.rs`).
- Linux: add both actions; use inline reply where the server advertises it, otherwise Reply opens
  the room with the composer focused (mako has no inline reply).
- macOS: text-input reply action.
- Windows: actionable toasts are their own task (needs an AUMID and a different toast API); track it
  with the open item in the Windows doc.

### P. Pop-out room windows (deferred)

Deferred on 2026-11-09: no need yet. The one use the user sees is putting a second room on another
workspace. Kept here so the design is not lost. Universal. Depends on N5 and D5.

**P1. Second-window plumbing** · L
- Window label `room-<n>`; capability file `src-tauri/capabilities/room-window.json` with the subset
  a room needs. Allow those labels in the `synara-media:` protocol and `desktop_timeline_subscribe`.
  Audit every `MAIN_WINDOW_LABEL` use and each global `emit`.
- Sync settings between windows (localStorage `storage` events into the jotai atoms).
- Done when: a room window shows a live timeline, media, and sends messages; closing it leaks no
  timeline channel.

**P2. Room window shell** · M
- Route `/window/room/:id`: room header, timeline, composer, thread pane; no list, no bar. In
  floating mode on Linux it keeps a slim title strip; in tiling mode none.

**P3. Rules** · M
- One window per room (D5); the main window leaves a popped-out room. Notifications are suppressed
  for a room visible in any focused window. Read receipts follow the focused window. Quit closes
  all; closing main to tray leaves room windows open. Window state is not restored across restarts
  in the first version.

**P4. Thread pop-out** · M — later, same plumbing.

### H. Theme and fonts

**H1. Palette engine** · Universal · M
- `ThemeManager` accepts an `ExternalPalette { background, foreground, accent, ... }` and derives
  the surface ramp and Primary tokens from it the way it does from `themeBaseColor` today, with
  contrast clamping so any palette stays readable.
- Rust: `desktop_system_palette` command and a change event, backed by a provider trait.

**H2. Font settings** · Universal · S
- Appearance: Interface font (Default, Monospace, custom family) and Monospace font. Drives
  `--font-secondary` and `--font-mono`.

**H3. Omarchy provider** · Provider · M
- **Spike first:** confirm on the Omarchy box where the active theme lives and its format (expected
  under `~/.config/omarchy/current/`), and how the current font is exposed. The workstation was
  unreachable while this plan was written.
- Read the palette, watch for theme switches (add the `notify` crate; watch the parent directory,
  since a theme switch swaps a symlink), and offer "Follow Omarchy theme" in Appearance only when
  the provider is available.
- Done when: `omarchy-theme-set` re-themes a running Synara within a second.

**H4. System accent providers** · Provider · S each, optional — macOS and Windows accent colour
through the same trait, so "follow the system" exists on every platform.

## Order of work

1. F1, F2, F3 (independent of each other; F4 can run alongside).
2. N1 → N2 → N3 → N4 → N5 → N6 → N7. This is the biggest visible change, so it gets judged first.
3. T1, T2 and L1 on the Omarchy box, then L2, L3.
4. K1–K4.
5. I1, I2, I3.
6. T3, then H1, H2, then the H3 spike and provider; H4 when wanted.
7. P (deferred).

Review gates: local build after N (macOS), after T and L (Omarchy). Releases can
ship per group; N should not ship without L1, because the narrow list route depends on the bar.

## Risks

- **Space management regressions.** `SpaceTabs` is the most intricate part of the rail. N3 ports
  behaviour test by test; D1 deliberately drops one gesture.
- **Multi-window.** The main-only assumptions are spread across protocol, commands, tray and events.
  P1 is the task most likely to grow.
- **WebKitGTK at small sizes** and under Hyprland fractional scaling: check early in F2.
- **Windows notifications** lag the other two platforms until the toast work is done.
