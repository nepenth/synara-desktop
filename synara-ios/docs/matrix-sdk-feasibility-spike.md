# Matrix Rust SDK Swift Feasibility Spike

Reviewed: 2026-05-26

Status: package probe complete; real login not attempted.

Related task: IOS-0006 in
[Synara iOS Project Spec](../../synara/docs/synara-ios-project-spec.md).

## Recommendation

Proceed with the native SwiftUI plus Matrix Rust SDK plan.

The official Swift package resolved, downloaded its prebuilt binary artifact,
compiled the Swift wrapper sources, linked a local probe executable, and the
probe ran successfully. This is enough to start a native iOS skeleton and a
more serious Matrix service wrapper.

Do not attempt real login/session work until the iOS app shell has a Keychain
session abstraction, redacted logging, and test-account-only configuration.

## Sources Checked

- Matrix Rust Components Swift:
  <https://github.com/matrix-org/matrix-rust-components-swift>
- Matrix Rust Components Swift manifest:
  <https://raw.githubusercontent.com/matrix-org/matrix-rust-components-swift/main/Package.swift>
- Matrix Rust SDK:
  <https://github.com/matrix-org/matrix-rust-sdk>

The current package manifest advertises Swift tools 5.7, iOS 16+, macOS 12+,
and the binary artifact:

```text
MatrixSDKFFI.xcframework.zip
```

at release `26.05.13`.

## Probe Location

```text
synara-ios/spikes/matrix-sdk-probe
```

The probe imports `MatrixRustSDK` without creating a client, storing
credentials, or contacting a homeserver.

## Commands Run

Resolve:

```sh
HOME=/private/tmp/synara-swift-home \
XDG_CACHE_HOME=/private/tmp/synara-swift-cache \
CLANG_MODULE_CACHE_PATH=/private/tmp/synara-clang-module-cache \
SWIFT_MODULE_CACHE_PATH=/private/tmp/synara-swift-module-cache \
swift package resolve
```

Build:

```sh
HOME=/private/tmp/synara-swift-home \
XDG_CACHE_HOME=/private/tmp/synara-swift-cache \
CLANG_MODULE_CACHE_PATH=/private/tmp/synara-clang-module-cache \
SWIFT_MODULE_CACHE_PATH=/private/tmp/synara-swift-module-cache \
swift build
```

Run:

```sh
.build/arm64-apple-macosx/debug/MatrixSDKProbe
```

## Results

- SwiftPM resolved `matrix-rust-components-swift` at normalized version
  `26.5.13`, revision `02133b466cddbd5c911881acbb29cf14e5563344`.
- SwiftPM downloaded:

```text
https://github.com/matrix-org/matrix-rust-components-swift/releases/download/26.05.13/MatrixSDKFFI.xcframework.zip
```

- The build compiled Matrix Rust SDK Swift wrapper files including:
  - `matrix_sdk.swift`
  - `matrix_sdk_base.swift`
  - `matrix_sdk_common.swift`
  - `matrix_sdk_crypto.swift`
  - `matrix_sdk_ffi.swift`
  - `matrix_sdk_ui.swift`
- The build produced a local macOS probe binary.
- `codesign --verify` passed on the probe binary.
- Running the probe printed:

```text
MatrixRustSDK import succeeded.
```

## Build Caveat

`swift build` exited non-zero after link during SwiftPM's apply/signing step:

```text
internal error in Code Signing subsystem
```

The binary was still produced, code-sign verification passed, and the executable
ran successfully. Treat this as a local SwiftPM/macOS signing quirk to resolve
before adding this probe to CI. It did not prevent module import, wrapper
compilation, or executable runtime validation.

The link step also emitted many warnings that bundled objects were built for a
newer macOS version than the package's declared macOS 12 floor. This matters for
future macOS reuse, but the iOS app target is the main concern for this spike.

## SDK Coverage Assessment

This spike verified package resolution and module import only. The following
coverage still needs a real test homeserver spike:

| Area            | Status                                             | Next proof                                                                        |
| --------------- | -------------------------------------------------- | --------------------------------------------------------------------------------- |
| Login           | Available through SDK surface, not exercised       | Test-account password login in native app shell or a dedicated integration probe. |
| Session restore | Not exercised                                      | Keychain-backed session store plus SDK restore test.                              |
| Room list       | Not exercised                                      | Mock first, then test homeserver room-list sync.                                  |
| Timeline        | Not exercised                                      | Wrapper around SDK timeline APIs with fixture-backed mapping tests.               |
| E2EE            | SDK includes crypto wrapper sources; not exercised | Test encrypted room sync with non-production account.                             |
| Media           | Not exercised                                      | Upload/download wrapper with authenticated media policy.                          |
| Pusher APIs     | Not exercised                                      | APNs token mock plus Matrix pusher registration against staging homeserver.       |

## Minimum Platform Implications

- The official Swift package currently declares iOS 16+.
- Synara iOS should set iOS 16 as the initial minimum unless a later SDK release
  changes the supported platform floor.
- SwiftUI app structure can still target modern iOS patterns without supporting
  older devices that the SDK does not support.

## Follow-Up Work

- Add a native iOS architecture ADR selecting SwiftUI plus Matrix Rust SDK as
  the primary path and Tauri iOS as non-shipping for now.
- Create the real `Synara.xcodeproj` under `synara-ios/`.
- Add a `SynaraMatrix` module that wraps SDK APIs behind app-owned protocols.
- Add redacted logging before any real auth calls.
- Add Keychain storage before any persisted session work.
- Use only test accounts and test rooms for the first login and room-list spike.

## Acceptance Result

- Swift package integration: passed.
- Local module import: passed.
- Real login: intentionally not attempted.
- Recommendation: proceed to IOS-0007 architecture ADR, then Phase 1 native iOS
  skeleton.
