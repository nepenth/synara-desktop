# Synara iOS

Synara iOS is the repository's native SwiftUI Matrix client. It uses the shared
Rust application core through a generated repository-local `SynaraCore` Swift
package and adds Apple-specific UI, Keychain, APNs, media, and application
lifecycle behavior.

The app is distributed to internal testers through TestFlight. Release
readiness is tracked by evidence in the validation and release documents, not
by old phase labels in planning files.

## Architecture

- SwiftUI owns presentation, navigation, accessibility, and Apple platform
  integrations.
- `../crates/synara-core/` owns Matrix lifecycle and domain behavior.
- `../crates/synara-core-bindgen/` generates the Swift interface and
  XCFramework.
- `SynaraCore` build products are generated locally and remain ignored.
- Cross-platform schemas and fixtures live in `../synara/docs/contracts/`.

Start with the [codebase knowledge base](../CODEBASE_KNOWLEDGE_BASE.md),
[iOS architecture ADR](../docs/adr/0002-ios-architecture.md), and
[Rust language boundaries ADR](../docs/adr/0004-rust-language-boundaries.md).

## Local Build

On a clean checkout, generate the Swift package and XCFramework before opening
or building the Xcode project. This requires the repository Rust toolchain,
Xcode, the configured Apple Rust targets, and XcodeGen.

```sh
cd ..
scripts/generate-synara-core-swift.sh
cd synara-ios
xcodegen generate
```

Compile the simulator app and test bundles:

```sh
scripts/ci-build.sh
```

Run tests on an installed simulator:

```sh
RUN_IOS_TESTS=1 \
IOS_TEST_DESTINATION='platform=iOS Simulator,name=iPhone 17 Pro' \
scripts/ci-build.sh
```

List available destinations when the named simulator differs:

```sh
xcodebuild \
  -project Synara.xcodeproj \
  -scheme Synara \
  -showdestinations
```

Signed physical-device, archive, and TestFlight operations require the
maintainer's Apple Developer team and protected signing credentials. See the
[release checklist](docs/release-checklist.md) and
[device readiness guide](docs/device-readiness.md).

## Live Testing

Live Matrix tests use disposable accounts and rooms. Supply credentials through
an ignored local environment file or protected CI secret. Never add account
passwords, access tokens, homeserver admin tokens, APNs keys, provisioning
profiles, App Store Connect keys, or recovery material to source, fixtures,
screenshots, logs, or documentation.

The simulator path can validate login, sync, room/timeline behavior, encrypted
messaging, sends, rich formatting, settings, and session restore. APNs delivery,
physical-device background behavior, signing, and TestFlight upgrade behavior
require their corresponding real-device or App Store Connect evidence.

## Current Evidence

- [iOS validation status](docs/ios-validation-status.md)
- [iOS functionality matrix](docs/ios-functionality-matrix.md)
- [iOS visual fidelity matrix](docs/ios-visual-fidelity-matrix.md)
- [Release checklist](docs/release-checklist.md)
- [Live simulator smoke](docs/live-simulator-smoke.md)
- [E2EE validation](docs/e2ee-validation.md)
- [Push gateway staging](docs/push-gateway-staging.md)
- [Logging policy](docs/logging-policy.md)

The [project specification](../synara/docs/synara-ios-project-spec.md),
[App Store plan](../synara/docs/synara-ios-app-store-plan.md), and numbered
phase plans remain useful scope and decision history. Treat dated completion
claims in those files as historical unless confirmed by the current evidence
documents above.
