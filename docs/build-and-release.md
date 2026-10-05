# Build And Release Runbook

Reviewed: 2026-09-26

This is the entry point for agents and maintainers preparing Synara builds or
releases. Read this before changing packaging, signing, updater, TestFlight, or
release workflow behavior.

## Release Lanes

| Lane                | Purpose                                                                                              |         Client-visible update? |
| ------------------- | ---------------------------------------------------------------------------------------------------- | -----------------------------: |
| `main`              | Integration branch. Runs normal CI on push and PR.                                                   |                             No |
| `release/vX.Y.Z`    | Version and notes PR. Runs the full client Quality gate, including iOS simulator unit and UI suites. |                             No |
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
on ordinary feature PRs and the applicable desktop package checks. Main pushes run Apple unit tests for shared-Core/iOS changes and keep the UI
lane disabled. Releases, `needs-ios-ui` labels, nightly schedules, and manual
full runs retain their fuller test policy.

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
host directory. Apple CI caches both it and `target/synara-core-apple`.

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

The pinned `Swatinem/rust-cache` remains the baseline. Equivalent jobs share a
family; readers cannot replace it with their narrower dependency subset.
Compiled target caches are kept separate for Ubuntu, Arch, native macOS,
universal macOS and Apple target/profile graphs. Registry-only families do not
store compiled objects and keep the first rollout within the storage budget.

| Family                                 | Writer                                                       | Readers                                                           |
| -------------------------------------- | ------------------------------------------------------------ | ----------------------------------------------------------------- |
| `validate-rust-desktop`                | CI Rust validation on `main`                                 | PR validation, exact-tag validation and six native Synapse proofs |
| `ci-synara-core-apple-simulator-arm64` | CI unit lane on `main`, simulator-only without device opt-in | UI, compile, diagnostics and exact-tag simulator lanes            |
| `release-linux-deb`                    | Desktop Package Smoke dispatched on `main`                   | PR `.deb` smoke and tagged `.deb` release                         |
| `release-macos`                        | Unsigned macOS seed or intended signed build on `main`       | Tagged universal macOS release                                    |
| `desktop-registry-arch`                | Desktop Package Smoke dispatched on `main`                   | Arch smoke and release; registry only                             |
| `desktop-registry-macos-host`          | Desktop Package Smoke dispatched on `main`                   | Native macOS smoke; registry only                                 |
| `release-synara-core-apple-device`     | Unsigned device seed on `main`                               | Tagged TestFlight device release                                  |

GitHub can restore current/default/base-branch caches, but a PR merge-ref cache
does not warm sibling PRs or `main`, and one tag cannot warm another tag.
See [GitHub cache scope rules](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#restrictions-for-accessing-a-cache).
Consequently the Linux/universal release families need a successful main seed.
After these changes land, an authorized maintainer can explicitly dispatch
Desktop Package Smoke on `main` to seed Ubuntu and registry families. The manual **Seed unsigned release build caches** workflow independently
builds universal macOS executables (`--no-bundle`) or generates and validates
Core/NSE device archives. Neither lane accesses signing secrets or publishes
client releases. Only `main` saves those cache families; branch runs and tagged
releases read them. The existing signed macOS lane may also seed during an
intended candidate build. No warming schedule or storage-limit increase is
introduced.

Adding the isolated host bindgen directory changes the Apple cache archive
paths/version. The first new main simulator seed may therefore be cold, even
when the inventory lists an older entry with the same family. Inventory entries
are candidates; the job's restore log establishes an actual compatible hit.

Use `npm run audit:build-cache` for a read-only inventory, or append `-- --json`
for machine-readable sizes, refs, entries and missing main seeds. The audit
snapshot had approximately 8.52 GB against the configured 10 GB maximum and
7-day retention. Inspect storage before adding compiled Arch/native-macOS
families or additional generations. Old PR/tag caches can expire naturally;
do not use blanket cleanup that deletes active main seeds. GitHub permits a
paid storage-limit increase, but that is a separate operational decision.

### Build measurements and cache rollout

Recent successful [desktop smoke](https://github.com/nepenth/synara-desktop/actions/runs/37263529083)
build steps took 20m42 (`.deb`), 21m53 (Arch) and 23m03 (macOS), without Rust
caching. A [cached Rust validation run](https://github.com/nepenth/synara-desktop/actions/runs/37263625878)
spent 61s restoring and 68s saving its cache. Its shared test step included
about 4m50 compiling followed by roughly ten minutes executing successive
integration-test binaries. Cache work does not remove that execution time;
the tests remain enabled. Six native proof jobs also compiled the same desktop
test dependency graph without a cache, then ran brief individual proofs.

Core's 73 integration-test targets are now seven: four domain harnesses,
the isolated self-reexecuting authorized live proof, feature-gated indexed
search, and the small UniFFI source transformation/registration target. Original
sources and fixture paths remain in place. All 286 original test declarations
are retained; an added registration guard rejects orphaned sources or missing
harnesses. With four test threads, default suites passed 281 tests with the
existing live proof ignored; indexed search passed five. Initial linking took
2m39 locally, default execution 112s, and seven executables totaled 902 MiB.
These measurements use debug=0/jobs=2 on this host and are not a comparable
hosted speedup claim. Shared CI test steps explicitly cap test threads at four.
The desktop crate's SharedCore queue-teardown proof now lives in Core's
lifecycle harness and passed there with every assertion preserved; desktop
tests no longer need Apple bindings. The registration guard passes with the
added source. Final Core unit tests passed 1,117 cases, retaining four existing
ignored cases, and both desktop and shared-workspace strict Clippy checks pass.

Run a focused suite through its module filter, for example:
`cargo test -p synara-core --test sdk_behaviors offline_timeline_cold_restart::`.

Node installation jobs share a content-verified npm download cache keyed by
both lockfiles, OS and architecture. Only main writes; PRs and tags restore.
Installed `node_modules` directories are never cached. Arch packaging now uses
`.node-version` rather than whichever Node major pacman currently provides.
Pure Node checks that install no packages do not restore this cache.

Rust dependency auditing installs the pinned `cargo-audit` 0.22.2 binary using
a pinned [installer with embedded release checksums](https://github.com/taiki-e/install-action).
Source-build fallbacks are disabled, avoiding repeated compilation of the audit
tool. Exact-tag fallback validation also runs the audit when it cannot reuse
successful CI evidence; the required Quality gate includes the CI audit result.

Renderer assets use project-owned Vite hooks instead of a generic glob/copy
plugin. PDF worker bytes, config and locale URLs are unchanged, verified with
actual dev servers (root and nested bases), production build fixtures and the
normal runtime output guard. Removing the copy plugin eliminates 15 packages
and the remaining development advisory chain; both full npm audits are clean.

These timings are the old baseline. Local fixtures and dependency graph checks
establish correctness, not hosted speedup. Compare seeded/warm whole-job times
at the same commit and toolchain, including restore/save costs and cache bytes,
before claiming improvement or changing cache backends.

The manual **Rust Cache Benchmark** workflow installs a pinned
[Kache action](https://github.com/kunobi-ninja/kache-action) and executable
0.28.1. It compares two fresh host NSE builds without a wrapper, then a cold
and warm Kache build, removing only its private Cargo target between stages.
The store is local-only and capped at 512 MiB; GitHub persistence and automatic
PR comments are disabled. Source fingerprints reject changes during the
experiment, disk limits stop oversized runs, and interruption/failure cleanup
removes owned scratch. JSON and job summaries include timings, hit/miss deltas,
logical bytes, commit and toolchain. This experiment measures compiler reuse;
it excludes Swatinem/GitHub restore/save, downloads, source edits, Apple/Xcode
and full desktop builds, and a small store may evict useful entries.

The pinned Swatinem backend remains production policy pending comparable
whole-job measurements. Kache's GitHub-backed store also inherits branch
isolation and immutable snapshots. Any S3-backed trial needs its own trusted
writers, retention and storage design.

Core now separates common notification/store primitives, full application
services and Apple bindings with explicit additive Cargo features. Validate
shipping package graphs separately: workspace tests can intentionally unify
features, while a narrow NSE host check verifies that its shared primitives
compile without full application owners. Apple release gates still inspect the
actual archive ABI and extension size; graph counts alone cannot replace those
checks or predict archive-size reductions.

## Local Validation Gates

Run these before accepting desktop/runtime changes:

```bash
npm run check:repo-layout
npm run check:versions
npm run check:docs
npm run check:matrix-boundaries
npm run check:quality-gates
npm run check:synapse-harness
npm --prefix synara run typecheck:modernization
npm run test:modernization
npm --prefix synara run check:eslint
npm --prefix synara run check:prettier
cargo clippy --locked -p synara --all-targets -- -D warnings
cargo check --locked -p synara
cargo test --locked -p synara
cargo clippy --locked -p synara-core -p synara-nse-core -p synara-core-bindgen --all-targets -- -D warnings
cargo check --locked -p synara-core -p synara-nse-core -p synara-core-bindgen
cargo test --locked -p synara-core -p synara-nse-core -p synara-core-bindgen
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
measurement from concurrent functional files. Its cases, budgets, sample windows,
assertions, and retry policy remain unchanged.

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
   the iOS simulator unit and UI suites, to pass. Merge when green.
4. Push `vX.Y.Z` at the merged `main` commit. That tag starts the Release
   workflow. Do not wait for another full CI run on the merge commit.

When a change needs interactive candidate testing, add the `needs-package` PR
label. It builds disposable smoke artifacts:

- `synara-macos-app`: unsigned/ad-hoc macOS `.app` release-candidate smoke artifact.
- `synara-linux-arch-pkg`: Arch/CachyOS pacman package artifact for
  `pacman -U` smoke and GitHub Release-backed pacman repo validation.
- `synara-linux-deb`: Debian-family package smoke artifact.

Record any interactive results in
[production-smoke-checklist.md](production-smoke-checklist.md). Release PRs do
not build disposable desktop packages by default; the tag builds the signed
and publishable packages once, from the exact release commit.

## Production Publish Flow

Production publication is owned by the singular `Release` workflow.
It is deliberately tag-push-only: do not add `workflow_dispatch` unless it
requires an explicit tag and checks out that exact tag SHA. GitHub's normal
manual workflow branch selector is not a safe release-source selector.

1. The `Release` workflow validates that the tag matches the committed shared
   version and is reachable from `main`. Exact-tag jobs reuse a proven
   `Quality gate` on that SHA (or the incoming PR parent of a merge commit)
   and otherwise rerun full desktop/runtime and iOS simulator tests at the
   tagged SHA. After that gate, desktop packaging starts immediately:
   - macOS signed/notarized DMG, macOS updater archive, signatures, and
     `latest.json`.
   - Linux `.deb` plus fixed `apt-repo` release assets (`Packages`,
     `Packages.gz`, `Release`, and the package).
   - Arch-family `synara-desktop-bin` package plus fixed `pacman-repo` release
     assets (`synara.db`, `synara.files`, and package file).
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
already uploaded build. It does not rebuild or republish desktop assets. 4. Confirm hosted macOS `latest.json`. 5. Verify the fixed Linux repository URLs:

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
iOS simulator unit and UI suites. Ordinary feature PRs skip those iOS suites.
Desktop Package Smoke remains available on release PRs through the
`needs-package` label or manual dispatch.

## Required Release Secrets

macOS releases require Apple Developer ID and notarization secrets consumed by
the protected release workflow. The expected variable names and validation
rules are listed below; values must remain in GitHub Secrets or
permission-restricted local storage.

macOS signing also requires `MACOS_PROVISIONING_PROFILE_BASE64`, containing a
current `MAC_APP_DIRECT` profile for `com.whylandcreative.synara.desktop` with
Time Sensitive Notifications enabled and the same Developer ID Application
certificate used to sign the app. Store it at the same secret scope as the other
macOS signing secrets. The signed release and manual signed-build workflows
validate its team, application identifier, platform, expiry, capability and
certificate before building. They generate identifier-bound signing entitlements,
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
The Release workflow's exact-tag validation jobs protect publication.
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
