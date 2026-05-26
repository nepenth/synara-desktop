# Synara iOS

Status: Phase 0 planning home.

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

Shared contracts are currently owned by:

```text
../synara/docs/contracts
```

Do not add production credentials, APNs keys, provisioning profiles, App Store
Connect API keys, test account passwords, or homeserver admin tokens to this
directory.
