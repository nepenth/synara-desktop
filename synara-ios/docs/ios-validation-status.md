# iOS Validation Status

Reviewed: 2026-05-26

Status: Phase 1 skeleton build-validated; simulator runtime execution pending.

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

The app target builds for a generic iOS simulator destination when DerivedData
is redirected outside the sandbox:

```sh
xcodebuild -project Synara.xcodeproj \
  -scheme Synara \
  -destination 'generic/platform=iOS Simulator' \
  -derivedDataPath /private/tmp/synara-ios-derived \
  build
```

Result: `BUILD SUCCEEDED`.

The app, unit test, and UI test targets compile with:

```sh
xcodebuild -project Synara.xcodeproj \
  -scheme Synara \
  -destination 'generic/platform=iOS Simulator' \
  -derivedDataPath /private/tmp/synara-ios-derived \
  build-for-testing
```

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
- Placeholder screens with iOS 16-compatible SwiftUI.
- Unit smoke tests for tab and sheet identifiers.
- UI smoke test that asserts primary tabs exist once simulator execution works.
