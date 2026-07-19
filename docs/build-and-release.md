# Build And Release Runbook

Reviewed: 2026-07-19

This is the entry point for agents and maintainers preparing Synara builds or
releases. Read this before changing packaging, signing, updater, TestFlight, or
release workflow behavior.

## Release Lanes

| Lane                              | Purpose                                                              |             Client-visible update? |
| --------------------------------- | -------------------------------------------------------------------- | ---------------------------------: |
| `main`                            | Integration branch. Runs normal CI on push and PR.                   |                                 No |
| `release/vX.Y.Z`                  | Release candidate branch. Runs CI and desktop package smoke on push. |                                 No |
| Published GitHub Release `vX.Y.Z` | Coordinated desktop publication and internal TestFlight upload.      | Yes, after each platform processes |

Do not publish a GitHub Release until the release branch gates, desktop package
smoke, and human smoke checklist have passed.

## Local Prerequisites

1. Install Node from `.node-version`.
2. Install Rust stable.
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
npm run check:matrix-boundaries
npm --prefix synara run typecheck:modernization
npm --prefix synara run test:modernization
npm --prefix synara run check:eslint
npm --prefix synara run check:prettier
cargo check --manifest-path src-tauri/Cargo.toml --locked
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run check:production-smoke
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
   - `CI`
   - `Desktop Package Smoke`
   - `iOS Skeleton` when iOS paths changed
6. Install and smoke the generated package artifacts:
   - `synara-macos-app`: unsigned/ad-hoc macOS `.app` release-candidate smoke artifact.
   - `synara-linux-arch-pkg`: Arch/CachyOS pacman package artifact for
     `pacman -U` smoke and GitHub Release-backed pacman repo validation.
   - `synara-linux-deb`: Debian-family package smoke artifact.
7. Record smoke evidence in [production-smoke-checklist.md](production-smoke-checklist.md).

Release branch pushes build candidate artifacts for validation. They must not
publish a production updater channel.

## Production Publish Flow

Production release publication is intentionally separate from release branch
validation.

1. Create or update a GitHub Release for tag `vX.Y.Z`.
2. Publish only after the release branch and smoke evidence are approved.
3. The `Release Synara` workflow first requires the tag to equal the shared
   repository version, then builds every client from that tag:
   - macOS signed/notarized DMG, macOS updater archive, signatures, and
     `latest.json`.
   - Linux `.deb`.
   - Arch-family `synara-desktop-bin` package plus fixed `pacman-repo` release
     assets (`synara.db`, `synara.files`, and package file).
   - iOS signed App Store archive uploaded for internal TestFlight testing.
4. Verify the iOS build finishes App Store Connect processing and appears in
   the configured internal TestFlight group.
5. Verify hosted macOS `latest.json` before advertising in-app macOS updates.
6. Verify the fixed pacman repo URL before advertising Arch-family updates:

```text
https://github.com/nepenth/synara-desktop/releases/download/pacman-repo/synara.db
```

7. Confirm installed-app update behavior:
   - iOS updates through TestFlight.
   - macOS updates through the Tauri updater flow.
   - Linux updates through `paru -Syu` or `sudo pacman -Syu`; the app may only
     notify/instruct.

The updater implementation plan and required GitHub variables/secrets live in
[../GITHUB_RELEASE_UPDATER_PLAN.md](../GITHUB_RELEASE_UPDATER_PLAN.md). The
release-branch CI strategy lives in [../RELEASE_BRANCH_CI_PLAN.md](../RELEASE_BRANCH_CI_PLAN.md).

## Required Release Secrets

macOS releases require Apple Developer ID and notarization secrets documented in
[../README.md](../README.md).

Updater-enabled releases require:

- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
- `SYNARA_UPDATER_PUBKEY`
- `SYNARA_UPDATER_ENDPOINT`

Internal TestFlight releases require the tag-restricted `testflight` GitHub
environment and its Apple Distribution certificate, app and notification
service provisioning profiles, and App Store Connect API key secrets. The
complete variable list and rotation notes live in
[../synara-ios/docs/testflight-upload.md](../synara-ios/docs/testflight-upload.md#coordinated-github-release).

Never commit updater private keys, Apple certificates, passwords, or notarization
credentials.

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

## Current Release Constraints

- Desktop external-link opening is a P0 smoke gate for macOS and Linux.
- Timeline/session-history uses bounded rendering and single-owner placement but
  still needs daily-use evidence from `docs/timeline-diagnostics.md` before the
  release candidate is promoted.
- GitHub-release updater key material is configured. Production workflow proof
  now splits by platform: macOS uses Tauri updater metadata, while Linux uses
  the GitHub Release-backed pacman repo.
