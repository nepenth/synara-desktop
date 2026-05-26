# iOS License Inventory And Release Gate

Reviewed: 2026-05-26

Status: draft inventory. This is engineering tracking, not legal advice.

## Release Gate

External TestFlight and App Store submission are blocked until legal review
confirms the distribution strategy for AGPL-covered Synara code and all iOS
dependencies.

No license text is changed by this inventory.

## Current Repository Licenses

| Component                  | Location                               | Observed license       | Release impact                                                                                  |
| -------------------------- | -------------------------------------- | ---------------------- | ----------------------------------------------------------------------------------------------- |
| Synara desktop repository  | `LICENSE`                              | AGPL-3.0-only          | Blocking legal review for App Store distribution.                                               |
| Synara app runtime         | `synara/LICENSE`                       | AGPL-3.0-only          | Blocking legal review if code, UI structure, or derivative implementation is reused in iOS.     |
| Desktop shell dependencies | `package.json`, `src-tauri/Cargo.toml` | Mixed dependency graph | Must be inventoried before desktop release; not automatically part of native iOS unless reused. |
| Runtime web dependencies   | `synara/package.json`                  | Mixed dependency graph | Relevant for desktop. Relevant to iOS only if assets, code, or package logic are reused.        |

## Planned iOS Dependencies

| Dependency                                                               | Intended use                                                  | Observed license/status                                               | Source checked                                                                         |
| ------------------------------------------------------------------------ | ------------------------------------------------------------- | --------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Matrix Rust SDK                                                          | Matrix client, sync, E2EE, room state, media foundations      | Apache-2.0; project describes SDK as production ready                 | <https://github.com/matrix-org/matrix-rust-sdk>                                        |
| Matrix Rust Components Swift                                             | Swift Package wrapper for Matrix Rust SDK components          | Apache-2.0; README notes Swift components are unstable and may change | <https://github.com/matrix-org/matrix-rust-components-swift>                           |
| Tauri                                                                    | Feasibility spike only, not default shipping iOS architecture | MIT or Apache-2.0 where applicable                                    | <https://github.com/tauri-apps/tauri> and <https://v2.tauri.app/concept/architecture/> |
| SwiftUI, Foundation, UserNotifications, Security, AuthenticationServices | Native Apple frameworks                                       | Apple platform SDK terms                                              | Requires Apple Developer Program agreement review.                                     |

## Legal Questions To Resolve

- Can AGPL-3.0-only app code be distributed through Apple's App Store without
  adding impermissible restrictions?
- If not, is the right path an App Store exception, dual-license grant,
  contributor permission, clean-room native implementation, or source
  distribution mechanism?
- Are generated Swift types from JSON Schemas derivative of AGPL-covered runtime
  code or only contract artifacts?
- What source availability, attribution, in-app notice, and website notice are
  required for TestFlight and App Store builds?
- Does Matrix Rust SDK's precompiled binary distribution for Swift introduce
  additional notice, source, or export-control obligations?
- Do encryption features require export compliance answers in App Store Connect?

## Engineering Rules Until Review Closes

- Keep iOS Phase 0 and Phase 1 simulator work local and internal.
- Do not submit to external TestFlight or App Store Connect for review.
- Do not change repository license files without explicit legal approval.
- Do not copy desktop UI implementation wholesale into SwiftUI. Use shared
  contracts and product behavior as the compatibility layer.
- Track all new iOS dependencies in this file before adding them to the Xcode
  project or Swift Package manifest.

## Acceptance Criteria

- Current repo licenses are inventoried.
- Planned Matrix and Tauri spike dependencies are identified.
- AGPL/App Store compatibility is an explicit release blocker.
- No production distribution proceeds until the blocker is closed.
