# Build And Release Runbook

Reviewed: 2026-10-07

This is the entry point for agents and maintainers preparing Synara builds or
releases. Read this before changing packaging, signing, updater, TestFlight, or
release workflow behavior.

## Release Lanes

| Lane                | Purpose                                                                                              |         Client-visible update? |
| ------------------- | ---------------------------------------------------------------------------------------------------- | -----------------------------: |
| `main`              | Integration branch. Runs normal CI on push and PR.                                                   |                             No |
| `release/vX.Y.Z`    | Version and notes PR. Runs the client Quality gate, including iOS simulator unit tests.             |                             No |
| Pushed tag `vX.Y.Z` | Coordinated macOS, Linux, and internal TestFlight release.                                           | Yes, after every client passes |

The maintainer merges a green release PR and pushes the matching version tag as
the single deliberate publication action. The tag workflow builds and checks
the actual macOS, Linux, and iOS distributables before publishing them. Manual
device and installed-package smoke is recommended when relevant, but does not
block routine releases.

## Local Prerequisites

1. Install Node from `.node-version`.
2. Install Rust 1.96 (repo root `rust-toolchain.toml` pins the channel).
3. Install Tauri platform prerequisites.
4. Run dependency installation from both package roots:

```bash
npm ci
npm --prefix synara ci
```

Linux system package details live in [linux.md](linux.md). macOS local signing
and app replacement notes live in [macos-local-signing.md](macos-local-signing.md).

## Rust workspace and output paths

Desktop, shared Core, NSE, and bindgen use the root `Cargo.lock` and common
workspace dependencies. Desktop package output is under
`target/release/bundle/` (or `target/universal-apple-darwin/release/bundle/`
for universal macOS builds). Apple generators explicitly use
`target/synara-core-apple` for their isolated feature builds.

A desktop `--manifest-path src-tauri/Cargo.toml` invocation uses the same
workspace lockfile. CI validates the desktop package and the three shared
packages separately to avoid repeating desktop checks. NSE shipping isolation
is checked with its explicit production feature graph and archive exports;
workspace-wide tests do not substitute for that check.

Any shared Core source, nested FFI module, workspace manifest/lockfile,
`.cargo` configuration, or toolchain change triggers the Apple compile gate
on ordinary feature PRs. Main pushes run Apple unit tests for shared-Core/iOS
changes. Release PRs and the `needs-ios` label run the unit lane; XCUITests run
nightly, on manual dispatch, and with the `needs-ios-ui` label.

## CI shape

`CI` has one required check, `Quality gate`, which aggregates every job and
accepts jobs skipped by scope. Scopes are computed by `scripts/ci-scopes.sh`
and tested by `scripts/__tests__/ci-scopes.test.mjs`.

| Job                    | Pull requests and `main`                                      | Nightly / dispatch |
| ---------------------- | ------------------------------------------------------------- | ------------------ |
| Rust format and lint   | Rust, Cargo or CI changes                                      | Always             |
| Rust tests             | Same, in parallel with lint                                    | Always             |
| Node desktop runtime   | Renderer, scripts, workflow or packaging changes               | Always             |
| Rust dependency audit  | Lockfile or manifest changes                                   | Always             |
| iOS compile gate       | iOS/FFI changes when the simulator lane is skipped             | —                  |
| iOS simulator tests    | Release PRs, `needs-ios`, `main` pushes touching iOS/FFI       | Always             |
| iOS simulator UI tests | `needs-ios-ui` only                                            | Always             |
| Synapse live proofs    | Core, desktop Matrix bridge, Synapse harness or lockfile       | Always             |
| Package builds         | —                                                              | Nightly, or dispatch with `packages: true` |

The nightly run has its own concurrency lane, so pushes to `main` no longer
cancel it. The package jobs build the Linux `.deb`, the Arch package (from
the same Linux binary) and an ad hoc signed universal macOS app; on `main` they
also write the release compiler caches, and a final job prunes superseded cache
snapshots. Pull-request label events do not trigger CI; a `needs-ios` or
`needs-ios-ui` label applies from the next push. Flaky lanes retry once: the
native-timeline Playwright suite (`--retries=1`) and the iOS simulator suites
(`IOS_TEST_RETRY_ON_FAILURE=1`). Retried passes stay visible in the reports.

Workflow hygiene is `npm run check:workflows`: pinned
[actionlint](https://github.com/rhysd/actionlint) plus
`scripts/check-workflows.mjs` (SHA-pinned actions, explicit permissions, job
timeouts, step-scoped secrets) and the documentation hygiene check.

The CI workflow's manual inputs allow additional unsigned Apple validation:
`apple_slices=all` builds the four full-Core slices and the three NSE slices;
`check_ios_device_release=true` also builds the unsigned device Release app
and inspects its NSE archive. The device opt-in installs all Apple targets even
when the slice input retains its default. These inputs affect only the unit-test
job, have a bounded 120-minute budget, and do not sign, upload, or publish a
release. Defaults remain `simulator-arm64` and device Release disabled.

## Dependency and build-cache ownership

The 2026-10-05 audit keeps one Cargo workspace and lockfile. Shared versions
and default-feature policy belong in root `Cargo.toml`; each consuming crate
declares its actual imports and required capabilities. Repeating a compatible
dependency in two manifests does not by itself compile two copies. Cargo
unifies features within the selected graph; host/build dependencies, tests,
target architectures and profiles can legitimately need distinct artifacts.
See [Cargo feature resolution](https://doc.rust-lang.org/cargo/reference/features.html)
and [build-cache layout](https://doc.rust-lang.org/cargo/reference/build-cache.html).

Desktop no longer declares unused direct SDK/utility dependencies. Its direct
HTTP and Matrix UI test imports live in dev dependencies. Core's `full-app`
feature owns shared desktop/iOS services and SDK capabilities. Desktop opts
out of Core defaults and enables `full-app`, `search-index` and `x509-identity`;
it compiles neither Apple binding owners nor the UniFFI runtime/build tools.
Apple's full generator explicitly enables `full-uniffi`, which adds bindings
to `full-app`. Standalone Core retains that default for compatibility. UniFFI's
tooling defaults are disabled centrally; only `synara-core-bindgen` enables
`cli` and `cargo-metadata`.

Shipping NSE's `nse-preview` graph excludes Core command owners, DTO/Platform
surfaces, timeline/sync/room-list owners and desktop notification delivery.
It retains shared client/store/lifecycle primitives, notification preferences,
agent classification and fail-closed error policy. QR, Markdown, widget and
MSC4426 SDK capabilities belong to `full-app`; encryption, encrypted-state
store compatibility and SQLite remain explicit in the common client graph.
Matrix SDK UI's NotificationClient still brings its upstream timeline
dependencies; separating those would require
an SDK API change rather than duplicating its notification implementation.

Compared with the previous audit commit, distinct package/version identities
on **normal and build** production edges fall from 551 to 526 for desktop
`aarch64-apple-darwin`, 643 to 619 for desktop `x86_64-unknown-linux-gnu`, and
352 to 346 for NSE `aarch64-apple-ios`. Full Core iOS retains its exact 351-package
set. These comparisons use the same lockfile and include build tooling; they
are dependency identities, not compiler invocations or measured speedups.
`npm run check:core-features` checks desktop, Apple and NSE shipping graph
feature sets, including forwarded Cargo features and target-specific build
dependencies.

Keep full Core, desktop and shipping NSE package builds separate. In particular,
NSE still requires its no-default-features Core edge, narrow exported ABI and
size-optimized `nse-release` profile. Combining them in a production build to
save compilation can enable capabilities that the extension must not ship.
Ordinary workspace tests are useful but do not prove the shipping graph.

Core and NSE ordinary builds emit only a Rust library. Apple generators use
`cargo rustc --lib --crate-type staticlib` explicitly with the existing release
or `nse-release` profile. This avoids building an unused Core dynamic library
and static archives during normal desktop and Rust test runs. A real minimal
Cargo fixture verifies both ordinary-library and explicit-staticlib modes;
Apple archive ABI and extension-size guards remain in the generation route.

Both Apple generators share the persistent host tool directory
`target/synara-core-bindgen`; `SYNARA_APPLE_BINDGEN_TARGET_DIR` overrides it.
Space-bounded mode deletes architecture intermediates while preserving this
host directory. Apple CI restores Kache compiler objects for both generators;
it does not archive these Cargo target directories.

Under the existing publication lock, both generators compare the entire staged
Swift/XCFramework pair with the existing pair: file bytes, directory entries,
modes and symlink targets. Identical pairs keep their inodes and timestamps;
any difference uses the existing atomic publication and rollback route.
Comparison failures abort before replacement, and NSE archive export checks
still run before publication. Generator fixtures cover identical reruns in
normal, bounded and overridden-cache modes; actual Xcode timing remains a
hosted/local Apple-toolchain measurement.

Desktop's build script preserves an unchanged release-hardening capability
file's modification time instead of rewriting a watched input. Its Git input
paths are resolved through Git, including linked worktrees and packed refs;
missing loose refs watch an existing parent until Git creates them. Five
filesystem/Git regression tests run directly with `rustc --test` in Rust CI
and exact-tag fallback validation, without compiling the application first.

### Cache families and writers

Rust compilation now uses **Kache 1.0.0** through the pinned shared
`.github/actions/setup-rust-cache` action. It caches compiler outputs, including
executables and supported C/C++ objects, instead of Cargo `target` directories.
`CARGO_INCREMENTAL=0` keeps compiler invocations cacheable. Cargo registry
archives/indexes and Git databases have a separate download cache. Main writers
fetch the complete locked workspace graph before publishing, so a narrow Apple
lane cannot freeze an incomplete download snapshot. Extracted
sources, build fingerprints and incremental state are excluded. The `cc` crate
is pinned to 1.2.66 so it recognizes the wrapper after selecting each target's
compiler. CMake launchers also preserve cross compiler selection.

Only designated main-branch writers publish snapshots; PRs and tags restore.
Keys include family, host OS/architecture, the exact Rust compiler, Apple
Xcode/SDK identity, lockfile and ISO week. Restore prefixes retain the same
family and toolchain. A weekly key keeps one snapshot per family instead of one
per commit (a per-commit key kept a 4 GiB copy per `main` push and evicted the
release families); Kache's content keys validate each invocation.
The upstream action's own immutable lockfile snapshots are disabled in favor
of repository-owned persistence. Its post step stops the private daemon before
the outer cache action saves the store.

The CI store is colocated with the checkout for linking/cloning into targets,
including Arch containers. Its configurable **5 GiB LRU eviction target** is
asynchronous, not a hard peak disk cap. Local stores use Kache's adaptive sizing
unless overridden. No S3 backend or automatic PR comments are enabled.

| Family                                 | Writer (on `main`)                                           | Readers                                                    |
| -------------------------------------- | ------------------------------------------------------------ | ---------------------------------------------------------- |
| `validate-rust-desktop`                | CI Rust tests                                                | Rust lint, PR Rust jobs and the Synapse live proofs        |
| `ci-synara-core-apple-simulator-arm64` | CI iOS unit lane, simulator-only without device opt-in       | UI and compile lanes                                       |
| `release-linux-deb`                    | Nightly Linux package build                                  | Tagged `.deb` release (the Arch package reuses its binary) |
| `release-macos`                        | Nightly universal macOS app build                            | Tagged universal macOS release                             |
| `release-synara-core-apple-device`     | Nightly iOS device cache seed                                | Tagged TestFlight device release                           |
| `xcode-compilation-v1`                 | CI iOS unit lane; weekly, at most 512 MiB                    | Simulator/UI/compile and TestFlight lanes                  |

GitHub lets every branch and tag restore caches saved on the default branch,
but a PR merge-ref cache does not warm sibling PRs or `main`, and one tag cannot
warm another tag. See
[GitHub cache scope rules](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#restrictions-for-accessing-a-cache).
The release families are therefore written by the nightly `main` run, which
never accesses signing secrets. After the package jobs, `prune-caches` deletes
every entry superseded by a newer one of the same family and ref
(`scripts/prune-actions-caches.mjs`), keeping the repository under its 10 GB
cache limit. Use `npm run audit:build-cache` for read-only sizes and refs, or
append `-- --json`.

Install the SHA256-verified local tool once with `npm run cache:setup`.
`npm run cargo -- <arguments>`, `npm run tauri -- <arguments>` and supported
Apple/macOS scripts configure the same persistent cache. Direct Cargo needs
`eval "$(scripts/setup-rust-cache.sh --env)"` in the current shell. No global
Cargo or shell settings are modified. Remove existing Rust workspace wrappers,
including any in Cargo configuration, before selecting the cache owner.
`npm run cache:status` shows statistics and tracked targets. `npm run cache:clean`
previews tracked targets stale for 14 days; append `-- --yes` to apply cleanup.
Automatic target cleanup is disabled. `SYNARA_RUST_CACHE_ROOT`,
`SYNARA_RUST_CACHE_TOOLS_DIR` and `SYNARA_RUST_CACHE_MAX_SIZE` override local
paths/size; explicitly supplied `KACHE_*` settings remain authoritative.
Local stores stay local; Swift and Xcode retain their native cache described below.

### Build measurements

Core's 73 integration-test targets are seven: four domain harnesses, the
isolated authorized live proof, feature-gated indexed search, and the small
UniFFI source transformation/registration target. A registration guard rejects
orphaned sources or missing harnesses. Shared CI test steps cap test threads at
four. Run a focused suite through its module filter, for example:
`cargo test -p synara-core --test sdk_behaviors offline_timeline_cold_restart::`.

Node installation jobs share a content-verified npm download cache keyed by
both lockfiles, OS and architecture. Only main writes; PRs and tags restore.
Installed `node_modules` directories are never cached.

Rust dependency auditing installs the pinned `cargo-audit` 0.22.2 binary using
a pinned [installer with embedded release checksums](https://github.com/taiki-e/install-action).

The 2026-10-05 Kache pilot and benchmark measurements are recorded in the
[local pilot](reviews/2026-10-05-kache-local-pilot.json),
[hosted benchmark](reviews/2026-10-05-kache-hosted-benchmark.json),
[production benchmark](reviews/2026-10-05-kache-production-benchmark.json),
[native correctness](reviews/2026-10-05-kache-1-native-correctness.json),
[native reuse](reviews/2026-10-05-kache-native-reuse.json) and
[Apple archive reuse](reviews/2026-10-05-kache-apple-archive-reuse.json)
reports. Measured on hosted runners: with warm caches the v2.1.45 release took
20 minutes and its universal macOS build 10.5 minutes; cold, v2.1.46 took 58
minutes, 53 of them compiling the universal macOS app.

Core now separates common notification/store primitives, full application
services and Apple bindings with explicit additive Cargo features. Validate
shipping package graphs separately: workspace tests can intentionally unify
features, while a narrow NSE host check verifies that its shared primitives
compile without full application owners. Apple release gates still inspect the
actual archive ABI and extension size; graph counts alone cannot replace those
checks or predict archive-size reductions.

### Swift and Xcode build reuse

The reachable Swift package graph is currently entirely local: `SynaraCore`
and `SynaraNseCore` wrap the generated Rust archives. No remote SwiftPM download
cache or synthetic `Package.resolved` is needed. All supported project build
scripts inspect this graph; if remote dependencies are added, a reviewed lock
becomes required and automatic resolution/package updates are disabled.

Local command-line builds now keep persistent caches under
`~/Library/Caches/Synara/Xcode/<checkout identity>/<toolchain identity>`.
The identity includes the selected compiler, Xcode build, Swift version, iOS
and simulator SDK versions and architecture. Separate DerivedData lanes cover
simulator, UI, unsigned device and signed archive builds. Explicit path
overrides remain available. XcodeGen uses its native spec/source-list cache
plus checks of generated outputs, so unchanged projects, schemes and plists
retain timestamps; changed or missing outputs force regeneration. Generated
Swift/XCFramework pairs already preserve identical outputs.

[Apple compilation caching](https://developer.apple.com/documentation/xcode-release-notes/xcode-26-release-notes)
is enabled for Xcode 26 and newer in local build scripts and the checked-in
project, including direct IDE builds. CLI builds use a dedicated local-only
compiler CAS directory shared by their build lanes. IDE builds use Xcode's
normal cache location. Ordinary incremental DerivedData reuse remains useful;
the native cache can also reuse compilation results after those intermediates
are gone. Custom compiler prefix mapping is not introduced.

CI restores **only compiler CAS objects**, using the exact selected toolchain
identity with no fallback across toolchains. The native size limit is 256M per
database, with an independent **512 MiB aggregate publication gate**. Oversized,
empty and already-published weekly snapshots are skipped. Only successful main
iOS unit builds save; PRs and release lanes read. Weekly immutable
keys reduce cache churn; missing objects compile normally. DerivedData,
SourcePackages, signing credentials, result bundles, archives and products are
excluded. Inspect `npm run audit:build-cache` and the workflow cache summaries
before increasing budgets or generations, especially with the current 10 GB
repository limit. Toolchain changes can temporarily add extra snapshots.

Apple CI jobs select **Xcode 26.6 / 17F113** using
`DEVELOPER_DIR=/Applications/Xcode_26.6.app/Contents/Developer`, a version
available in the [macOS 26 runner image](https://github.com/actions/runner-images/blob/main/images/macos/macos-26-arm64-Readme.md).
The selected version is checked before Swift cache work; the nightly device
cache seed checks it directly. Apple Rust cache identities include
`DEVELOPER_DIR`, Xcode build, macOS/iOS/simulator SDK versions and Rust compiler,
so SDK changes select a different compatible snapshot family. When
upgrading Xcode, update all Apple job paths and the version assertions together,
then validate the simulator/device lanes. Local tools remain
free to select another installed Xcode; their own cache identity isolates it.

A disposable real Xcode 27.0 / Swift 6.4 proof on 2026-10-05 built a small tool
with a local Swift package, resource-generated accessor and intentional warning.
The first build took **10.78s with 0/74 cache hits**; after removing only the
fixture's DerivedData, the repeat took **3.69s with 74/74 hits**. Warning replay
and executable output were preserved; compiler objects occupied approximately
119 MiB. This proves the configured mechanism on this toolchain, not Synara
application build acceleration. Full app and hosted comparisons still require
real builds, including restore/save costs. CI emits native cache diagnostic
remarks and build timing summaries to support those measurements.

## Local Validation Gates

Run these before accepting desktop/runtime changes:

```bash
npm run check:repo-layout
npm run check:versions
npm run check:docs
npm run check:matrix-boundaries
npm run check:workflows   # requires actionlint on PATH
npm run check:synapse-harness
npm --prefix synara run typecheck:modernization
npm run test:modernization
npm --prefix synara run check:eslint
npm --prefix synara run check:prettier
cargo clippy --locked -p synara --all-targets -- -D warnings
cargo test --locked -p synara
cargo clippy --locked -p synara-core -p synara-nse-core -p synara-core-bindgen --all-targets -- -D warnings
cargo clippy --locked -p synara-core --features search-index,x509-identity --all-targets -- -D warnings
cargo test --locked -p synara-core -p synara-nse-core -p synara-core-bindgen
cargo test --locked -p synara-core --features search-index,x509-identity
npm run check:production-smoke
```

When Docker is available, run the real cross-device Synapse regression gate as
well. The final reset is destructive only to the generated loopback harness:

```bash
scripts/synapse-integration.sh up
npm run check:synapse-harness
scripts/synapse-integration.sh reset
```

The root modernization command includes its runtime build pretest. Shared-package
and desktop tests select their packages explicitly; shipping NSE feature and
archive isolation remains a separate check.

For timeline work, run both the model harness and the native-timeline browser
harness:

```bash
npm --prefix synara run test:timeline-performance
npm --prefix synara run test:browser:native-timeline
```

The browser native-timeline harness uses one file worker to isolate its performance
measurement from concurrent functional files. CI runs it with one retry because
the Hermes frame-timing assertion is sensitive to shared-runner load.

For renderer dependency and platform-runtime changes, run the release browser
entrypoint, which includes runtime-maturity in Chromium and WebKit:

```bash
npm --prefix synara run test:browser:desktop-polish
```

Chromium-only CI coverage does not establish WebKit coverage. Current browser
fixtures do not certify physical minimum-system WebKit or installed native URL
transport.

## Local Builds

Development shell:

```bash
npm run tauri dev
```

Linux package smoke:

```bash
npm run tauri build -- --bundles deb --config '{"bundle":{"createUpdaterArtifacts":false}}'
```

macOS unsigned local smoke:

```bash
npm run tauri build -- --bundles app --config '{"bundle":{"macOS":{"entitlements":"Entitlements.adhoc.plist"}}}'
```

The ad-hoc smoke lane deliberately claims no restricted notification entitlement.
It verifies package construction and UI behavior, not Time Sensitive or Critical
delivery. Shipping Developer ID bundles retain the Time Sensitive entitlement
and must embed the matching provisioning profile before signing.

macOS workstation tasks requiring `xcodebuild`, Swift, simulator execution, or
full app launch smoke are tracked in
[iOS validation status](../synara-ios/docs/ios-validation-status.md) and
[desktop validation status](desktop-validation-status.md).

## Release PR Flow

1. Create `release/vX.Y.Z` from `main` and bump all client versions and the iOS
   build number with `npm run bump:version -- X.Y.Z --ios-build X.Y.Z`.
2. Add the changelog, `docs/releases/vX.Y.Z.md`, and
   `synara-ios/release-notes/vX.Y.Z-en-US.txt`.
3. Open the release PR into `main`. Require its `CI / Quality gate`, including
   the iOS simulator unit suite, to pass. Merge when green. XCUITests run
   nightly; add `needs-ios-ui` before pushing to include them in the PR.
4. Push `vX.Y.Z` at the merged `main` commit. That tag starts the Release
   workflow. Do not wait for another full CI run on the merge commit.

When a change needs interactive candidate testing, dispatch `CI` on the branch
with `packages: true`, or download the artifacts of the latest nightly run. The
package jobs produce disposable smoke artifacts:

- `synara-macos-app`: unsigned/ad-hoc macOS `.app` release-candidate smoke artifact.
- `synara-linux-arch-pkg`: Arch/CachyOS pacman package artifact for
  `pacman -U` smoke and GitHub Release-backed pacman repo validation.
- `synara-linux-deb`: Debian-family package smoke artifact.

Record any interactive results in
[production-smoke-checklist.md](production-smoke-checklist.md). The tag builds
the signed and publishable packages once, from the exact release commit.

To rehearse the signed macOS build without publishing, run
`scripts/build-macos-local-release.sh` on a Mac with the signing identity.

## Production Publish Flow

Production publication is owned by the singular `Release` workflow.
It is deliberately tag-push-only: do not add `workflow_dispatch` unless it
requires an explicit tag and checks out that exact tag SHA. GitHub's normal
manual workflow branch selector is not a safe release-source selector.

1. The `Release` workflow validates that the tag matches the committed shared
   version and is reachable from `main`. It then requires a successful CI
   `Quality gate` on that SHA or on the incoming PR parent of the merge commit
   (`scripts/reuse-proven-quality-gate.mjs --require`), waiting up to 45
   minutes for a gate that is still running. It does not re-run the suite at
   the tag; without a proven gate the release fails. Desktop packaging starts
   in parallel and publication waits for the gate:
   - macOS signed/notarized DMG, macOS updater archive, signatures, and
     `latest.json`.
   - Linux `.deb` plus fixed `apt-repo` release assets (`Packages`,
     `Packages.gz`, `Release`, and the package).
   - Arch-family `synara-desktop-bin` package, which repackages the `.deb`
     job's binary after checking its shared libraries resolve on Arch, plus
     fixed `pacman-repo` release assets (`synara.db`, `synara.files`, and
     package file).

   The macOS build retries bundling, signing and notarization up to three
   times (a retry reuses the compiled target, so it costs minutes), and DMG
   notarization and stapling retry the same way.
2. GitHub Release publishes those desktop artifacts through the
   `production-release` environment without a second human approval. It does
   not wait on TestFlight. The environment still scopes signing secrets.
3. iOS TestFlight upload and internal promotion run in parallel as their own
   track. Confirm the TestFlight state snapshot; Apple should report the exact
   build as `IN_BETA_TESTING`.

If an exact build uploads successfully but only its TestFlight promotion job
fails, repair the cause on `main` and use **TestFlight Promotion Recovery** with
the existing release tag and exact uploaded build number. This recovery checks
that the tag is on `main`, reads the release notes from `main`, and promotes the
already uploaded build. It does not rebuild or republish desktop assets.

After publication, confirm hosted macOS `latest.json` and verify the fixed
Linux repository URLs:

```text
https://github.com/nepenth/synara-desktop/releases/download/pacman-repo/synara.db
https://github.com/nepenth/synara-desktop/releases/download/apt-repo/Packages
```

For periodic or higher-risk releases, also smoke installed-app update behavior:

- iOS updates through TestFlight.
- macOS updates through the Tauri updater flow.
- Linux updates through `sudo apt upgrade`, `paru -Syu`, or
  `sudo pacman -Syu`; the app may only notify/instruct.

Updater secrets, endpoint names, and publication rules live in this runbook.
Release-branch PRs into `main` from `release/vX.Y.Z` run Quality gate including
the iOS simulator unit suite. Ordinary feature PRs skip the simulator suites.

## Required Release Secrets

macOS releases require Apple Developer ID and notarization secrets consumed by
the protected release workflow. The expected variable names and validation
rules are listed below; values must remain in GitHub Secrets or
permission-restricted local storage.

macOS signing also requires `MACOS_PROVISIONING_PROFILE_BASE64`, containing a
current `MAC_APP_DIRECT` profile for `com.whylandcreative.synara.desktop` with
Time Sensitive Notifications enabled and the same Developer ID Application
certificate used to sign the app. Store it at the same secret scope as the other
macOS signing secrets. The signed release workflow
validates its team, application identifier, platform, expiry, capability and
certificate before building. It generates identifier-bound signing entitlements,
embed `Contents/embedded.provisionprofile`, and inspect the final app signature.
For local signed builds set `SYNARA_MACOS_PROVISIONING_PROFILE` to that profile's
absolute path in the local signing environment. Enabling this standard capability
does not enable Critical Alerts or notification filtering; those require their
separate Apple approvals and explicit signed configurations.

Updater-enabled releases require:

- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
- `SYNARA_UPDATER_PUBKEY`
- `SYNARA_UPDATER_ENDPOINT`

Signed APT repository publication additionally requires these GitHub Actions
secrets, preferably scoped to the protected `production-release` environment:

- `SYNARA_APT_SIGNING_PRIVATE_KEY`: ASCII-armored export of the dedicated
  repository signing private key.
- `SYNARA_APT_SIGNING_PRIVATE_KEY_PASSWORD`: the private-key passphrase.

Create and retain the production key outside the repository, store its recovery
copy in an approved password manager or offline encrypted storage, and publish
only the exported binary public keyring. Record its full fingerprint in the
release operations record so rotations can be independently verified.

Current production APT signing-key fingerprint (expires 2028-08-24):

```text
EB88 3952 04C1 EE19 7EE8  3B2F 3E02 F509 BB6B 0D2B
```

Never commit updater private keys, Apple certificates, passwords, or notarization
credentials.

Keep the GitHub `production-release` Environment for signing secrets and
deployment history. A second human reviewer approval is not required for a
single-maintainer repository: pushing the validated version tag is the release
authorization.

Set the repository variable `SYNARA_TESTFLIGHT_INTERNAL_ONLY` to `true` or
`false` to control internal-only TestFlight distribution for subsequent tag
pushes. It defaults to `true`; there is no manual-dispatch override.

Do not configure the `production-release` environment with required status checks
from ordinary CI workflows that do not run on tag refs: those checks cannot
report against a release-tag deployment and will leave publication blocked.
The Release workflow's proven-Quality-gate requirement protects publication.
Branch-protection status checks remain appropriate for `main` and release
branches where their workflows actually run.

If a release job fails with `incorrect updater private key password`, rotate the
Tauri updater keypair and GitHub secrets `TAURI_SIGNING_PRIVATE_KEY`,
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, and repository variable
`SYNARA_UPDATER_PUBKEY` together.

## Linux Pacman Repo

The production pacman repo is a public GitHub Release-backed repository:

```ini
[synara]
SigLevel = Optional TrustAll
Server = https://github.com/nepenth/synara-desktop/releases/download/pacman-repo
```

Release CI must own every production repo mutation:

1. Build the Arch package in an Arch container.
2. Run `scripts/build-pacman-repo.sh`.
3. Upload the package to the versioned release.
4. Create the fixed `pacman-repo` release with `--latest=false` if needed.
5. Delete old fixed-repo database/package assets.
6. Upload the new fixed-repo database/package assets.

Maintainers and agents should not manually run `repo-add` for production
publication. Manual commands are acceptable only for local smoke packages.

## Linux APT Repo

The Debian-family repository is a flat signed public repository backed by
the fixed `apt-repo` GitHub Release:

```text
deb [arch=amd64 signed-by=/etc/apt/keyrings/synara-archive-keyring.gpg] https://github.com/nepenth/synara-desktop/releases/download/ apt-repo/
```

Release CI builds the `.deb`, runs `scripts/build-apt-repo.sh`, uploads the new
package, imports the private key only inside the protected publication job,
generates and verifies `InRelease` and `Release.gpg`, publishes the public
keyring, and then removes obsolete package assets. Do not manually rebuild or
sign production metadata.

GitHub Release asset replacement is not transactional. The supported bootstrap
publisher minimizes the inconsistent window and keeps the prior package until
the new signed metadata is live, but the production-hardening target is an
atomically deployed static repository.

## Release Constraints

- Human macOS and Linux package-install smoke is optional routine coverage;
  use it when packaging or platform behavior changed in a way CI cannot prove.
- Physical-device iOS upgrade, performance, APNs, and archive evidence is
  recommended for relevant changes and can be collected after publication.
- macOS uses signed Tauri updater metadata; Linux uses the GitHub
  Release-backed pacman repository; iOS uses TestFlight or the App Store.
- Production publication is blocked unless the exact-tag workflow validates all
  configured clients and protected credentials.

Apple artifact symbol checks require `rustup component add llvm-tools-preview
--toolchain 1.96` before generating bindings. Apple CI installs this component
explicitly in unit, UI, compile, diagnostics, and device release lanes. The NSE
generator validates its reader before building and checks every completed
XCFramework archive architecture before publication. Each architecture must
expose `_uniffi_synara_nse_core_fn_method_nsepreviewrequest_resolve` and contain
no full-Core `uniffi_synara_core_` exports. The decoder is the selected Rust
sysroot's LLVM tool, with an exact LLVM version check; missing tools, decoder
errors, empty archive symbol output, or missing positive exports fail closed.
The explicit `SYNARA_NSE_ARCHIVE_NM` override exists for controlled fixtures
and is subject to the same version/readback checks.

The shipping extension checker decodes every final MachO architecture and
checks linked images. Release stripping may remove final symbol-table entries;
its report records unavailable final readback rather than treating empty output
as standalone isolation proof. The mandatory positive and negative archive ABI
readback is primary. Neither checker infers export absence from raw bytes.
[LLVM's symbol tool documentation](https://llvm.org/docs/CommandGuide/llvm-nm.html)
describes bitcode/object decoding and architecture selection; the
[Rust component documentation](https://rust-lang.github.io/rustup/concepts/components.html)
identifies the matching toolchain component.
