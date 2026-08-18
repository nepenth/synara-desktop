# Synara Codebase Knowledge Base

> Living architecture guide for contributors and coding agents.
>
> Last reviewed: 2026-08-18. Source code, manifests, and release workflows take
> precedence if a dated planning or migration document disagrees with this file.

## Product Summary

Synara is a native-first Matrix client shipped through three product channels:

| Client | UI              | Application engine                                | Distribution                    |
| ------ | --------------- | ------------------------------------------------- | ------------------------------- |
| macOS  | Tauri 2 + React | Shared Rust core                                  | Signed/notarized DMG            |
| Linux  | Tauri 2 + React | Shared Rust core                                  | Debian and Arch-family packages |
| iOS    | SwiftUI         | Shared Rust core through generated Swift bindings | TestFlight and App Store        |

The repository does not support a standalone browser client. The React/Vite
package is the embedded desktop application runtime. Windows and Android are
also outside the supported release matrix.

All clients use the same product version. Version consistency is enforced by
`npm run check:versions`, and an exact version tag coordinates desktop and iOS
publication.

## Architecture

Synara has one Matrix application engine and two UI shells:

```text
crates/synara-core/
        |
        +-- src-tauri/ + synara/       macOS and Linux
        |
        `-- generated Swift bindings + synara-ios/   iOS
```

### Shared Rust Core

`crates/synara-core/` owns Matrix lifecycle and domain behavior through
`matrix-rust-sdk`, including authentication, sync, rooms, timelines, sends,
account data, media policy, and security-sensitive Matrix operations.

`crates/synara-core-bindgen/` generates the Swift package and XCFramework used
by the iOS app. Generated artifacts are build outputs, not an independent
implementation.

The core exposes typed operations where practical. Its generic command
envelope must not carry credentials, recovery secrets, local file paths, or
large media bytes. See [ADR 0004](docs/adr/0004-rust-language-boundaries.md).

### Desktop Shell

`src-tauri/` owns platform behavior that belongs to macOS or Linux:

- application windows, tray, menus, shortcuts, and lifecycle;
- Keychain or Secret Service integration;
- native notifications, badges, external URLs, and file access;
- platform diagnostics and release hardening;
- narrow bridge adapters between the UI and shared core.

`synara/` owns desktop presentation: React state, routing, settings, the Slate
composer, room/timeline rendering, and timeline virtualization. Product code
must reach Matrix through the native bridge and shared core, not through a
second JavaScript Matrix client.

Important entry points:

| Concern           | Entry point                                     |
| ----------------- | ----------------------------------------------- |
| Desktop process   | `src-tauri/src/main.rs`, `src-tauri/src/lib.rs` |
| Core bridge       | `src-tauri/src/bridge/`                         |
| React bootstrap   | `synara/src/index.tsx`                          |
| Platform facade   | `synara/src/app/platform/`                      |
| Matrix facade     | `synara/src/app/matrix/`                        |
| Desktop app shell | `synara/src/app/pages/App.tsx`                  |

### iOS Shell

`synara-ios/` is a native SwiftUI application. It consumes the repository-local
`SynaraCore` Swift package and owns Apple-specific concerns:

- SwiftUI navigation, scenes, settings, and accessibility;
- Keychain session material and protected local state;
- APNs registration, notification routing, and the notification extension;
- Photos, camera, Files, share sheets, and other Apple UI surfaces;
- App Store signing, archive, TestFlight, and device behavior.

Important entry points:

| Concern            | Entry point                                  |
| ------------------ | -------------------------------------------- |
| Application        | `synara-ios/Synara/App/SynaraApp.swift`      |
| Dependency graph   | `synara-ios/Synara/App/AppEnvironment.swift` |
| Root UI            | `synara-ios/Synara/App/RootShellView.swift`  |
| Services           | `synara-ios/Synara/Services/`                |
| Shared contracts   | `synara-ios/Synara/Contracts/`               |
| Project definition | `synara-ios/project.yml`                     |

## Repository Topology

| Path                          | Ownership                                                |
| ----------------------------- | -------------------------------------------------------- |
| `crates/synara-core/`         | Shared Rust Matrix/application core                      |
| `crates/synara-core-bindgen/` | Swift binding generation                                 |
| `src-tauri/`                  | macOS/Linux native shell and bridge                      |
| `synara/`                     | Embedded desktop React application runtime               |
| `synara-ios/`                 | Native SwiftUI application and extension                 |
| `synara/docs/contracts/`      | Cross-platform schemas and fixtures                      |
| `integration/synapse/`        | Disposable local integration server harness              |
| `packaging/`                  | Linux packaging metadata                                 |
| `scripts/`                    | Build, validation, policy, and release tooling           |
| `.github/workflows/`          | CI, package smoke, and exact-tag release automation      |
| `docs/`                       | Current operations plus historical architecture evidence |

`synara/` is a normal tracked directory. There are no required submodules or
sibling repositories.

## Product Capabilities

The implemented surface includes authentication and secure session restore,
joined-room navigation, spaces, room administration, encrypted timelines,
rich message composition and rendering, replies, edits, reactions, threads,
polls, media and file workflows, search, notifications, Later items, room
notes, and structured agent cards/actions. Platform presentation and some
operating-system integrations differ by client.

Capability status changes frequently. Use these sources instead of copying
counts or phase labels into new documents:

- [desktop validation status](docs/desktop-validation-status.md);
- [iOS validation status](synara-ios/docs/ios-validation-status.md);
- [iOS functionality matrix](synara-ios/docs/ios-functionality-matrix.md);
- [shared contracts](synara/docs/synara-contracts.md);
- [production smoke checklist](docs/production-smoke-checklist.md).

## State And Security Boundaries

- Matrix credentials and recovery material belong in platform credential
  storage, never local documentation or generic command payloads.
- macOS uses Keychain; Linux uses Secret Service when available; iOS uses
  Keychain and protected application storage.
- Large media bytes and local file paths stay in typed platform operations.
- Logs must use redacted identifiers and diagnostic IDs. Never log access
  tokens, passwords, APNs tokens, private room/event identifiers, or recovery
  material.
- Live tests use disposable accounts supplied through ignored environment
  files or CI secrets. They must not be embedded in fixtures or commands.
- `config.json` contains public product defaults only. It is not a secret store.

## Shared Contracts

Cross-platform product semantics are defined under `synara/docs/contracts/`.
When changing a shared behavior:

1. Update the human contract and schema.
2. Add or update canonical fixtures.
3. Update the shared core behavior and desktop adapter.
4. Validate the desktop consumer.
5. Update the iOS consumer and conformance tests.
6. Update capability and validation documentation.

Do not create platform-specific interpretations of route, notification, Later,
agent-action, account-data, or rich-text contracts without documenting the
intentional difference.

## Build And Validation

Install dependencies:

```sh
npm ci
npm --prefix synara ci
```

Primary repository gates:

```sh
npm run check:repo-layout
npm run check:versions
npm run check:docs
npm run check:matrix-boundaries
npm run check:quality-gates
npm run check:synapse-harness
npm run check:production-smoke
```

Desktop runtime and Rust gates:

```sh
npm --prefix synara run typecheck
npm --prefix synara run test:modernization
npm --prefix synara run test:browser:timeline
npm --prefix synara run check:eslint
npm --prefix synara run check:prettier
cargo test --workspace --locked
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

iOS binding generation and simulator validation:

```sh
scripts/generate-synara-core-swift.sh
cd synara-ios
xcodegen generate
scripts/ci-build.sh
```

See [the build and release runbook](docs/build-and-release.md) for packaging,
signing, and publication.

## Release Model

Normal pushes and pull requests validate the repository. Release candidate
branches may build smoke artifacts. A pushed exact version tag is the only
production release source and coordinates:

- signed and notarized macOS artifacts;
- Linux package artifacts and repository metadata;
- a signed iOS archive uploaded to internal TestFlight;
- final release publication only after every required client succeeds.

Secrets are supplied through protected CI environments or local ignored files.
No release key, certificate, App Store Connect credential, updater private key,
or live test credential belongs in Git.

## Documentation Rules

[The documentation index](docs/README.md) identifies living guides and dated
records. A migration packet may accurately describe an old state while being
wrong as current architecture; such records must carry a historical or
superseded banner.

When architecture changes:

1. update the root README and this knowledge base;
2. update the relevant ADR or add a new one;
3. update validation and release runbooks;
4. mark displaced plans as historical rather than silently leaving them
   authoritative;
5. run `npm run check:docs` and the normal quality gates.

## Engineering Invariants

- One shared Rust Matrix/application core; no second production Matrix backend.
- Two UI shells: desktop React/Tauri and native SwiftUI.
- No standalone browser product or browser fallback for native security work.
- No required submodules or sibling repositories.
- No credentials, private infrastructure names, personal paths, or live account
  identifiers in tracked documentation.
- Platform APIs own platform behavior; feature code does not call Tauri or Apple
  services ad hoc.
- Shared behavior changes include contract and cross-platform test updates.
- All release clients carry the same product version.
- Historical evidence remains clearly labeled and is not treated as current
  implementation guidance.
