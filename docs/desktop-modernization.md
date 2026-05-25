# Desktop modernization

This desktop wrapper owns native shell behavior that the Synara app runtime cannot
provide on its own.

## Native tray

The app creates a native tray/status-bar item on macOS and Linux. The tray keeps
Synara available after the main window is closed and exposes quick actions:

- Show Synara
- Later
- Notifications
- Quit Synara

Closing the main window hides it to the tray instead of quitting. Use the tray
Quit action or the app menu quit action to exit.

## Global shortcuts

The wrapper registers these shortcuts through Tauri's global shortcut plugin:

- `CmdOrCtrl+Shift+C`: show Synara
- `CmdOrCtrl+Shift+L`: open Later
- `CmdOrCtrl+Shift+N`: open Notifications

The shortcuts route through the same navigation helper used by the tray, so
notification-center and Later inbox deep links share one path.

## Media permissions

The macOS bundle includes explicit camera and microphone usage descriptions for
Matrix calls. Linux media permission UX is
managed by WebKit/WebRTC and the desktop environment, but the wrapper keeps the
same app runtime permissions model.

## Runtime bridge

The wrapper injects `window.__SYNARA_DESKTOP__` before the Synara app loads. The
bridge exposes capability flags and canonical desktop routes so the app runtime
can adapt UI for tray, global shortcuts, media permission, Later, and
notification-center workflows without duplicating native wrapper logic.

## Performance diagnostics

The wrapper exposes `desktop_get_performance_capabilities()` so the app runtime
can report platform and build identity in diagnostics. Desktop smoothness work
is handled through the app runtime's bounded timeline rendering, scroll
anchoring, and instrumentation rather than private WebKit refresh-rate toggles.

## Implemented integration points

The app-runtime modernization branch documents and implements the richer client
features: threading UX, notification center, room favorites/folder groups, and
backend agent workflow actions. This wrapper connects those runtime features to
native surfaces without duplicating Matrix account data or room state:

- Native notification clicks deep-link into room/event anchors through the app
  runtime's `navigateRoom(roomId, eventId)` path.
- Tray badge/count updates mirror active Later and notification summaries through
  the `desktop_set_badge_count` command.
- Backend-backed agent workflow actions emit typed `synara://agent-action` Tauri
  events through `desktop_agent_action` rather than scraping message content.
  The wrapper re-validates action kinds, text lengths, markdown lengths, and
  HTTPS-only URLs before copying, opening, or emitting any action payload.
- Auto-update entry points are disabled in this branch. A signed release channel
  should be configured and tested before reintroducing a user-visible update
  menu item.

## Feature ownership

Some modernization state intentionally stays in the app runtime:

- Threading UX remains an app-runtime concern. The desktop wrapper should only
  focus and deep-link to thread or room anchors supplied by the app runtime.
- The notification-center overhaul is implemented in the app runtime. The wrapper
  owns native notification permission, delivery, activation, and tray/badge
  entry points.
- Room favorites and folder groups remain Matrix/client state in the app runtime.
  The wrapper does not keep a second native copy that could drift from sync.
- Backend-backed agent workflows are represented as structured app-runtime
  actions first. The wrapper forwards them as explicit Tauri commands/events so
  backend integrations can subscribe to a typed bridge.

## Coverage checklist

| Improvement                            | Status                                                                                                    |
| -------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Native tray/status menu                | Landed in this wrapper.                                                                                   |
| Media permission polish                | Landed through macOS camera/microphone usage strings and WebKit/WebRTC permission alignment.              |
| Deeper threading UX                    | Landed in the app runtime; this wrapper deep-links to supplied room/event/thread anchors.                 |
| Full notification-center overhaul      | Landed in the app runtime; this wrapper owns native notification activation and tray/badge entry points.  |
| Native notification click deep-linking | Landed through runtime `navigateRoom(roomId, eventId)` clicks and wrapper focus/navigation events.        |
| Tray badge/count updates               | Landed through `desktop_set_badge_count`.                                                                 |
| Rich per-room notification settings    | Landed in the app runtime; no desktop-local duplicate state.                                              |
| Room favorites/folder groups           | Landed as Matrix/client sidebar state in the app runtime; no desktop-local duplicate state.               |
| Backend-backed agent workflow bridge   | Landed through `desktop_agent_action` and `synara://agent-action` events.                                 |
| Desktop fluidity                       | Landed through runtime rendering and scroll-anchor improvements; no private WebKit refresh-rate override. |
