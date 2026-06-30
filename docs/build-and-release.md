# Build And Release Runbook

Reviewed: 2026-06-30

This is the entry point for agents and maintainers preparing Synara Desktop
builds or releases. Read this before changing packaging, signing, updater, or
release workflow behavior.

## Release Lanes

| Lane | Purpose | Client-visible update? |
|---|---|---:|
| `main` | Integration branch. Runs normal CI on push and PR. | No |
| `release/vX.Y.Z` | Release candidate branch. Runs CI and desktop package smoke on push. | No |
| Published GitHub Release `vX.Y.Z` | Production artifact publication and updater metadata. | Yes, after assets and `latest.json` are uploaded |

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
6. Install and smoke the generated package artifacts.
7. Record smoke evidence in [production-smoke-checklist.md](production-smoke-checklist.md).

Release branch pushes build candidate artifacts for validation. They must not
publish a production updater channel.

## Production Publish Flow

Production release publication is intentionally separate from release branch
validation.

1. Create or update a GitHub Release for tag `vX.Y.Z`.
2. Publish only after the release branch and smoke evidence are approved.
3. The `Release Desktop` workflow builds release artifacts, updater archives,
   signatures, and `latest.json`.
4. Verify hosted `latest.json` before advertising the release.
5. Confirm installed-app update behavior on macOS and Linux before declaring the
   updater release ready.

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

Never commit updater private keys, Apple certificates, passwords, or notarization
credentials.

## Current Release Constraints

- Desktop external-link opening is a P0 smoke gate for macOS and Linux.
- Timeline/session-history behavior has a shipped mitigation but still needs
  daily-use and checklist evidence.
- GitHub-release updater key material is configured, but production updater
  workflow proof and installed-app update smoke remain deferred until P0 desktop
  behavior is green.
