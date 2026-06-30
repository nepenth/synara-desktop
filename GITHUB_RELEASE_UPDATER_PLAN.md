# GitHub Release Updater Project Plan

Reviewed: 2026-06-30

Purpose: park the auto-update work behind a clear plan so the team can resume it
later without blocking Timeline, link-opening, composer, macOS smoke, or iOS
validation work.

Release branch and client-visible publication strategy lives in
`RELEASE_BRANCH_CI_PLAN.md`.

## Executive Summary

Synara can use the Tauri v2 updater plugin with GitHub Releases as the update
channel. The app checks a static `latest.json` file hosted on the latest GitHub
release, downloads the platform-specific updater artifact, and verifies the
artifact signature against a public key embedded in the release-time app config.

Recommended endpoint shape:

```text
https://github.com/nepenth/synara-desktop/releases/latest/download/latest.json
```

This is feasible and aligned with Tauri's supported static JSON updater model.
It is not the highest-priority local task right now because the remaining proof
requires real signing secrets, repository variables, release artifacts, and a
signed release workflow run.

2026-06-30 key-material update: this trusted machine generated a
password-protected Tauri updater signing keypair and configured the GitHub
repository without committing key material. The private key and password are
stored as GitHub Actions secrets; the public key and endpoint are stored as
GitHub repository variables. The temporary local key files were removed. Keep
the feature parked until the desktop P0 link-opening blocker is fixed and the
maintainer explicitly resumes release workflow validation.

## Current Repository State

Implemented:

- Tauri updater plugin dependency and frontend package are present.
- Desktop runtime registers the Tauri updater plugin only when `plugins.updater`
  is present in the active config, so the committed disabled local config can
  still launch.
- Check-only updater permission scaffolding exists.
- `scripts/check-release-updater.mjs` provides advisory and strict readiness
  modes.
- `scripts/configure-release-updater.mjs` materializes release-time updater
  config from repository variables.
- `.github/workflows/release-desktop.yml` runs release-time updater config before
  strict validation and packaging.
- The release workflow is prepared to upload updater archives, `.sig` sidecars,
  and generated `latest.json`.
- `scripts/generate-release-updater-metadata.mjs` can generate static metadata
  for Linux and macOS updater artifacts.

Configured:

- `TAURI_SIGNING_PRIVATE_KEY` GitHub Actions secret.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` GitHub Actions secret.
- `SYNARA_UPDATER_PUBKEY` GitHub repository variable.
- `SYNARA_UPDATER_ENDPOINT` GitHub repository variable pointing at the latest
  GitHub release `latest.json`.
- Local dry-run inspection of release-time updater config passes with the
  configured public GitHub variables.

Deferred:

- Signed release workflow run.
- Hosted `latest.json` verification.
- Installed-app update check/install smoke.
- Final product UX decision: silent check, manual "Check for Updates", or
  prompted download/install.

Current blocking precondition:

- Desktop link opening is failing on both macOS and Linux by human smoke report
  on 2026-06-30. Do not spend additional implementation time on updater UX or
  release-channel proof until that P0 behavior is fixed and smoke-tested.

Latest local gate hardening:

- `scripts/check-release-updater.mjs` should require the workflow to download
  updater artifacts, run `scripts/generate-release-updater-metadata.mjs`, and
  upload generated `latest.json` instead of accepting a broad `latest.json`
  string match as signed metadata evidence.

## Required Secrets And Variables

GitHub repository variables:

| Name | Required | Notes |
|---|---:|---|
| `SYNARA_UPDATER_PUBKEY` | Yes | Public Tauri updater key. Safe to expose in app config. |
| `SYNARA_UPDATER_ENDPOINT` | Optional | Defaults to GitHub latest-release `latest.json` URL if omitted. Must be HTTPS if set. |

GitHub repository secrets:

| Name | Required | Notes |
|---|---:|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Yes | Private updater signing key. Never commit. |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | If key is password-protected | Required by Tauri signing if configured. |
| macOS Developer ID / notarization secrets | Yes for macOS releases | Already tracked by the release workflow and Mac validation queue. |

## Proposed Architecture

1. Build release jobs materialize updater config at runtime.
   - Keep committed `src-tauri/tauri.conf.json` free of production key material.
   - Release workflow injects `SYNARA_UPDATER_PUBKEY`.
   - Release workflow derives or injects the endpoint.

2. Release jobs build signed updater artifacts.
   - Linux builds AppImage updater archive plus `.sig`.
   - macOS builds app updater archive plus `.sig`.
   - macOS app signing/notarization remains separate and mandatory.

3. Metadata job generates `latest.json`.
   - Downloads Linux and macOS updater artifacts from workflow artifacts.
   - Validates each updater archive has a non-empty `.sig`.
   - Emits Linux and macOS platform entries.
   - Uploads `latest.json` to the GitHub release.

4. App-side update UX checks the endpoint.
   - Initial safe mode: expose a manual check in Settings/About.
   - Later mode: periodic background check with a clear prompt.
   - Do not silently install without explicit product decision.

## Implementation Milestones

### Milestone 1: Key Material And GitHub Configuration

1. Generate Tauri updater keypair using the current Tauri CLI.
2. Store public key in `SYNARA_UPDATER_PUBKEY`.
3. Store private key in `TAURI_SIGNING_PRIVATE_KEY`.
4. Store signing password if applicable.
5. Decide whether to set `SYNARA_UPDATER_ENDPOINT`; otherwise use GitHub latest.

Acceptance evidence:

- **Status:** Complete as of 2026-06-30.
- Repository variable/secret names and presence confirmed without exposing
  private values.
- Release-time updater config dry-run inspection passed using the configured
  GitHub public variables.
- Full `npm run check:release-updater -- --require-enabled` still belongs in a
  release job after config materialization, because committed local config
  remains intentionally disabled.

### Milestone 2: Release Workflow Proof

1. Publish a draft/test release from a disposable tag.
2. Confirm Linux and macOS jobs produce updater archives and `.sig` sidecars.
3. Confirm metadata job generates and uploads `latest.json`.
4. Download `latest.json` and verify it contains expected version/platform URLs.

Acceptance evidence:

- GitHub Actions run URL.
- Release URL.
- Artifact names.
- Redacted `latest.json` content or validation output.

### Milestone 3: Installed-App Smoke

1. Install version N from a signed release artifact.
2. Publish version N+1 as a test release.
3. Run app update check against the GitHub latest endpoint.
4. Verify no placeholder/local endpoints are contacted.
5. Verify update download/install behavior matches the chosen UX.

Acceptance evidence:

- App version before/after.
- Logs showing update check result.
- Pass/fail notes for macOS and Linux.

### Milestone 4: Product UX

Decide and implement one of:

- Manual "Check for Updates" in Settings/About.
- Startup check that prompts when an update is available.
- Background periodic check with non-intrusive prompt.

Acceptance evidence:

- UX copy reviewed.
- Updater permissions match the chosen behavior.
- Frontend tests cover no-update, update-available, error, and install failure
  states.

## Risks And Controls

| Risk | Control |
|---|---|
| Private signing key exposure | Store only in GitHub secrets; never print in logs. |
| Bad `latest.json` metadata | Generate from actual artifacts and `.sig` sidecars, then validate hosted output. |
| macOS update artifact not notarized/signed correctly | Keep macOS signing/notarization gates separate and release-blocking. |
| Accidental placeholder endpoint/key | Strict release checker must fail on placeholders and non-HTTPS endpoints. |
| Disabled local config breaks desktop launch | Keep updater plugin registration conditional on active `plugins.updater`; run macOS desktop launch smoke before resuming updater implementation. |
| Silent disruptive updates | Start with manual or prompted UX until product policy is explicit. |
| GitHub latest endpoint ambiguity | Use semver release discipline and avoid publishing broken latest releases. |

## Resume Prompt

Use this prompt when picking updater work back up:

```text
You are resuming the GitHub Release Updater project for nepenth/synara-desktop. Read GITHUB_RELEASE_UPDATER_PLAN.md, PRODUCTION_READINESS_GOAL.md, .github/workflows/release-desktop.yml, scripts/check-release-updater.mjs, scripts/configure-release-updater.mjs, scripts/generate-release-updater-metadata.mjs, and docs/production-smoke-checklist.md.

First inspect the current git status. Preserve unrelated user changes. Determine whether any local draft updater-checker edits should be kept, discarded, or folded into the next commit.

Do not expose secrets. Implement only the next smallest milestone toward signed GitHub-release updates. Prefer proving release workflow behavior and hosted metadata over adding UX. Run targeted script tests plus npm run check:release-updater, update CHANGELOG.md and PRODUCTION_READINESS_GOAL.md, then commit with a descriptive message.
```

## Current Recommendation

Park this work until after:

1. macOS and Linux link-opening smoke passes.
2. Timeline Resurrection smoke evidence is collected or daily use remains stable.
3. macOS composer parity smoke passes.
4. iOS Timeline/Xcode validation has a pass/fail result.

After those P0 user-facing checks are evidence-backed, resume this plan at
Milestone 2 because GitHub updater variables/secrets are already configured.

When release automation resumes, pair this updater plan with
`RELEASE_BRANCH_CI_PLAN.md` so clients only see updates after release branch CI,
signed artifacts, updater metadata, and publish approval all succeed.

Before making more updater changes, rerun the postmortem guardrail from
`PRODUCTION_READINESS_GOAL.md`: prove both committed disabled-config desktop
launch and release-time strict updater readiness.
