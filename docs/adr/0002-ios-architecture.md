# ADR 0002: Synara iOS Architecture

Originally accepted: 2026-05-26.

Last reviewed: 2026-09-01.

Status: accepted as amended by [ADR 0003](0003-shared-native-rust-core.md).

## Decision

Synara iOS is a native SwiftUI application. Swift owns scenes, navigation,
feature presentation, accessibility, Apple platform services, Keychain/APNs
integration, and the notification service extension.

ADR 0003 superseded this ADR's original direct Swift service-layer adaptation
of `matrix-rust-components-swift`. Shared Matrix lifecycle and application
authority now live in the project-owned `crates/synara-core`; iOS consumes that
Core through generated `SynaraCore` Swift/UniFFI bindings. Swift service objects
are thin platform adapters and projections, not a second Matrix engine.

Tauri iOS is not the shipping architecture. It may be used for isolated
research only and must not become a parallel product path without a replacement
ADR.

## Current ownership

### Swift/iOS owns

- SwiftUI views, navigation, layout, gestures, selection, accessibility, and
  Dynamic Type;
- app and scene lifecycle observations;
- Keychain and protected platform storage adapters;
- APNs registration, notification routing, NSE lifecycle, badges, and taps;
- Photos, camera, Files, share sheets, permissions, signing, and App Store
  behavior.

### Shared Core owns

- Matrix client lifecycle, sync, room/timeline state, writes, crypto, trust,
  receipts, account data, and shared product policy;
- typed models and operations consumed by both desktop and iOS;
- protocol validation and bounds that are independent of a UI output context.

The complete boundary rubric is [ADR 0004](0004-rust-language-boundaries.md).

## Current evidence

- Product Swift sources import the repository-local `SynaraCore` module; the
  direct `MatrixRustSDK` import remains only in the historical feasibility
  spike.
- `synara-ios/SynaraCore/` defines the local package and generated artifact
  boundary.
- `synara-ios/Synara/Services/SharedCore*` contains thin Swift adapters over the
  shared owner.
- The notification extension has a narrow lifecycle and must not start the full
  sync engine.

## Shared contracts and release consequences

- Cross-client schemas and fixtures remain canonical under
  `synara/docs/contracts/`; generated or mirrored Swift types must conform to
  those fixtures.
- Simulator CI can run unsigned. Signed archive, physical-device, APNs,
  privacy, export-compliance, and App Store validation remain release gates.
- Matrix session and crypto stores use Core/matrix-rust-sdk owners. Keychain
  holds platform secrets; decrypted message bodies are not independently
  persisted without a separate decision.
- Logout/local-wipe behavior must remove the scoped credentials, Core stores,
  caches, drafts, and push state defined by current product requirements.

## Rejected alternatives

- **Ship Tauri iOS:** rejected for native interaction, accessibility, keyboard,
  notification, lifecycle, and App Review reasons.
- **Maintain a second Swift Matrix engine:** superseded because it creates
  cross-client policy and state-machine drift.
- **Rewrite working desktop presenters before iOS:** rejected because it does
  not improve shared Matrix authority.
- **Separate iOS repository:** rejected by ADR 0001.

## Consequences

- Native iOS quality is preserved without duplicating Matrix/application logic.
- Platform observations flow into Core authority through typed boundaries; Core
  does not own SwiftUI or Apple lifecycle APIs.
- Any proposal to replace SwiftUI, ship Tauri iOS, or restore an independent
  Swift Matrix service layer requires a replacement ADR.
