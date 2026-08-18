# Release Branch CI Plan

Reviewed: 2026-06-30

> **Historical plan, now implemented.** The exact-tag coordinated release model
> described below is represented by the current workflows. Use
> [the build and release runbook](docs/build-and-release.md) for operations and
> treat proposed/current wording below as design history.

Purpose: define the recommended release-branch and client-update path before
changing GitHub Actions behavior.

## Current CI And Release Shape

| Workflow                                      | Current Trigger                                | Current Role                                                                                            |
| --------------------------------------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `.github/workflows/ci.yml`                    | Push/PR to `main` and `release/**`, manual     | Desktop/runtime validation, actual iOS simulator tests, and one always-present aggregate quality gate.  |
| `.github/workflows/desktop-package-smoke.yml` | PR to `main` for package/runtime paths, manual | Linux `.deb` package and macOS `.app` package smoke with updater artifacts disabled.                    |
| `.github/workflows/ios-skeleton.yml`          | Manual                                         | On-demand unsigned iOS simulator build/test diagnostics with result bundles.                            |
| `.github/workflows/macos-signed-build.yml`    | Manual only                                    | Signed/notarized macOS DMG artifact, updater artifacts disabled.                                        |
| `.github/workflows/release.yml`               | Pushed `v*` tag                                | Coordinated macOS/Linux artifacts, TestFlight upload, updater metadata, and GitHub Release publication. |

## Problem Statement

The repo currently has enough pieces for signed update publication, but the
workflow trigger model is not ideal for a controlled client-visible update:

- Publishing a GitHub Release is what makes the update visible through the
  configured latest-release endpoint.
- The current release workflow starts after the release is published, which can
  create a window where the release exists before all artifacts and `latest.json`
  are present.
- `main` CI proves code health, but it does not represent a stabilized release
  candidate lane.
- Manual signed macOS build exists, but it is separate from final macOS updater
  and Linux pacman repo publication.

## Recommended Release Model

Use three lanes:

1. `main`: integration branch.

   - Continue running normal CI on every push and PR.
   - No client-visible update is published directly from `main`.

2. `release/vX.Y.Z`: release candidate branch.

   - Created from `main` when the release candidate is cut.
   - CI and package smoke should run on every push to the branch.
   - Signed package/release-candidate builds may run here, but should upload
     artifacts to GitHub Actions or a draft/prerelease only.
   - Do not update the production latest-release updater endpoint from every
     release-branch push.

3. Published GitHub Release `vX.Y.Z`: production update.
   - Only this should make clients see an update through
     `https://github.com/nepenth/synara-desktop/releases/latest/download/latest.json`.
   - Publication should happen after artifacts, signatures, notarization, and
     `latest.json` are verified.

## Proposed CI Changes

### Milestone 1: Release Branch Validation

Update CI triggers:

- `ci.yml`
  - Add push and PR coverage for `release/**`.
- `desktop-package-smoke.yml`
  - Add push and PR coverage for `release/**`.
- `ci.yml`
  - Run actual iOS simulator tests on every covered branch and aggregate them
    with desktop/runtime validation under the stable `Quality gate` job.

Acceptance:

- Push to `release/v1.2.20` runs CI and package smoke without manual dispatch.
- Pull requests targeting `release/**` run the same gates as release branch
  pushes.

Status:

- Implemented for `ci.yml` and `desktop-package-smoke.yml`; iOS execution is now
  part of the main CI gate instead of a path-filtered skeleton workflow.
- `desktop-package-smoke.yml` now includes Linux `.deb`, Linux Arch pacman
  package, and macOS `.app` artifacts for release-branch candidate smoke.
- Waiting for first `release/vX.Y.Z` branch push to provide live Actions
  evidence.

### Milestone 2: Release Candidate Artifact Workflow

Add or extend a workflow for release candidate artifacts:

- Trigger: `workflow_dispatch` and optionally push to `release/**`.
- Build Linux and macOS artifacts from the release branch.
- Linux release-candidate artifact for the current goal is the Arch/CachyOS
  pacman package used by `synara-desktop-bin`, not a Tauri AppImage self-update
  channel.
- Use signing/notarization where available.
- Upload artifacts to the workflow run or a GitHub prerelease marked as
  prerelease.
- Do not publish production `latest.json` to the latest stable release endpoint.

Acceptance:

- Release candidate artifacts are installable/testable by internal testers.
- Human smoke evidence can reference a workflow run and artifact IDs.
- No existing clients are offered the update automatically.

### Milestone 3: Controlled Production Publish

Replace or supplement the current `release.published` model with a controlled
publish workflow:

- Trigger: manual `workflow_dispatch` with version/tag input, or tag push
  `vX.Y.Z` plus an explicit publish step.
- Create or update a draft GitHub Release.
- Build Linux and macOS release artifacts.
- Upload macOS updater archives and `.sig` sidecars.
- Generate and upload macOS `latest.json`.
- Build the Arch-family pacman package, generate `synara.db` / `synara.files`,
  and publish the fixed `pacman-repo` GitHub Release assets.
- Verify hosted macOS `latest.json` and fixed pacman repo database.
- Publish the release only after the assets and metadata are present.

Acceptance:

- Clients cannot see the update until the workflow has uploaded and verified all
  required assets.
- Release evidence includes the GitHub Actions run URL, release URL, signed
  artifact names, metadata URL, fixed pacman repo URL, and updater verification
  output.

### Milestone 4: Environment Protection

Add a `production-release` GitHub Environment for workflows that use signing and
updater secrets.

Recommended controls:

- Required reviewer before production publish.
- Secrets scoped to the environment where practical.
- Release workflow cannot publish production updates from arbitrary branches.

Acceptance:

- Signed release publication requires intentional maintainer approval.
- Release candidate workflows can still build non-production artifacts without
  exposing unnecessary secrets.

## Current Day Priority

Do not implement these CI changes before the P0 link-opening fix unless the
maintainer explicitly changes priority.

Recommended order for today:

1. Fix macOS/Linux desktop link opening.
2. Run focused link-opening smoke on both platforms.
3. If link opening passes, implement Milestone 1 release branch validation.
4. Defer production publish workflow changes until the first release candidate
   has successful smoke evidence.

## Client Update Rule

Clients should only see a new update after:

1. Version metadata is bumped.
2. Release branch CI passes.
3. macOS/Linux package smoke passes.
4. Signed/notarized artifacts are generated.
5. macOS updater `.sig` files and `latest.json` are generated and verified.
6. The fixed `pacman-repo` release assets are replaced by CI, not by manual
   `repo-add` commands.
7. Full desktop/runtime and iOS validation reruns successfully at the exact tag
   SHA before any release artifact job starts.
8. The GitHub Release is intentionally published.
