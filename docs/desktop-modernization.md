# Desktop modernization

This desktop wrapper owns native shell behavior that the Cinny web client cannot
provide on its own.

## Native tray

The app creates a native tray/status-bar item on macOS and Linux. The tray keeps
Cinny available after the main window is closed and exposes quick actions:

- Show Cinny
- Later
- Notifications
- Check for Updates
- Quit Cinny

Closing the main window hides it to the tray instead of quitting. Use the tray
Quit action or the app menu quit action to exit.

## Global shortcuts

The wrapper registers these shortcuts through Tauri's global shortcut plugin:

- `CmdOrCtrl+Shift+C`: show Cinny
- `CmdOrCtrl+Shift+L`: open Later
- `CmdOrCtrl+Shift+N`: open Notifications

The shortcuts route through the same navigation helper used by the tray, so
notification-center and Later inbox deep links share one path.

## Media permissions

The macOS bundle includes explicit camera and microphone usage descriptions for
Matrix calls. Linux media permission UX is
managed by WebKit/WebRTC and the desktop environment, but the wrapper keeps the
same web client permissions model.

## Web bridge

The wrapper injects `window.__CINNY_DESKTOP__` before the Cinny app loads. The
bridge exposes capability flags and canonical desktop routes so the web client
can adapt UI for tray, global shortcuts, updater, media permission, Later, and
notification-center workflows without assuming every deployment is native.

## macOS high-refresh mode

The wrapper exposes `desktop_set_high_refresh_rate(enabled)` and
`desktop_get_performance_capabilities()` for the web client. On macOS, the
command uses `tauri-plugin-macos-fps` to unlock the WKWebView frame-rate cap for
ProMotion/high-refresh displays. On Linux and Windows the command returns
`false` and does nothing.

This is intentionally experimental and off by default because the plugin uses a
private WebKit preference. The paired web client only shows the setting in the
macOS desktop app.

## Implemented integration points

The web modernization branch documents and implements the richer client features:
threading UX, notification center, room favorites/folder groups, and backend
agent workflow actions. This wrapper connects those web features to native
surfaces without duplicating Matrix account data or room state:

- Native notification clicks deep-link into room/event anchors through the web
  client's `navigateRoom(roomId, eventId)` path.
- Tray badge/count updates mirror active Later and notification summaries through
  the `desktop_set_badge_count` command.
- Backend-backed agent workflow actions emit typed `cinny://agent-action` Tauri
  events through `desktop_agent_action` rather than scraping message content.
  The wrapper re-validates action kinds, text lengths, markdown lengths, and
  HTTPS-only URLs before copying, opening, or emitting any action payload.
- Auto-update entry points emit the `cinny://desktop-action` check-updates event
  so the web client can present update state inside the app.

## Feature ownership

Some modernization state intentionally stays in the web client:

- Threading UX remains a web-client concern. The desktop wrapper should only
  focus and deep-link to thread or room anchors supplied by the web client.
- The notification-center overhaul is implemented in the web client. The wrapper
  owns native notification permission, delivery, activation, and tray/badge
  entry points.
- Room favorites and folder groups remain Matrix/client state in the web client.
  The wrapper does not keep a second native copy that could drift from sync.
- Backend-backed agent workflows are represented as structured web-client
  actions first. The wrapper forwards them as explicit Tauri commands/events so
  backend integrations can subscribe to a typed bridge.

## Coverage checklist

| Improvement | Status |
| --- | --- |
| Native tray/status menu | Landed in this wrapper. |
| Media permission polish | Landed through macOS camera/microphone usage strings and WebKit/WebRTC permission alignment. |
| Deeper threading UX | Landed in the paired web branch; this wrapper deep-links to supplied room/event/thread anchors. |
| Full notification-center overhaul | Landed in the paired web branch; this wrapper owns native notification activation and tray/badge entry points. |
| Native notification click deep-linking | Landed through web `navigateRoom(roomId, eventId)` clicks and wrapper focus/navigation events. |
| Tray badge/count updates | Landed through `desktop_set_badge_count`. |
| Rich per-room notification settings | Landed in the paired web branch; no desktop-local duplicate state. |
| Room favorites/folder groups | Landed as Matrix/client sidebar state in the paired web branch; no desktop-local duplicate state. |
| Backend-backed agent workflow bridge | Landed through `desktop_agent_action` and `cinny://agent-action` events. |
| macOS high-refresh rendering | Landed behind an experimental web setting backed by `tauri-plugin-macos-fps`; non-macOS platforms no-op. |
