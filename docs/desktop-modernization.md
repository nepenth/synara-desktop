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
future notification-center and Later inbox deep links can share one path.

## Media permissions

The macOS bundle includes explicit camera and microphone usage descriptions for
Matrix calls, voice messages, and audio recording. Linux media permission UX is
managed by WebKit/WebRTC and the desktop environment, but the wrapper keeps the
same web client permissions model.

## Web bridge

The wrapper injects `window.__CINNY_DESKTOP__` before the Cinny app loads. The
bridge exposes capability flags and canonical desktop routes so the web client
can adapt UI for tray, global shortcuts, updater, media permission, Later, and
notification-center workflows without assuming every deployment is native.

## Follow-up integration points

The web modernization branch documents and implements the richer client features:
threading UX, notification center, room favorites/folder groups, and backend
agent workflows. Desktop-specific follow-up work should connect those web
features to native surfaces:

- Native notifications should deep-link into room/event anchors.
- Tray badge/count updates mirror active Later and notification summaries through
  the `desktop_set_badge_count` command.
- Backend-backed agent workflow actions emit typed `cinny://agent-action` Tauri
  events through `desktop_agent_action` rather than scraping message content.
- Auto-update UI should listen for the `cinny://desktop-action` check-updates
  event and present update state inside the app.

## Feature ownership

Some modernization work is intentionally not implemented in the native wrapper:

- Threading UX remains a web-client concern. The desktop wrapper should only
  focus and deep-link to thread or room anchors supplied by the web client.
- The notification-center overhaul remains a web-client concern. The wrapper
  owns native notification permission, delivery, activation, and tray/badge
  entry points.
- Room favorites and folder groups remain Matrix/client state in the web client.
  The wrapper can expose shortcuts or tray sections after the web client provides
  a stable summary API.
- Backend-backed agent workflows should be represented as structured web-client
  actions first. The wrapper can later expose native export, clipboard, file, or
  notification integrations through explicit Tauri commands/events.
