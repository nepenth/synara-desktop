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

2. Commit and push the version metadata to `main`.
3. Wait for the normal `CI / Quality gate` check to pass.
4. Create and push `vX.Y.Z` at that exact `main` commit.
5. The `Release` workflow validates that the tag matches the committed shared
   version and is reachable from `main`. Exact-tag jobs reuse a proven
   `Quality gate` on that SHA (or the incoming PR parent of a merge commit)
   and otherwise rerun full desktop/runtime and iOS simulator tests at the
   tagged SHA. After that gate, the workflow builds:
   - macOS signed/notarized DMG, macOS updater archive, signatures, and
     `latest.json`.
   - Linux `.deb`.
   - Arch-family `synara-desktop-bin` package plus fixed `pacman-repo` release
     assets (`synara.db`, `synara.files`, and package file).
   - iOS signed archive uploaded to internal TestFlight, followed by an
     App Store Connect gate that requires the exact build to be valid and in
     beta testing for every configured internal group.
6. Only after every build and verification passes, the workflow creates the
   versioned GitHub Release and updates the fixed pacman repository through the
   `production-release` environment approval.
7. Confirm the workflow's TestFlight state snapshot and hosted macOS
   `latest.json`. Manual App Store Connect inspection is confirmatory; the
   workflow already fails unless Apple reports the exact build as
   `IN_BETA_TESTING`.
8. Verify the fixed pacman repo URL:

```text
https://github.com/nepenth/synara-desktop/releases/download/pacman-repo/synara.db
```

9. Confirm installed-app update behavior:
   - iOS updates through TestFlight.
   - macOS updates through the Tauri updater flow.
   - Linux updates through `paru -Syu` or `sudo pacman -Syu`; the app may only
     notify/instruct.

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

## Release Constraints

- Human macOS and Linux package-install smoke remains required for each release
  candidate even when automated build and unit gates pass.
- Physical-device iOS upgrade, performance, APNs, and archive evidence remains
  a release-candidate responsibility.
- macOS uses signed Tauri updater metadata; Linux uses the GitHub
  Release-backed pacman repository; iOS uses TestFlight or the App Store.
- Production publication is blocked unless the exact-tag workflow validates all
  configured clients and protected credentials.
