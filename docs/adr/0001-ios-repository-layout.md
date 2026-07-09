# ADR 0001: iOS Repository Layout

Reviewed: 2026-05-26

Status: accepted for Phase 0.

## Decision

Create the native iOS project under the canonical `synara-desktop` repository:

```text
synara-desktop/
  src-tauri/
  synara/
  synara-ios/
```

The initial iOS project home is:

```text
synara-desktop/synara-ios
```

This keeps macOS, Linux, and iOS planning, contracts, CI, and release gates in
one source of truth after the former standalone `nepenth/synara` repository was
archived.

## Rationale

- The active app runtime already lives in `synara-desktop/synara`.
- Shared contracts already live in `synara-desktop/synara/docs/contracts`.
- Desktop CI already validates repository layout, version metadata, Rust shell,
  runtime typechecks, tests, lint, and formatting.
- A same-repo iOS project makes contract fixtures easy to consume from Swift
  tests without a package registry, submodule, or second repository sync.
- The project is still early enough that keeping history separated is less
  valuable than preventing split-brain development.

## Shared Contract Paths

iOS code and tests should reference the source contract artifacts in:

```text
../synara/docs/contracts
```

from inside `synara-ios/`.

The first iOS conformance tests should load JSON fixtures directly from that
path or from a generated test-resource copy. Do not fork contract files into
`synara-ios/` unless a build tool generates them from the canonical schemas.

## Expected Build Commands

Desktop package smoke:

```sh
cd synara-desktop
npm run check:repo-layout
npm run check:versions
npm run tauri build -- --bundles app
```

Runtime validation:

```sh
cd synara-desktop/synara
npm run typecheck:modernization
npm run test:modernization
npm run check:eslint
npm run check:prettier
```

iOS Phase 1 commands, once the Xcode project exists:

```sh
cd synara-desktop/synara-ios
xcodebuild -list -project Synara.xcodeproj
xcodebuild -project Synara.xcodeproj -scheme Synara -destination 'platform=iOS Simulator,name=iPhone 17' build
xcodebuild -project Synara.xcodeproj -scheme Synara -destination 'platform=iOS Simulator,name=iPhone 17' test
```

The simulator name can be adjusted to the installed Xcode runtime. CI must not
require Apple signing credentials for unsigned simulator builds.

## Rejected Alternatives

### Separate `nepenth/synara-ios` Repository

Rejected for Phase 0 because it would immediately reintroduce multi-repo drift.
Contracts, docs, CI, and release gates would need cross-repository automation
before the iOS app has proven its SDK path.

### Sibling Checkout Beside `synara-desktop`

Rejected because the project just removed confusing sibling runtime checkouts.
A sibling `synara-ios` checkout would be easy to confuse with the canonical
desktop repo and would make local instructions less deterministic.

### Git Submodule Or Git Subtree For Shared Contracts

Rejected because the repo intentionally removed the `synara` submodule. Shared
contracts should remain normal tracked files in this monorepo until there is a
clear external consumer that justifies packaging them separately.

## Consequences

- The root repository owns iOS docs, future iOS source, and iOS CI.
- iOS work can use local contract fixtures without network access.
- Release tags must clearly distinguish desktop-only releases from future
  multi-platform releases.
- If the iOS project later needs separate access control or release cadence, it
  can be split out after Phase 0 and Phase 1 prove the architecture.

## Acceptance Criteria

- The chosen layout is documented here.
- The rejected alternatives are recorded.
- Build commands for desktop, runtime, and future iOS work are documented.
- No existing runtime or desktop-shell files are moved by this decision.
