# Synara iOS

Status: Native MVP with active Phase 6.7 fidelity validation and Phase 6.9
performance hardening.

This directory is reserved for the native SwiftUI iOS app. The current accepted
layout keeps iOS in the canonical `synara-desktop` monorepo beside the desktop
Tauri shell and the app runtime.

Authoritative planning documents:

- [Monorepo knowledge base](../CODEBASE_KNOWLEDGE_BASE.md) — start here for
  architecture, feature status, shared contracts, and expansion guidance across
  desktop, runtime, and iOS.
- [iOS project spec](../synara/docs/synara-ios-project-spec.md)
- [iOS App Store plan](../synara/docs/synara-ios-app-store-plan.md)
- [iOS repository layout ADR](../docs/adr/0001-ios-repository-layout.md)
- [iOS architecture ADR](../docs/adr/0002-ios-architecture.md)
- [Shared native Rust core ADR](../docs/adr/0003-shared-native-rust-core.md)
- [Rust language boundaries ADR](../docs/adr/0004-rust-language-boundaries.md)
- [Apple Developer enrollment checklist](docs/apple-developer-enrollment-checklist.md)
- [License inventory](docs/license-inventory.md)
- [Release checklist](docs/release-checklist.md)
- [Tauri iOS feasibility spike](docs/tauri-ios-feasibility-spike.md)
- [Matrix SDK feasibility spike](docs/matrix-sdk-feasibility-spike.md)
- [iOS validation status](docs/ios-validation-status.md)
- [Phase 6.7 functionality and visual fidelity plan](docs/phase-6-7-plan.md)
- [iOS functionality matrix](docs/ios-functionality-matrix.md)
- [iOS visual fidelity matrix](docs/ios-visual-fidelity-matrix.md)
- [Phase 6.9 performance plan](docs/phase-6-9-performance-plan.md)
- [iOS device readiness](docs/device-readiness.md)
- [iOS logging policy](docs/logging-policy.md)
- [iOS CI notes](docs/ci-notes.md)
- [Live simulator smoke](docs/live-simulator-smoke.md)
- [Push gateway staging](docs/push-gateway-staging.md)

Shared contracts are currently owned by:

```text
../synara/docs/contracts
```

Do not add production credentials, APNs keys, provisioning profiles, App Store
Connect API keys, test account passwords, or homeserver admin tokens to this
directory.

## Local Build

The app imports the repository-local `SynaraCore` package. On a clean checkout,
generate its Swift bindings and XCFramework **before** opening or building the
app (requires Xcode and Rust 1.93 with the Apple targets):

```sh
cd ..
scripts/generate-synara-core-swift.sh
cd synara-ios
```

`ci-build.sh` runs this generation automatically in iOS CI. Generated Swift,
headers, XCFrameworks, and build artifacts remain ignored and must not be
committed.

Generate the Xcode project after SynaraCore generation:

```sh
xcodegen generate
```

List schemes:

```sh
xcodebuild -list -project Synara.xcodeproj
```

Unsigned simulator build and test-bundle compilation, once local Xcode
first-launch setup is complete:

```sh
scripts/ci-build.sh
```

Run tests on a concrete installed simulator:

```sh
RUN_IOS_TESTS=1 IOS_TEST_DESTINATION='platform=iOS Simulator,name=iPhone 16' scripts/ci-build.sh
```

For live simulator smoke testing against a Matrix homeserver, use a signed
local simulator run from Xcode or XcodeBuildMCP. The unsigned CI build path is
compile-oriented and is not valid for Keychain-backed session validation.
