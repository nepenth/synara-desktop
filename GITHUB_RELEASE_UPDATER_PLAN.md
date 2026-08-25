# GitHub Release Updater Project Plan

Reviewed: 2026-07-02

> **Historical implementation plan.** The exact-tag release workflow now owns
> signed macOS updater metadata, coordinated Linux packages, and internal
> TestFlight publication. Use [the build and release runbook](docs/build-and-release.md)
> for current operations. The detail below is retained as design history.

Purpose: park the auto-update work behind a clear plan so the team can resume it
later without blocking Timeline, link-opening, composer, macOS smoke, or iOS
validation work.

Release branch and client-visible publication strategy lives in
`RELEASE_BRANCH_CI_PLAN.md`.

## Executive Summary

Synara can use the Tauri v2 updater plugin with GitHub Releases as the macOS
update channel. The app checks a static `latest.json` file hosted on the latest
GitHub release, downloads the macOS updater artifact, and verifies the artifact
signature against a public key embedded in the release-time app config.

Recommended endpoint shape:

```text
https://github.com/nepenth/synara-desktop/releases/latest/download/latest.json
```

This is feasible and aligned with Tauri's supported static JSON updater model.
The repository now has the user-facing updater layer needed to prove the
installed-app path; the remaining proof requires release artifacts and a signed
release workflow run.

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
- Tauri process plugin dependency and frontend package are present for
  post-install relaunch.
- Desktop runtime registers the Tauri updater plugin only when `plugins.updater`
  is present in the active config, so the committed disabled local config can
  still launch.
- Desktop runtime registers the process plugin for restart support.
- Install-capable updater permissions and process restart permission exist in
  `src-tauri/capabilities/main.json`.
- Settings/About exposes an `Updates` tile with current version, last check
  state, and a manual `Check for Updates` action.
- A desktop updater provider runs a startup/background check cadence and shows
  non-blocking prompts for available updates.
- macOS available-update prompts can download, install, and relaunch through
  the Tauri updater/process plugins.
- Linux checks the GitHub latest-release metadata and shows APT or pacman/paru
  guidance without self-installing.
- The macOS app menu includes `Check for Updates...` and emits the same
  frontend manual-check event used by Settings/About.
- `scripts/check-release-updater.mjs` provides advisory and strict readiness
  modes.
- `scripts/configure-release-updater.mjs` materializes release-time updater
  config from repository variables.
- `.github/workflows/release.yml` runs release-time updater config before
  strict validation and packaging.
- The release workflow is prepared to upload macOS updater archives, `.sig`
  sidecars, and generated `latest.json`.
- `scripts/generate-release-updater-metadata.mjs` can generate static metadata
  for macOS updater artifacts, while still tolerating Linux entries if a future
  product decision adds them back.
- `scripts/build-pacman-repo.sh` generates a GitHub Release-backed pacman repo
  database for Arch-family Linux updates.
- `scripts/build-apt-repo.sh` generates a GitHub Release-backed flat APT repo
  for Debian-family Linux updates.
- `scripts/sign-apt-repo.sh` publishes independently verified `InRelease`,
  `Release.gpg`, and a client-scoped binary public keyring.

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
- Linux AppImage self-update. Product decision on 2026-06-30 is to use a
  package-manager-owned Linux update path through GitHub Release-backed APT
  and pacman repositories instead.

Current release-proof precondition:

- The first updater-capable build still must be installed manually. A real
  self-update proof requires publishing one updater-capable version, installing
  it, then publishing a newer version for it to detect and install.
- The manual `.github/workflows/macos-signed-build.yml` workflow is useful for
  signed/notarized DMG smoke, but it is not an updater-channel proof because it
  builds with `createUpdaterArtifacts:false` and does not materialize
  `plugins.updater`.
- Linux updater work must not wire Tauri self-install for pacman/paru installs.
  Linux app behavior should be notification/instruction only.

Latest local gate hardening:

- `scripts/check-release-updater.mjs` should require the workflow to download
  updater artifacts, run `scripts/generate-release-updater-metadata.mjs`, and
  upload generated `latest.json` instead of accepting a broad `latest.json`
  string match as signed metadata evidence.
- `scripts/check-release-updater.mjs` now also requires install-capable updater
  permissions, Tauri process plugin dependencies, process plugin registration,
  and `process:allow-restart` for macOS install/relaunch support.

## Required Secrets And Variables

GitHub repository variables:

| Name                      | Required | Notes                                                                                 |
| ------------------------- | -------: | ------------------------------------------------------------------------------------- |
| `SYNARA_UPDATER_PUBKEY`   |      Yes | Public Tauri updater key. Safe to expose in app config.                               |
| `SYNARA_UPDATER_ENDPOINT` | Optional | Defaults to GitHub latest-release `latest.json` URL if omitted. Must be HTTPS if set. |

GitHub repository secrets:

| Name                                      |                     Required | Notes                                                             |
| ----------------------------------------- | ---------------------------: | ----------------------------------------------------------------- |
| `TAURI_SIGNING_PRIVATE_KEY`               |                          Yes | Private updater signing key. Never commit.                        |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`      | If key is password-protected | Required by Tauri signing if configured.                          |
| macOS Developer ID / notarization secrets |       Yes for macOS releases | Already tracked by the release workflow and Mac validation queue. |

### Rotate Updater Signing Key Material

Use this when the release workflow fails with:

```text
failed to decode secret key: incorrect updater private key password
```

That error means `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` does not match
`TAURI_SIGNING_PRIVATE_KEY`. Rotate the updater keypair as a pair; do not try to
guess or partially update one secret.

Run from the repository root on a trusted machine with GitHub CLI access:

```sh
mkdir -p .secrets/updater
chmod 700 .secrets .secrets/updater

UPDATER_PASSWORD="$(openssl rand -base64 32)"

npm run tauri -- signer generate \
  --ci \
  --password "$UPDATER_PASSWORD" \
  --write-keys .secrets/updater/tauri-updater.key \
  --force

printf 'synara updater signing probe\n' >/tmp/synara-updater-signing-probe.txt
npm run tauri -- signer sign \
  -k "$(cat .secrets/updater/tauri-updater.key)" \
  -p "$UPDATER_PASSWORD" \
  /tmp/synara-updater-signing-probe.txt >/dev/null

gh auth refresh -h github.com -s repo -s workflow
gh secret set TAURI_SIGNING_PRIVATE_KEY < .secrets/updater/tauri-updater.key
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --body "$UPDATER_PASSWORD"
gh variable set SYNARA_UPDATER_PUBKEY --body "$(cat .secrets/updater/tauri-updater.key.pub)"
```

After the secrets and variable are updated, rerun the release workflow from the
GitHub Actions UI with **Re-run all jobs**, or run:

```sh
gh run rerun <release-workflow-run-id>
```

Delete `.secrets/updater/` after the GitHub secrets have been verified unless
the key is being moved into an approved password manager.

## Proposed Architecture

1. Build release jobs materialize updater config at runtime.
   - Keep committed `src-tauri/tauri.conf.json` free of production key material.
   - Release workflow injects `SYNARA_UPDATER_PUBKEY`.
   - Release workflow derives or injects the endpoint.

2. Release jobs build signed updater artifacts and package-managed Linux
   artifacts.
   - macOS builds app updater archive plus `.sig`.
   - macOS app signing/notarization remains separate and mandatory.
   - Linux builds the `synara-desktop-bin` pacman package in an Arch container.
   - Linux runs `scripts/build-pacman-repo.sh` and publishes the fixed
     `pacman-repo` release assets.
   - Linux runs `scripts/build-apt-repo.sh` and publishes the fixed `apt-repo`
     release assets.
   - Linux does not self-install updates from inside the app for this goal.

3. Metadata job generates `latest.json`.
   - Downloads macOS updater artifacts from workflow artifacts.
   - Validates each updater archive has a non-empty `.sig`.
   - Emits macOS platform entries.
   - Uploads `latest.json` to the GitHub release.

4. App-side update UX checks the endpoint.
   - Expose a manual check in Settings/About and the macOS app menu.
   - Run startup/background checks with a clear prompt.
   - Do not silently install without explicit user confirmation.

## Linux Distribution Policy

Decision, 2026-06-30:

- Linux release distribution is APT- or pacman/paru-owned for the current goal.
- Do not support Linux AppImage self-update right now.
- The Linux desktop app may notify that a newer package-managed release exists,
  but it must not download/install/replace itself.

Recommended Linux release shape:

1. Publish a fixed `pacman-repo` GitHub Release containing `synara.db`,
   `synara.files`, and `synara-desktop-bin-<version>-<pkgrel>-x86_64.pkg.tar.zst`.
2. Publish a fixed `apt-repo` GitHub Release containing `Packages`,
   `Packages.gz`, signed `Release` metadata, the public keyring, and
   `Synara_<version>_amd64.deb`.
3. Users configure the matching package-manager repository once.
4. Users update with APT, paru, or pacman.
5. In-app Linux update UI reports package-manager instructions only.

For notification-only UX, prefer checking the fixed pacman repo database or
package version for `synara-desktop-bin` when network access is available.
Checking the latest GitHub Release version is acceptable as a fallback, but it
can notify before the pacman repo assets have been replaced.

Do not point Linux installs at Tauri self-updater sidecar artifacts. The GitHub
Release-backed pacman repo should contain normal pacman package files intended
for package-manager installation.

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
2. Confirm macOS job produces updater archives and `.sig` sidecars.
3. Confirm metadata job generates and uploads macOS `latest.json`.
4. Confirm Linux Arch job produces `synara-desktop-bin` and the fixed
   `pacman-repo` release assets.
5. Download `latest.json` and `synara.db` and verify they contain expected
   version/platform/package data.

Acceptance evidence:

- GitHub Actions run URL.
- Release URL.
- Artifact names.
- Redacted `latest.json` and pacman repo validation output.

Scope note:

- The release workflow now treats Linux as package-manager-owned and
  notification-only for the current goal.
- Local workstation builds and the manual macOS signed-build workflow are not
  updater-enabled unless they run the release updater config step.
- Linux release evidence should verify GitHub Release artifact publication,
  pacman repo metadata, and package-manager update behavior, not Tauri
  self-update install behavior.

### Milestone 3: Installed-App Smoke

0. Implement a minimal updater invocation surface with no silent install
   behavior.
1. Install macOS version N from a signed release artifact built by the release
   workflow, not the manual macOS signed-build workflow.
2. Publish macOS version N+1 as a test release through the release workflow.
3. Run app update check against the GitHub latest endpoint.
4. Verify no placeholder/local endpoints are contacted.
5. Verify update download/install behavior matches the chosen UX.
6. Separately install Linux version N through the `synara` pacman repo, publish
   N+1, and verify `paru -Syu` or `sudo pacman -Syu` updates
   `synara-desktop-bin`. The Linux app should only notify/instruct the user to
   run package-manager updates.

Acceptance evidence:

- **Implementation status:** Local frontend/native implementation complete as
  of 2026-07-02; release smoke still pending.
- App version before/after.
- Logs showing update check result.
- Pass/fail notes for macOS and Linux.

### Milestone 4: Product UX

Implemented UX:

- Manual "Check for Updates" in Settings/About.
- macOS app menu "Check for Updates...".
- Startup and 12-hour background check that prompts when an update is available.
- macOS `Install and Restart` and `Later` prompt actions.
- Linux `Open Release Page` and `Later` prompt actions with package-manager
  guidance.

Acceptance evidence:

- **Implementation status:** Complete locally as of 2026-07-02.
- Frontend tests cover no-update, update-available, missing plugin/config,
  Linux comparison/guidance, dismissed-version suppression, and download
  progress.
- Release smoke must still prove the macOS install/relaunch path with real
  signed artifacts.

## Risks And Controls

| Risk                                                 | Control                                                                                                                          |
| ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| Private signing key exposure                         | Store only in GitHub secrets; never print in logs.                                                                               |
| Bad `latest.json` metadata                           | Generate from actual artifacts and `.sig` sidecars, then validate hosted output.                                                 |
| macOS update artifact not notarized/signed correctly | Keep macOS signing/notarization gates separate and release-blocking.                                                             |
| Accidental placeholder endpoint/key                  | Strict release checker must fail on placeholders and non-HTTPS endpoints.                                                        |
| Disabled local config breaks desktop launch          | Keep updater plugin registration conditional on active `plugins.updater`; run macOS desktop launch smoke before release signoff. |
| Silent disruptive updates                            | Start with manual or prompted UX until product policy is explicit.                                                               |
| GitHub latest endpoint ambiguity                     | Use semver release discipline and avoid publishing broken latest releases.                                                       |

## Resume Prompt

Use this prompt when picking updater work back up:

```text
You are resuming the GitHub Release Updater project for nepenth/synara-desktop. Read GITHUB_RELEASE_UPDATER_PLAN.md, PRODUCTION_READINESS_GOAL.md, .github/workflows/release.yml, scripts/check-release-updater.mjs, scripts/configure-release-updater.mjs, scripts/generate-release-updater-metadata.mjs, and docs/production-smoke-checklist.md.

First inspect the current git status. Preserve unrelated user changes. Determine whether any local draft updater-checker edits should be kept, discarded, or folded into the next commit.

Do not expose secrets. Implement only the next smallest milestone toward signed GitHub-release updates. Prefer proving release workflow behavior, hosted metadata, and installed-app update behavior now that the user-facing updater layer exists. Run targeted script tests plus npm run check:release-updater, update CHANGELOG.md and PRODUCTION_READINESS_GOAL.md, then commit with a descriptive message.
```

## Current Recommendation

Resume this plan at Milestone 2 because GitHub updater variables/secrets are
already configured and the user-facing updater layer exists. The decisive proof
is now a signed release workflow run followed by an installed-app N to N+1
macOS smoke.

When release automation resumes, pair this updater plan with
`RELEASE_BRANCH_CI_PLAN.md` so clients only see updates after release branch CI,
signed artifacts, updater metadata, and publish approval all succeed.

Before making more updater changes, rerun the postmortem guardrail from
`PRODUCTION_READINESS_GOAL.md`: prove both committed disabled-config desktop
launch and release-time strict updater readiness.
