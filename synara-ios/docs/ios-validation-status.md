# iOS Validation Status

Reviewed: 2026-05-26

Status: Phase 1 shell/foundation and Phase 2 homeserver selection are
build-validated; simulator runtime execution pending.

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
