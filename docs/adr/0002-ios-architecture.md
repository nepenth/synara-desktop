# ADR 0002: Synara iOS Architecture

Reviewed: 2026-05-26

Status: accepted for Phase 1 scaffolding.

## Decision

Build Synara iOS as a native SwiftUI app backed by the Matrix Rust SDK Swift
components.

Tauri iOS is not the default shipping architecture. It can remain a tactical
experiment for compatibility research, but it should not block the native iOS
app skeleton, Matrix service wrapper, or App Store-grade UX work.

## Evidence

- The pre-iOS desktop/runtime consolidation is complete.
- Packaged macOS and Linux CI smoke builds passed after the repository was
  consolidated.
- [ADR 0001](0001-ios-repository-layout.md) places iOS in this monorepo under
  `synara-ios/`.
- [Tauri iOS feasibility spike](../../synara-ios/docs/tauri-ios-feasibility-spike.md)
  initialized a generated Xcode project, but simulator runtime validation was
  blocked by local Xcode first-launch/simulator setup. Static assessment still
  showed high risk for push, media, keyboard/composer behavior, service worker
  assumptions, performance, and App Review fit.
- [Matrix SDK feasibility spike](../../synara-ios/docs/matrix-sdk-feasibility-spike.md)
  resolved the official Swift package, downloaded the binary FFI artifact,
  compiled wrapper sources, linked a local probe, and successfully ran a
  `MatrixRustSDK` import probe.
- The official Matrix Rust Components Swift package currently declares iOS 16+
  and macOS 12+ support.

## Architecture

Initial module boundaries:

```text
synara-ios/
  Synara.xcodeproj
  Synara/
    App/
    Features/
    SharedUI/
  SynaraCore/
    Contracts/
    Logging/
    Persistence/
    Security/
  SynaraMatrix/
    MatrixClientService.swift
    SessionStore.swift
    RoomListService.swift
    TimelineService.swift
  SynaraPush/
    APNsRegistrationService.swift
    MatrixPusherService.swift
    NotificationRouter.swift
  SynaraAgent/
    AgentAction.swift
    AgentActionValidator.swift
```

The first scaffold may start smaller, but it should preserve these ownership
lines:

- `Synara` owns app entry, scenes, navigation, feature UI, and dependency
  installation.
- `SynaraCore` owns app-owned contracts, redacted logging, Keychain wrappers,
  fixture loaders, and portable value types.
- `SynaraMatrix` owns Matrix Rust SDK adaptation and hides SDK volatility from
  views.
- `SynaraPush` owns APNs, Matrix pusher registration, badge routing, and
  notification tap handling.
- `SynaraAgent` owns agent action/card validation against shared contracts.

## Shared Contracts

The iOS app must consume the existing shared contracts from:

```text
synara/docs/contracts
```

Swift types may be generated or manually mirrored, but fixture conformance tests
must use the same JSON fixtures as the desktop runtime. Contract files must not
be forked into the iOS project.

## App Store Consequences

- Native SwiftUI gives the best path for App Review, accessibility, iPad
  behavior, system permissions, and platform interaction quality.
- The app must still pass the AGPL/App Store legal review gate before external
  TestFlight or App Store submission.
- APNs, Matrix pusher registration, privacy-safe payloads, App Store privacy
  labels, export compliance, and signing/provisioning remain first-class release
  gates.

## Push Consequences

- Push is native APNs plus Matrix pusher registration, not browser
  notification semantics.
- The Matrix push gateway remains a separate infrastructure decision. Sygnal is
  still the reference starting point unless a Synara-operated gateway is
  justified.
- Push payloads default to generic content. Exact room/event context is
  recomputed after app open and local sync/decryption.

## Crypto And Session Consequences

- Use Matrix Rust SDK approved stores for Matrix state and crypto state.
- Use Keychain or SDK-approved secure storage for access tokens, restore
  handles, and bootstrap secrets.
- Do not persist decrypted message bodies outside SDK-required stores without a
  separate design.
- Logout must wipe Keychain entries, SDK stores, caches, local drafts, and
  pending push registration state.

## CI Consequences

- Phase 1 CI should start with unsigned simulator builds and unit tests.
- Signed device/archive CI waits for Apple Developer enrollment and approved
  secret storage.
- Matrix SDK binary downloads should be pinned through `Package.resolved`.
- The desktop CI must remain green when iOS files are added.

## Rejected Alternatives

### Ship Tauri iOS First

Rejected as the default architecture because it carries high risk for App
Store-grade push, storage, keyboard/composer behavior, accessibility, iPad
layout, and App Review perception. The current runtime is valuable for desktop,
but iOS needs native ownership of platform behavior.

### Rewrite macOS And Linux Native Before iOS

Rejected because macOS and Linux are already working on Tauri and remain
first-class. A desktop rewrite would delay iOS without proving shared Matrix
domain value first.

### Build A Separate Clean-Room iOS Repository Now

Rejected for Phase 1 because contracts, planning, and CI are now centralized in
this monorepo. Separate repository access can be revisited after the app
skeleton and Matrix path are proven.

## Acceptance Criteria

- Primary iOS implementation path is selected.
- Role of Tauri iOS is recorded.
- Matrix SDK strategy and module boundaries are recorded.
- Consequences for App Store review, push, crypto, shared contracts, CI, and
  maintenance are explicit.
