# Matrix Rust SDK Swift Feasibility Spike

Reviewed: 2026-07-21

Status: package probe upgraded; live E2EE probe previously completed.

Related task: IOS-0006 in
[Synara iOS Project Spec](../../synara/docs/synara-ios-project-spec.md).

## Recommendation

Proceed with the native SwiftUI plus Matrix Rust SDK plan.

The official Swift package resolved, downloaded its prebuilt binary artifact,
compiled the Swift wrapper sources, linked a local probe executable, and the
probe ran successfully. A gated live probe also validated password login,
SDK crypto initialization, encrypted room detection, encrypted timeline
pagination, and encrypted message send against a disposable test room.

Do not move production app traffic to the SDK until the app has a narrow
`SynaraMatrix` wrapper, Keychain-backed app session metadata, redacted logging,
and gated test-account-only live validation.

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

at release `26.06.06`.

## Probe Location

```text
synara-ios/spikes/matrix-sdk-probe
```

The default probe imports `MatrixRustSDK` without creating a client, storing
credentials, or contacting a homeserver. The gated `live-e2ee` mode uses
environment-provided disposable credentials and does not print passwords,
access tokens, or refresh tokens.

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

Live E2EE run:

```sh
SYNARA_MATRIX_PROBE=live-e2ee \
SYNARA_E2EE_HOMESERVER=<test homeserver> \
SYNARA_E2EE_USERNAME=<test username> \
SYNARA_E2EE_PASSWORD=<test password> \
SYNARA_E2EE_ROOM=<encrypted room id, alias, or display name> \
SYNARA_E2EE_SEND=1 \
.build/arm64-apple-macosx/debug/MatrixSDKProbe
```

## Results

- SwiftPM resolved `matrix-rust-components-swift` at normalized version
  `26.6.6`, revision `ec3b2161ba371a13609e7181077d2f3baef188f5`.
- SwiftPM downloaded:

```text
https://github.com/matrix-org/matrix-rust-components-swift/releases/download/26.06.06/MatrixSDKFFI.xcframework.zip
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
- The earlier gated live E2EE validation (performed before the 26.06.06 package
  refresh) succeeded with:
  - SDK password login.
  - SDK E2EE initialization.
  - Encrypted joined-room discovery.
  - Room encryption state `encrypted`.
  - Timeline pagination.
  - Encrypted send acceptance.
  - Zero unable-to-decrypt callbacks in the observed timeline window.

## Build Caveat

With 26.06.06, `swift build` completed successfully, `codesign --verify` passed,
and the executable ran successfully. The earlier local SwiftPM apply/signing
failure did not recur during this refresh.

The link step also emitted many warnings that bundled objects were built for a
newer macOS version than the package's declared macOS 12 floor. This matters for
future macOS reuse, but the iOS app target is the main concern for this spike.

## SDK Coverage Assessment

The 26.06.06 refresh verified package resolution, wrapper compilation, module
import, and probe runtime compatibility. The earlier live probe exercised the
homeserver paths below; remaining app-integration depth is noted separately:

| Area            | Status                                                        | Next proof                                                                        |
| --------------- | ------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| Login           | Exercised in live probe with disposable password credentials. | Move login behind app-owned SDK wrapper and Keychain session metadata.            |
| Session restore | Not exercised.                                                | Keychain-backed session store plus SDK restore test.                              |
| Room list       | Exercised enough to discover joined encrypted rooms.          | SDK-backed app room-list service and simulator smoke.                             |
| Timeline        | Exercised for encrypted pagination and listener updates.      | App timeline mapper around SDK timeline APIs with fixture-backed mapping tests.   |
| E2EE            | Exercised for initialization, encrypted room state, and send. | App service integration, recovery, verification, key backup, and encrypted media. |
| Media           | Not exercised.                                                | Upload/download wrapper with authenticated and encrypted media policy.            |
| Pusher APIs     | Not exercised.                                                | APNs token mock plus Matrix pusher registration against staging homeserver.       |

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
- Move auth/session restore/room list/timeline send from REST-backed services to
  SDK-backed services.
- Preserve safe placeholders for UTD or unsupported encrypted states.
- Add recovery, verification, key backup, and encrypted media before external
  TestFlight/App Store release.

## Acceptance Result

- Swift package integration: passed.
- Local module import: passed.
- Live SDK login: passed with disposable credentials supplied through
  environment variables.
- Live SDK E2EE room validation: passed.
- Recommendation: integrate Matrix Rust SDK behind app-owned protocols before
  claiming production encrypted-room support.
