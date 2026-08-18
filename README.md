# Synara

Synara is a native-first Matrix client for macOS, Linux, and iOS. It combines
secure messaging, room and timeline workflows, native platform integration,
and structured agent interactions in one repository.

Synara does not ship a standalone browser client. The React/Vite package under
`synara/` is the application runtime embedded by the Tauri desktop shell.

## Product Channels

| Client | User interface  | Matrix/application core                           | Distribution                                 |
| ------ | --------------- | ------------------------------------------------- | -------------------------------------------- |
| macOS  | Tauri 2 + React | Shared Rust core with native macOS adapters       | Signed/notarized DMG for production releases |
| Linux  | Tauri 2 + React | Shared Rust core with native Linux adapters       | `.deb` and Arch-family package assets        |
| iOS    | SwiftUI         | Shared Rust core through generated Swift bindings | Internal TestFlight, then App Store release  |

Windows, Android, and public web distribution are not currently supported.

## Architecture

The repository has one Matrix application engine and two native UI shells:

- `crates/synara-core/` owns shared Matrix lifecycle, room, timeline, messaging,
  account-data, and security behavior through `matrix-rust-sdk`.
- `src-tauri/` owns desktop-only concerns such as Keychain/Secret Service,
  windows, tray, shortcuts, notifications, file paths, and byte-oriented media
  transfer.
- `synara/` owns the desktop React UI, presentation state, Slate composer, and
  timeline virtualization. It is not a separately supported web product.
- `synara-ios/` owns the native SwiftUI app, Apple platform services, Keychain,
  APNs integration, and the notification service extension.

The generic shared-core command envelope intentionally excludes credentials,
recovery material, local file paths, and large media bytes. Platform adapters
retain those responsibilities.

See [ADR 0004](docs/adr/0004-rust-language-boundaries.md) for the binding
rules and [the shared-core documentation](docs/shared-native-core/README.md)
for migration history and implementation detail.

## Repository Layout

```text
.
|-- crates/synara-core/       Shared Rust application core
|-- crates/synara-core-bindgen/
|                             Swift binding generator
|-- src-tauri/                macOS/Linux native shell and adapters
|-- synara/                   Desktop React/Vite application runtime
|-- synara-ios/               Native SwiftUI application and extension
|-- devAssets/                Generated desktop runtime consumed by Tauri
|-- scripts/                  Build, validation, release, and policy tooling
|-- integration/synapse/      Disposable local Synapse integration harness
|-- packaging/                Linux package metadata
|-- docs/                     Architecture, operations, and historical records
`-- .github/workflows/        CI, package smoke, signing, and release automation
```

`synara/` is a normal tracked directory, not a submodule. Fresh clones do not
need `--recursive` or any submodule command.

## Prerequisites

- Node.js at the exact version in `.node-version` (currently 24.13.1).
- Rust at the version in `rust-toolchain.toml` with required platform targets.
- Tauri 2 platform prerequisites.
- Xcode and XcodeGen for iOS work.
- Linux system packages documented in [docs/linux.md](docs/linux.md) for Linux
  builds.

Install JavaScript dependencies from the repository root:

```sh
npm ci
npm --prefix synara ci
```

## Desktop Development

Run the desktop application in development mode:

```sh
npm run tauri dev
```

Build the platform's default desktop bundle:

```sh
npm run tauri build
```

Build an ad-hoc local macOS application bundle:

```sh
npm run tauri build -- --bundles app
```

Build the desktop runtime without packaging:

```sh
npm run build:runtime
```

Edit the repository root `config.json` only. `scripts/build-runtime.mjs` copies
the canonical configuration and generated runtime into the locations consumed
by Tauri.

## iOS Development

Generate the repository-local Swift bindings and XCFramework before opening a
clean checkout in Xcode:

```sh
scripts/generate-synara-core-swift.sh
cd synara-ios
xcodegen generate
```

Compile the app and test bundles for the simulator:

```sh
cd synara-ios
scripts/ci-build.sh
```

Run the simulator suite on an installed simulator:

```sh
cd synara-ios
RUN_IOS_TESTS=1 \
IOS_TEST_DESTINATION='platform=iOS Simulator,name=iPhone 17 Pro' \
scripts/ci-build.sh
```

Live Matrix tests require disposable accounts supplied through the local
environment. Credentials must never be added to source, fixtures, screenshots,
logs, or documentation.

## Validation

Core repository checks:

```sh
npm run check:repo-layout
npm run check:versions
npm run check:docs
npm run check:matrix-boundaries
npm run check:quality-gates
npm run check:synapse-harness
npm run check:production-smoke
```

Desktop runtime checks:

```sh
npm --prefix synara run typecheck
npm --prefix synara run test:modernization
npm --prefix synara run test:browser:timeline
npm --prefix synara run check:eslint
npm --prefix synara run check:prettier
```

Rust checks:

```sh
cargo test --workspace --locked
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

The complete release-oriented command list is maintained in
[docs/build-and-release.md](docs/build-and-release.md).

## Releases

All clients share one version. Use the repository version tool rather than
editing package metadata independently:

```sh
npm run bump:version -- X.Y.Z --ios-build X.Y.Z
```

An exact `vX.Y.Z` tag on `main` starts the coordinated release workflow. The
workflow reruns desktop and iOS validation, builds the signed desktop packages,
uploads the matching iOS archive to internal TestFlight, verifies App Store
Connect state, and publishes release assets only after all required gates pass.

Production signing, notarization, updater, TestFlight, and pacman repository
requirements are documented in
[docs/build-and-release.md](docs/build-and-release.md). Local builds use the
committed disabled-updater configuration and do not contact an update channel.

## Credential And Privacy Rules

This repository is public. Never commit:

- Matrix passwords, access tokens, refresh tokens, recovery keys, session
  exports, homeserver administration credentials, or private server details.
- Apple certificates, provisioning profiles, App Store Connect private keys,
  APNs keys, Developer ID material, or Tauri updater private keys.
- `.env` files, private URLs, personal filesystem paths, device identifiers,
  crash logs containing account data, or screenshots of non-disposable rooms.

Use ignored local environment files for disposable test accounts and an
external, permission-restricted directory for signing material. CI consumes
only named GitHub secrets and variables; documentation must contain names and
placeholders, never values.

## Documentation

Start with [docs/README.md](docs/README.md) for the current documentation map.
[CODEBASE_KNOWLEDGE_BASE.md](CODEBASE_KNOWLEDGE_BASE.md) provides a concise
source-oriented architecture guide.

Dated plans, audits, handoffs, progress logs, and acceptance reports are kept
for engineering provenance. Unless a document explicitly says it is current,
its status, branch, version, commit, test count, and remaining-work statements
describe that historical snapshot. Current source, ADRs, validation contracts,
and release workflows take precedence.

## License

Synara is licensed under the GNU Affero General Public License v3.0-only. See
[LICENSE](LICENSE) for the license and [NOTICE](NOTICE) for copyright and
third-party attribution that must accompany distributions.
