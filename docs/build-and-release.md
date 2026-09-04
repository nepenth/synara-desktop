# Build And Release Runbook

Reviewed: 2026-08-18

This is the entry point for agents and maintainers preparing Synara builds or
releases. Read this before changing packaging, signing, updater, TestFlight, or
release workflow behavior.

## Release Lanes

| Lane                | Purpose                                                              |         Client-visible update? |
| ------------------- | -------------------------------------------------------------------- | -----------------------------: |
| `main`              | Integration branch. Runs normal CI on push and PR.                   |                             No |
| `release/vX.Y.Z`    | Release candidate branch. Runs CI and desktop package smoke on push. |                             No |
| Pushed tag `vX.Y.Z` | Coordinated macOS, Linux, and internal TestFlight release.           | Yes, after every client passes |

Do not push a release tag until the branch `Quality gate`, desktop package smoke,
and human smoke checklist have passed.

## Local Prerequisites

1. Install Node from `.node-version`.
2. Install Rust 1.93 (repo root `rust-toolchain.toml` pins the channel).
3. Install Tauri platform prerequisites.
4. Run dependency installation from both package roots:

```bash
npm ci
npm --prefix synara ci
```

Linux system package details live in [linux.md](linux.md). macOS local signing
and app replacement notes live in [macos-local-signing.md](macos-local-signing.md).

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
npm --prefix synara run test:modernization
npm --prefix synara run check:eslint
npm --prefix synara run check:prettier
cargo check --manifest-path src-tauri/Cargo.toml --locked
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run check:production-smoke
```

When Docker is available, run the real cross-device Synapse regression gate as
well. The final reset is destructive only to the generated loopback harness:

```bash
scripts/synapse-integration.sh up
npm run test:synapse-integration
scripts/synapse-integration.sh reset
```

For timeline work, also run:

```bash
npm --prefix synara run test:timeline-performance
```

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
npm run tauri build -- --bundles app
```

macOS workstation tasks requiring `xcodebuild`, Swift, simulator execution, or
full app launch smoke are tracked in [../MACOS_WORKSTATION_HANDOFF.md](../MACOS_WORKSTATION_HANDOFF.md).

## Release Branch Flow

1. Bump version metadata with `npm run bump:version -- X.Y.Z`.
2. Confirm `npm run check:versions`.
3. Create `release/vX.Y.Z` from `main`.
4. Push the release branch.
5. Confirm GitHub Actions:
   - `CI / Quality gate`, including real iOS simulator tests
   - `Desktop Package Smoke`
6. Install and smoke the generated package artifacts:
   - `synara-macos-app`: unsigned/ad-hoc macOS `.app` release-candidate smoke artifact.
   - `synara-linux-arch-pkg`: Arch/CachyOS pacman package artifact for
     `pacman -U` smoke and GitHub Release-backed pacman repo validation.
   - `synara-linux-deb`: Debian-family package smoke artifact.
7. Record smoke evidence in [production-smoke-checklist.md](production-smoke-checklist.md).

Release branch pushes build candidate artifacts for validation. They must not
publish a production updater channel.

## Production Publish Flow

Production publication is owned by the singular `Release` workflow.
It is deliberately tag-push-only: do not add `workflow_dispatch` unless it
requires an explicit tag and checks out that exact tag SHA. GitHub's normal
manual workflow branch selector is not a safe release-source selector.

1. Bump every client and the iOS build number together:

```bash
npm run bump:version -- X.Y.Z --ios-build X.Y.Z
```

2. Open a release PR (version files, changelog, release notes). Version-bearing
   manifests and configuration are executable build inputs and run the relevant
   validation. Use a `release/vX.Y.Z` head branch to require the iOS unit and UI
   suites as well. Only inert release prose qualifies for metadata-only skips.
3. Merge when Quality gate is green, then tag `vX.Y.Z` at that `main` commit.
   Do not wait for a second full suite on the merge push.
4. The `Release` workflow validates that the tag matches the committed shared
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
5. GitHub Release publishes those desktop artifacts through the
   `production-release` environment. It does not wait on TestFlight.
6. iOS TestFlight upload and internal promotion run in parallel as their own
   track. Confirm the TestFlight state snapshot; Apple should report the exact
   build as `IN_BETA_TESTING`.
7. Confirm hosted macOS `latest.json`.
8. Verify the fixed Linux repository URLs:

```text
https://github.com/nepenth/synara-desktop/releases/download/pacman-repo/synara.db
https://github.com/nepenth/synara-desktop/releases/download/apt-repo/Packages
```

9. Confirm installed-app update behavior:
   - iOS updates through TestFlight.
   - macOS updates through the Tauri updater flow.
   - Linux updates through `sudo apt upgrade`, `paru -Syu`, or
     `sudo pacman -Syu`; the app may only notify/instruct.

The updater implementation plan and required GitHub variables/secrets live in
[../GITHUB_RELEASE_UPDATER_PLAN.md](../GITHUB_RELEASE_UPDATER_PLAN.md). The
release-branch CI strategy lives in [../RELEASE_BRANCH_CI_PLAN.md](../RELEASE_BRANCH_CI_PLAN.md).

## Required Release Secrets

macOS releases require Apple Developer ID and notarization secrets consumed by
the protected release workflow. The expected variable names and validation
rules are documented in
[the GitHub release updater plan](../GITHUB_RELEASE_UPDATER_PLAN.md); values
must remain in GitHub Secrets or permission-restricted local storage.

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

Configure the GitHub `production-release` Environment with at least one required
human reviewer. The workflow declares the environment, but repository-level
review protection must be enabled in GitHub settings.

Set the repository variable `SYNARA_TESTFLIGHT_INTERNAL_ONLY` to `true` or
`false` to control internal-only TestFlight distribution for subsequent tag
pushes. It defaults to `true`; there is no manual-dispatch override.

Do not configure the `production-release` environment with required status checks
from ordinary CI workflows that do not run on tag refs: those checks cannot
report against a release-tag deployment and will leave approval permanently
blocked. Use required human reviewers for the environment and the Release
workflow's exact-tag validation jobs for automated publication protection.
Branch-protection status checks remain appropriate for `main` and release
branches where their workflows actually run.

If a release job fails with `incorrect updater private key password`, rotate the
Tauri updater keypair and GitHub secrets together. The full command sequence is
tracked in [../GITHUB_RELEASE_UPDATER_PLAN.md](../GITHUB_RELEASE_UPDATER_PLAN.md#rotate-updater-signing-key-material).

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

- Human macOS and Linux package-install smoke remains required for each release
  candidate even when automated build and unit gates pass.
- Physical-device iOS upgrade, performance, APNs, and archive evidence remains
  a release-candidate responsibility.
- macOS uses signed Tauri updater metadata; Linux uses the GitHub
  Release-backed pacman repository; iOS uses TestFlight or the App Store.
- Production publication is blocked unless the exact-tag workflow validates all
  configured clients and protected credentials.
