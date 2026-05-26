# Synara iOS

Status: Phase 1 native skeleton started.

This directory is reserved for the native SwiftUI iOS app. The current accepted
layout keeps iOS in the canonical `synara-desktop` monorepo beside the desktop
Tauri shell and the app runtime.

Authoritative planning documents:

- [iOS project spec](../synara/docs/synara-ios-project-spec.md)
- [iOS App Store plan](../synara/docs/synara-ios-app-store-plan.md)
- [iOS repository layout ADR](../docs/adr/0001-ios-repository-layout.md)
- [iOS architecture ADR](../docs/adr/0002-ios-architecture.md)
- [Apple Developer enrollment checklist](docs/apple-developer-enrollment-checklist.md)
- [License inventory](docs/license-inventory.md)
- [Release checklist](docs/release-checklist.md)
- [Tauri iOS feasibility spike](docs/tauri-ios-feasibility-spike.md)
- [Matrix SDK feasibility spike](docs/matrix-sdk-feasibility-spike.md)
- [iOS validation status](docs/ios-validation-status.md)

Shared contracts are currently owned by:

```text
../synara/docs/contracts
```

Do not add production credentials, APNs keys, provisioning profiles, App Store
Connect API keys, test account passwords, or homeserver admin tokens to this
directory.

## Local Build

Generate the Xcode project:

```sh
xcodegen generate
```

List schemes:

```sh
xcodebuild -list -project Synara.xcodeproj
```

Simulator build and test, once local Xcode first-launch setup is complete:

```sh
xcodebuild -project Synara.xcodeproj -scheme Synara -destination 'generic/platform=iOS Simulator' -derivedDataPath /private/tmp/synara-ios-derived build
xcodebuild -project Synara.xcodeproj -scheme Synara -destination 'generic/platform=iOS Simulator' -derivedDataPath /private/tmp/synara-ios-derived build-for-testing
```
