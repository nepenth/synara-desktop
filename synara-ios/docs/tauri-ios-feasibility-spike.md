# Tauri iOS Feasibility Spike

Reviewed: 2026-05-26

Status: preflight complete; simulator launch not completed.

Related task: IOS-0005 in
[Synara iOS Project Spec](../../synara/docs/synara-ios-project-spec.md).

## Recommendation

Do not use Tauri iOS as the default shipping architecture for Synara iOS.

Keep the native SwiftUI plus Matrix Rust SDK path as the primary plan. Tauri iOS
can remain a tactical compatibility experiment after Xcode first-launch and
simulator services are fixed, but it should not block native iOS Phase 0 or
Phase 1 work.

## Commands Run

Preflight:

```sh
PATH=<node-runtime-bin>:$PATH npx tauri --version
rustup target list --installed
xcodebuild -version
PATH=<node-runtime-bin>:$PATH npx tauri ios --help
```

Init:

```sh
PATH=<node-runtime-bin>:$PATH npx tauri ios init --ci --skip-targets-install
```

Simulator target setup:

```sh
rustup target add aarch64-apple-ios-sim
```

Simulator build attempt:

```sh
PATH=<node-runtime-bin>:$PATH npx tauri ios build --debug --target aarch64-sim --ci
```

## Environment Findings

- Tauri CLI is available at `tauri-cli 2.7.1`.
- Xcode is available: `Xcode 26.5`, build `17F42`.
- Rust initially had only `aarch64-apple-darwin` installed.
- The spike installed `aarch64-apple-ios-sim`.
- Tauri iOS init requires Apple-side helper tools:
  - `xcodegen`
  - `libimobiledevice`
  - `cocoapods`
- The first init attempt failed because sandboxed Homebrew writes were blocked.
- The escalated init path installed `xcodegen` and `libimobiledevice`.
- `brew install cocoapods` was needed because Tauri attempted to reinstall a
  CocoaPods keg that did not exist.
- Homebrew ran its normal cleanup/autoremove during the escalated install.
- No local iOS code-signing certificate was detected.

Installed helper paths after the spike:

```text
/opt/homebrew/bin/xcodegen
/opt/homebrew/bin/pod
/opt/homebrew/bin/idevicesyslog
```

Installed Rust targets after the spike:

```text
aarch64-apple-darwin
aarch64-apple-ios-sim
```

## What Worked

- `tauri ios init --ci --skip-targets-install` completed after Apple helper
  dependencies were installed.
- Tauri generated an Xcode project at:

```text
src-tauri/gen/apple/synara.xcodeproj
```

- The generated project included:
  - `project.yml`
  - `Podfile`
  - `LaunchScreen.storyboard`
  - iOS app `Info.plist`
  - iOS entitlements
  - generated app icon assets
  - Objective-C++ entry point
- The current Synara runtime production build completed during the simulator
  build attempt.
- The web runtime still builds with the current WASM crypto asset:

```text
matrix_sdk_crypto_wasm_bg-dMeGppz-.wasm
```

## What Blocked Runtime Validation

The simulator build did not reach a launchable app. It blocked in Xcode
first-launch setup:

```text
xcodebuild -runFirstLaunch
```

`xcodebuild -checkFirstLaunchStatus` exited with code `69` and no useful
stdout. `xcrun simctl list devices available` also blocked through
`xcodebuild -runFirstLaunch` outside the sandbox. The hung build/check processes
were stopped manually.

Because of that environment blocker, this spike did not verify runtime behavior
inside the iOS simulator and did not produce screenshots.

## Static Feasibility Assessment

### IndexedDB

The current app runtime depends on browser storage and Matrix JS SDK browser
behavior. WKWebView can support IndexedDB, but the persistence, quota, and
process-lifetime behavior must be tested directly before trusting session,
crypto, and sync state.

Risk: high for shipping architecture, acceptable for a throwaway shell spike.

### WebCrypto And WASM

The production runtime build includes Matrix crypto WASM. WKWebView generally
supports WebCrypto and WASM, but this needs device and simulator validation
under the exact Tauri iOS webview configuration.

Risk: medium to high until the app launches and performs login/E2EE sync.

### Service Worker Assumptions

The runtime still builds a PWA service worker. A packaged iOS WKWebView should
not be treated like Safari PWA distribution. Offline, cache, update, and push
semantics need a native design instead of relying on service worker behavior.

Risk: high for App Store-grade behavior.

### Media Auth And Downloads

The desktop runtime has desktop-specific file handoff and media integration.
iOS needs native Photos, Files, share sheet, camera permission, background
upload constraints, and secure media cache policy. A WebView shell would need
bridging work for App Store-grade behavior.

Risk: high.

### Keyboard And Composer

The current composer is optimized for desktop and browser behavior. Native iOS
keyboard avoidance, input accessory behavior, selection, paste, media insertion,
and Dynamic Type would need extensive WKWebView-specific work.

Risk: high.

### Routing

The route contract can map into either Tauri or native iOS. This is one of the
lower-risk areas because the app now has explicit route fixtures and schemas.

Risk: low to medium.

### Push Notifications

Tauri iOS would still require native APNs registration, a Matrix pusher, a push
gateway, tap routing, badge management, and privacy-safe payload design. The
existing browser notification model does not solve iOS push.

Risk: high.

### Performance Feel

The runtime bundle is large and desktop-feature rich. Without a simulator or
device launch, performance is unproven. Native SwiftUI still gives better
control over room list, timeline, composer, Dynamic Type, VoiceOver, and iPad
layout.

Risk: high until measured.

### App Review Risk

A Tauri iOS shell risks looking like a repackaged web client unless it receives
substantial native integration. That cuts against the project goal of an
App Store-grade native Matrix client.

Risk: high.

## Generated Files Policy

The generated `src-tauri/gen/apple/` directory was removed after the spike. It
is not committed because Tauri iOS is not the selected shipping architecture and
the generated project was produced only to test feasibility.

## Follow-Up If We Revisit Tauri iOS

- Complete Xcode first-launch setup on the local machine.
- Confirm simulator services list devices successfully.
- Install the physical-device Rust target if a device test is needed:

```sh
rustup target add aarch64-apple-ios
```

- Rerun:

```sh
npx tauri ios init --ci --skip-targets-install
npx tauri ios build --debug --target aarch64-sim --ci
```

- Launch in simulator and verify:
  - login with a test homeserver
  - session persistence
  - IndexedDB and Matrix crypto store behavior
  - WASM crypto load
  - timeline scroll performance
  - keyboard/composer behavior
  - media upload/download
  - route handling

## Acceptance Result

- Tauri iOS init: passed after Apple helper dependencies were installed.
- Simulator build: blocked by Xcode first-launch/simulator service setup.
- Runtime launch: not completed.
- Recommendation: do not pursue Tauri iOS as the default shipping path; proceed
  with native SwiftUI and Matrix Rust SDK feasibility.
