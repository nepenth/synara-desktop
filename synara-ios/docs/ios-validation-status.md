# iOS Validation Status

Reviewed: 2026-05-27

Status: Phase 1 shell/foundation, Phase 2 auth/session/sync/room-list/logout,
Phase 3 timeline UI, composer, event actions, media viewer, and media upload
MVP work are build-validated; simulator runtime execution pending.

## Project Shape

- `Synara.xcodeproj` exists under `synara-ios/`.
- `xcodegen generate` succeeds from `synara-ios/`.
- `xcodebuild -list -project Synara.xcodeproj` lists:
  - `Synara`
  - `SynaraTests`
  - `SynaraUITests`
- Shared scheme:
  - `Synara`

## Build Validation

The app target builds for a generic iOS simulator destination, and the app,
unit test, and UI test targets compile with:

```sh
scripts/ci-build.sh
```

Result: `BUILD SUCCEEDED`.

Result: `TEST BUILD SUCCEEDED`.

## Local Environment Blocker

Running tests or launching the simulator is currently blocked by a local Xcode
and CoreSimulator mismatch:

```text
CoreSimulator is out of date. Current version (1051.50.0) is older than build version (1051.54.0).
```

Until that local Xcode/simulator state is repaired, `xcrun simctl` cannot list
usable simulator runtimes and UI tests cannot execute.

## Current App Surface

- Native SwiftUI app entry point.
- `TabView` root shell.
- Independent `NavigationStack` path per primary tab.
- Primary tabs:
  - Rooms
  - Notifications
  - Later
- Settings
- Enum-backed routes and sheet destinations.
- Deep-link routing for placeholder room and settings destinations.
- App-level dependency registry installed through SwiftUI environment.
- Mock session, Matrix, push, logging, settings, and router services for tests
  and previews.
- Structured logging wrapper with redaction for tokens, APNs tokens, Matrix
  identifiers, event IDs, and URLs.
- iOS design-token baseline with shared empty, loading, error, and toolbar
  controls.
- Placeholder screens with iOS 16-compatible SwiftUI.
- Unit smoke tests for routing, dependency wiring, settings storage, and
  redaction.
- UI smoke tests that assert primary tabs exist and Settings can be selected
  once simulator execution works.

## Current Auth Surface

- Signed-out users land in a native homeserver selection flow.
- Homeserver addresses are normalized before discovery.
- Insecure `http://` homeserver input is rejected before discovery.
- Suggested homeservers are provided through the discovery service contract.
- Successful discovery routes to a login placeholder with the normalized
  homeserver base URL.
- Unit tests cover URL normalization, invalid input, mock discovery requests,
  and login routing.
- UI tests cover signed-out homeserver selection, invalid input, and successful
  navigation to the login placeholder once simulator execution works.
- Login screen accepts username and password input.
- Auth service contract supports password-login requests behind placeholder and
  mock implementations.
- Successful mock login updates the observable session store and transitions to
  the signed-in tab shell.
- Failed login shows non-sensitive errors and does not persist credentials.
- Unit tests cover auth request validation, mock auth fixtures, and session
  state transitions.
- UI tests cover missing-credential errors and successful mock login once
  simulator execution works.
- Secure session storage supports save, load, delete, corrupt-entry handling,
  and legacy envelope migration through the app session store contract.
- Login saves sessions through secure storage before transitioning to the
  signed-in shell.
- Matrix lifecycle service exposes stopped, starting, syncing, and failed sync
  states, with explicit start, stop, and local-reset hooks.
- Room list service renders loading, empty, failed, and loaded states with
  stable room IDs, unread counts, highlight state, and 1,000-room fixtures.
- Settings exposes logout through a local wipe service that stops sync, clears
  cached rooms, clears push registration state, deletes the secure session, and
  returns to the signed-out shell.
- Timeline service scaffolding normalizes raw events into stable timeline item
  models for text, media placeholders, redactions, unknown events, replies, and
  edits.
- Room timeline screen renders lazy rows with sender labels, reply/edit states,
  redactions, unknown event placeholders, media placeholders, and reaction
  summaries.
- Composer MVP supports multiline text input, empty-message guarding, local
  echo, send failure messaging, and per-room draft preservation.
- Event action service and context menu support reply, edit, redact, and react
  availability against the mock service layer.
- Media service supports authenticated media resources, safe media descriptions,
  viewer presentation, upload progress state, and sanitized upload display names.
- Unit tests cover secure session storage, Matrix lifecycle, room sorting and
  unread mapping, local wipe behavior, timeline mapping, composer/draft behavior,
  event action behavior, and media URL/path safety.
- UI tests cover opening a room from the room list, sending a mock message,
  adding a mock media attachment, and logout return to the homeserver selection
  shell once simulator execution works.
