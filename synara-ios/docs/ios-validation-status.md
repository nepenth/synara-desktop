# iOS Validation Status

Reviewed: 2026-05-27

Status: Phase 1 shell/foundation, Phase 2 auth/session/sync/room-list/logout,
Phase 3 timeline UI, composer, event actions, media viewer, and media upload
MVP work are build-validated and simulator runtime validated with deterministic
mock services.

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

## Local Simulator Validation

Local simulator execution is unblocked as of 2026-05-27. Validation ran on an
iPhone 17 Pro simulator using iOS 26.5.

```sh
xcodebuild -project Synara.xcodeproj -scheme Synara -configuration Debug \
  -destination 'platform=iOS Simulator,id=<simulator-id>' \
  -derivedDataPath /private/tmp/synara-ios-mcp-derived \
  -only-testing:SynaraTests test

xcodebuild -project Synara.xcodeproj -scheme Synara -configuration Debug \
  -destination 'platform=iOS Simulator,id=<simulator-id>' \
  -derivedDataPath /private/tmp/synara-ios-mcp-derived \
  -only-testing:SynaraUITests test
```

Results:

- `SynaraTests`: 49 tests, 0 failures.
- `SynaraUITests`: 9 tests, 0 failures.

UI tests launch the app with `SYNARA_UI_TESTS=1`, which forces deterministic
mock services instead of live Keychain, auth, and Matrix dependencies.

## Live Matrix Simulator Findings

Live validation on May 27, 2026 used a dedicated test account on a private test
homeserver. Credentials, homeserver details, and tokens were not written to
source files, tests, or git.

Findings are tracked in ordered implementation items in
[`synara/docs/synara-ios-project-spec.md`](../../synara/docs/synara-ios-project-spec.md):

- Unsigned simulator builds can compile the app but cannot validate
  Keychain-backed login persistence.
- Signed simulator login succeeded and transitioned into the Rooms shell.
- The test account had an `Alerts` invite; accepting it refreshed the room list
  into a joined-room row.
- The room timeline opened, but the title fell back to `Room` instead of
  preserving the `Alerts` display name.
- The initial live timeline showed Matrix state events as unsupported chat rows.
- The composer accepted text, but the send icon needed stronger hit-target and
  automation validation.
- Simulator accessibility hierarchy capture was incomplete during manual smoke,
  so the canonical live-smoke path still needs accessibility hardening.

The repeatable live-smoke checklist is
[`synara-ios/docs/live-simulator-smoke.md`](live-simulator-smoke.md).

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
- UI smoke tests assert primary tabs exist and Settings can be selected.

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
  navigation to the login form.
- Login screen accepts username and password input.
- Live auth uses the Matrix Client-Server `m.login.password` flow: it checks
  `/_matrix/client/v3/login` for password support, submits the password login
  request, and maps successful responses into secure app sessions.
- Mock auth remains forced for UI tests through `SYNARA_UI_TESTS=1`.
- Successful mock login updates the observable session store and transitions to
  the signed-in tab shell.
- Failed login shows non-sensitive errors and does not persist credentials.
- Unit tests cover auth request validation, Matrix login request construction,
  unsupported login flows, invalid credentials, mock auth fixtures, and session
  state transitions.
- A live homeserver flow check can be run manually, but no live test
  homeserver, username, or password is stored in the repository.
- UI tests cover missing-credential errors and successful mock login.
- Secure session storage supports save, load, delete, corrupt-entry handling,
  and legacy envelope migration through the app session store contract.
- Login saves sessions through secure storage before transitioning to the
  signed-in shell.
- Matrix lifecycle service exposes stopped, starting, syncing, and failed sync
  states, with explicit start, stop, and local-reset hooks.
- Live room list loading uses `/_matrix/client/v3/sync?timeout=0` with the
  secure session access token and maps joined and invited rooms into stable
  room summaries.
- Room list service renders loading, empty, failed, and loaded states with
  stable room IDs, unread counts, highlight state, invite previews, and
  1,000-room fixtures.
- Invited rooms expose native accept and decline actions backed by Matrix
  room membership endpoints.
- Live timeline loading uses Matrix room messages for joined rooms and maps
  text, reply, media, redacted, encrypted, and unknown events into the native
  timeline model.
- Live message sending uses Matrix `m.room.message` send with transaction IDs
  and supports reply metadata for text messages.
- Live edit sending uses Matrix `m.replace` replacement content.
- Live redaction and reaction actions use Matrix event endpoints and update the
  local timeline only after successful responses.
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
  shell.
