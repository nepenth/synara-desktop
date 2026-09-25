# Public readiness review — 2026-09-25

Reviewed `origin/main` at `3e465acc` (Synara 2.1.41, merge of PR #1158).
The working branch for follow-up fixes is `feature/public-readiness-2026-09-25`.
This document records findings. It does not change product code.

The public repository is https://github.com/nepenth/synara-desktop
(public, AGPL-3.0-only, issues on, discussions off, GitHub description and
homepage empty).

## Verdict

**Not ready to promote yet.** The desktop client is a real, versioned, public
release with a serious Rust session-custody design, and the git history does
not contain a committed live credential. A Matrix community reader who follows
the links the docs treat as current will still be told the shipping client is
`matrix-js-sdk`, the shared core is unfinished, and access tokens are written
to `localStorage`. The GitHub landing page has no install path, no screenshots,
and no security contact. Two product behaviors should be decided before any
“secure modern client” announcement: the recovery key is written into
Downloads, and the iOS notification extension decrypts event bodies when
time-sensitive approvals are on, which is the default.

Safe to say today, once the front door matches it:

> Synara is an AGPL-3.0-only Matrix client from Whyland Creative LLC. It is a
> derivative of Cinny. Desktop 2.1.41 is published on GitHub Releases as a
> universal macOS disk image and x86_64 Linux packages. The desktop UI is
> Tauri 2 and React over a shared Rust core that uses matrix-rust-sdk. The
> same repository contains a SwiftUI iOS app on that core, currently shared
> with internal TestFlight testers. Windows, Android, a standalone web client,
> and voice or video calling are not supported. Source builds need Node
> 24.13.1 and Rust 1.96.

Do not say yet: production-ready, fully featured, audited, high-performance,
an Element replacement, App Store availability, ARM Linux packages, a
clean-slate codebase, or that calls can be started.

## What is already solid

- **Release exists.** Tag `v2.1.41` (2026-09-23) matches `CHANGELOG.md` and
  the app version. GitHub Releases publishes a universal macOS DMG, an amd64
  `.deb`, and an x86_64 Arch package. `release.yml` publishes those assets
  only on `v*` tags, after a notarization staple step for macOS.
- **Architecture claim in the root README is true.** `crates/synara-core` is
  the shared engine, `matrix-sdk` is pinned to `=0.19.1`, `src-tauri` depends
  on that crate, and `matrix-js-sdk` is not in the npm manifests. `NOTICE`
  correctly names the Cinny derivative and the AGPL-3.0-only license.
- **No live cloud tokens, private keys, or tracked `.env` files at HEAD or in
  history.** Workflows reference Apple signing material only as
  `${{ secrets.* }}`. `.env` is gitignored and was never committed.
  `npm run check:docs` passes.
- **Session custody is real.** Access tokens stay off the session DTO. Store
  keys live in the OS keychain. An existing store with a missing key is not
  rekeyed. TLS verification cannot be disabled in the client builder.
  `Cargo.lock` has no git-forked `matrix-sdk`.
- **CI is in good shape for a public repo.** Actions are pinned to commit
  SHAs and that pin is enforced. There is no `pull_request_target`. Default
  workflow permissions are `contents: read`. `contents: write` is limited to
  the tag-gated publish job. Fork pull requests do not receive repo secrets.
  Rust builds use `--locked` on toolchain 1.96.

## Findings

Severity is for public promotion and user safety, not a CVSS score.

### Security (product)

| Sev | Location | Finding | Remediation |
| --- | --- | --- | --- |
| High | `src-tauri/src/matrix/secret_storage/live.rs` (`save_recovery_document`, around line 283) | Secret-storage bootstrap writes the SDK recovery key to `~/Downloads/synara-recovery-key.txt` (`RECOVERY_DOCUMENT_NAME` in `crates/synara-core/src/app/secret_storage/mod.rs`). Unix mode is `0600` and the file is created with `create_new`. Downloads is a common cloud-sync folder. The IPC result returns the filename, not the key. | Stop writing the key into Downloads. Show it once in a protected UI, or save only through an explicit user-chosen path that is not a synced folder. |
| High | `crates/synara-core/src/app/nse_preview.rs`; `synara-ios/SynaraNotificationService/NotificationService.swift`; defaults in `synara-ios/SynaraShared/SynaraNotificationPreviewSupport.swift` | Lock-screen message previews default **off**. Time-sensitive agent approvals default **on**, and that path restores the shared session and decrypts notification bodies (clamped). The keychain item is `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`. | Decrypt only when previews are enabled, or classify approvals without retaining the body. Keep a generic approval string when previews are off. |
| Medium | `src-tauri/capabilities/main.json` | The main webview invokes login, secret-storage, room-key import/export, media save, and clipboard read. Those commands are the desktop UI for those tasks. Update install and process restart are macOS-only. External links are opened in Rust after a URL filter. | Accepted. Tauri grants a capability to a window. A second file that still targets `main` would leave the same commands callable. Login and recovery stay on this window. Rust zeroizes passphrases, shows a recovery key once, and returns room-key filenames rather than key bytes. |
| Medium | `src-tauri/src/matrix/media/product_commands.rs` | `matrix_media_download` returns decrypted bytes to the webview (up to 300 MiB). This contradicts the written “no media bytes on IPC” rule. | Keep display on opaque handles and stream saves in Rust. |
| Medium | `src-tauri/src/desktop_file_transfer.rs` | Composer saves use check-then-`fs::write` under Downloads. `fs::write` follows symlinks. The absolute path is returned to the webview. | Create with `O_EXCL` and no-follow. Do not return the path to the webview. |
| Medium | `src-tauri/src/desktop_logging.rs` | `desktop_append_log` accepts webview text. Redaction covers token, password, and `Bearer` shapes, then keeps 4,000 characters. No production caller that logs message bodies was found. | Drop arbitrary append from the webview. Log only allowlisted events. |
| Medium | `src-tauri/src/lib.rs` and `main.json` | Release UI was served from `http://localhost:<port>`, and the capability allowed `http://localhost:*/*`. The port was probed and released before the asset plugin listened. | Done on this branch: release and dev both use `WebviewUrl::App`. Release navigation is pinned to `tauri://localhost`. The main capability no longer grants a remote localhost origin. Widget loopback grants stay on the widget webview. A packaged binary has not been smoked. |
| Medium | `synara/src/app/pages/client/ClientNonUIFeatures.tsx` | Desktop approval notifications omit `commandPreview`, but the OS body can still include the agent-supplied `Reason:` line. | Use a generic string on the OS surface. Keep the reason inside the in-app review screen. |
| Low | `src-tauri/src/matrix/auth/product_commands.rs` | Login takes the password as a plain `String`. Registration and secret-storage inputs are zeroized. | Wrap the login password the same way. |
| Info | `src-tauri/tauri.conf.json` | `createUpdaterArtifacts` is false and the updater plugin is not configured in the committed tree. Tag release materializes the pubkey and endpoint. Install and relaunch permissions apply only on macOS. | Keep that release materialization. Linux continues to read `latest.json` and update through apt or pacman. |
| Info | iOS `Info.plist` files | `ITSAppUsesNonExemptEncryption` is false. ATS exceptions are absent. | Left false on purpose. The key tells Apple the app uses only exempt encryption: standard TLS and mass-market messaging cryptography. Changing it is an App Store export filing, and iOS is internal TestFlight only. Confirm the declaration with counsel before any App Store submission. |

The July 2026 threat model (`docs/matrix-rust-sdk/security-threat-model.md`,
`security-risk-register.md`) still says `ready_for_independent_review` and
still describes a `matrix-js-sdk` / SDK 0.18 cutover. Several of its open
risks are now mitigated in code (tokens off the session DTO, keychain vaults,
fail-closed missing store keys). Decrypted media bytes no longer cross IPC, and the recovery key is shown once instead of being written to Downloads. `synara-ios/docs/security-review.md`
(2026-05-27) says the notification extension does not decrypt; current code
does when keys exist.

This pass did not run the app, a homeserver, the NSE on a device, `cargo audit`,
or `npm audit`. It did not review the matrix-sdk-crypto gossip machine, every
Tauri command, or crash-report sinks.

### Secrets and privacy (repository)

No committed GitHub PATs, AWS keys, Slack tokens, OpenAI keys, private keys,
or signing material. `crates/synara-core/src/app/x509/test_ca.pem` is a public
test certificate, not a private key. `.env.macos-signing.example` uses
placeholder team id `ABC123DEFG` and placeholder Apple credentials. The
organization name in that file matches the public copyright in `NOTICE`.

| Sev | Location | Finding | Remediation |
| --- | --- | --- | --- |
| Medium | `synara/src/app/pages/client/home/__tests__/homeIdentity.test.ts:8-9`; `crates/synara-core/src/app/timeline/view.rs:1811` | Tests name a private homeserver host and a personal MXID localpart. The token in the test URL is the placeholder `secret`. | Replace with `matrix.example.org` and `@alice:example.org`. |
| Medium | `docs/matrix-rust-sdk/operating-instructions.md:45`; `docs/matrix-rust-sdk/product-lane-protocol.md:33` | Internal model selector naming a private host. | Generalize or remove. |
| Low | Git history, removed from HEAD | About 503 historical hits of a personal macOS home path in old planning docs (removed in `b1bbf897` / `14516d95`). Zero hits at HEAD. | Optional `git filter-repo` if operator path privacy matters. Not required to rotate credentials. Rewriting `main` invalidates tags. Prefer not rewriting. |
| Low | `commitmsg.txt` (tracked) | Draft commit message. No secrets. | Delete it and gitignore the filename. |

`check:docs` scans documentation-shaped paths. It would catch
that private homeserver host in `docs/` and would not catch it in the Rust and
TypeScript tests where it lives today. There is no repo-wide secret scanner
(gitleaks or equivalent) in CI.

`config.json` and `synara/config.json` list public homeservers
(`matrix.org`, `mozilla.org`, `converser.eu`, `unredacted.org`, `xmr.se`)
and public room aliases. That is product configuration, not a credential.

### Documentation

`npm run check:docs` passes. It checks private-data patterns and local
Markdown links. It does not check whether a page’s claims are still true.
There are roughly 390 tracked Markdown files. About 160 of them live under
`docs/matrix-rust-sdk/`. `docs/README.md` already explains the
current-versus-historical split. The problem is that ADR 0003, the Matrix
program index, the future-projects index, and the iOS README still point at
pages whose bodies describe an older client.

P0 — a newcomer would believe the wrong product:

1. `docs/matrix-rust-sdk/README.md` has a completed-archive banner, then a
   start-here table that calls `operating-instructions.md` the **live
   operating model** and `matrix-rust-sdk-full-replacement-plan.md` the
   **authoritative plan**.
2. `docs/matrix-rust-sdk-full-replacement-plan.md` still says, in present
   tense, that the shipping desktop product uses only `matrix-js-sdk` and
   targets SDK 0.18. Cargo pins `=0.19.1`.
3. `docs/matrix-rust-sdk/program-status.md` is labeled a frozen 2026-08-08
   snapshot, then still records release/main as `matrix-js-sdk-only` and
   cutover as `not_started`.
4. `docs/shared-native-core/README.md` and `11-implementer-playbook.md` still
   read as if the shared-core end state has not been reached. The root README
   and ADR 0003 send readers there for current detail.
5. `docs/desktop-secure-secret-storage-plan.md` contradicts itself. The
   decision section describes native-first session persistence.
   “Current State” still says login writes `synara_access_token` into
   `localStorage`. `synara/src/app/state/sessions.ts` marks those keys as
   retired and kept only for one-way cleanup. This is the security claim most
   likely to scare a Matrix reviewer.
6. `synara-ios/README.md` lists `ios-functionality-matrix.md` (2026-05-28) and
   `e2ee-validation.md` as current evidence. Those pages still say read
   receipts, polls, encrypted media, verification, and key backup are missing
   or not supported, and that the app is not ready for external TestFlight.
   Later changelog entries and Core media decrypt paths disagree.
7. No root `SECURITY.md`. The threat model a visitor will find is the
   pre-cutover July 2026 document.

P1 — confusing for contributors, not the announcement headline:

- `docs/shared-native-core/` files `01`, `06`, `08`, `10`, `12`, `13`, and
  `PLAN.md` still describe two engines or “current handoff” status. Several
  have no historical banner.
- `docs/adr/0003-shared-native-rust-core.md` still cites matrix-sdk 0.18 as
  current evidence and defers live status to the shared-core program docs.
- `docs/maturity_improvement_plan1.md`, `docs/native-first-architecture-spike.md`,
  `docs/desktop-matrix-sdk-boundaries.md`, and
  `docs/matrix-sdk-alignment-audit.md` still describe a js-sdk runtime. Some
  have banners below the status line.
- `docs/future-projects/rust-ownership-expansion/program/STATE.md` and
  `ACTIONS.md` (updated 2026-09-03) were not updated after the 2026-09-10
  desktop verification review.
- `docs/build-and-release.md` tells readers to run
  `npm run test:synapse-integration`, which does not exist. The real entry
  points are `npm run check:synapse-harness` and `integration/synapse/README.md`.
- Root `Cargo.toml` still says the workspace is “P1 — crate extraction” and
  that `src-tauri` is excluded because it does not depend on Core yet.
  `src-tauri/Cargo.toml` depends on `synara-core`.
- `docs/linux.md` says “Rust stable”. `rust-toolchain.toml` pins **1.96**.
  Root `package.json` `engines.node` is `>=22.12.0` while `.node-version` and
  the README require **24.13.1**.
- `src-tauri/tauri.conf.json` still has a Windows WiX block. The README’s
  “Windows is not supported” is the product statement.
- `.github/workflows/ci.yml` still lists pull-request branches
  `feature/matrix-rust-sdk-full-replacement` and `feature/shared-native-core`.

Leave `docs/reviews/`, `docs/releases/`, `CHANGELOG.md`, and the ADR archive
as provenance. They are safe once nothing current calls them the live
architecture.

### Git history

**Do not rewrite history** for secrets. Nothing matched “committed a live
secret, then deleted it.” `main` has 1,421 commits from 2026-05-12 through
2026-09-23. The pack is about 46 MiB. There is no `node_modules/`, `target/`,
or xcframework in history.

Polish, going forward:

- 107 separate `docs(core): refresh provenance after #NNN` commits landed in
  mid-August. Fold those bumps into the PR that caused them.
- Merge `afe1e314` (PR #666) has an ~83 KB body that pastes an internal
  operating log, including `agent@synara.local` and `prime-agent@local`
  trailers and a note that a model was “whyland-spark, locally hosted.”
  Not worth rewriting `main` and its tags. Worth shorter merge messages
  from here on.
- Conventional commits were 83–93% of non-merges in July and August, and
  about 7% of the last 150 non-merge subjects. The new subjects are full
  sentences, not junk. Pick one style.
- Author mail on `main` is GitHub noreply or `cursor.com`. No private mailbox
  domain showed up.
- The public remote still has on the order of 100 branches (`cursor/*`,
  `agent/*`, `release/*`, plus an unmerged alternate history). `main` itself
  is readable. The branch list is the noisy surface.

Largest blobs are avatar PNGs, `icon.icns`, a Twemoji font, and design
explorations. Optional later cleanup. Not a promotion blocker.

### Community packaging

The app can be downloaded. The GitHub front door does not say so.

1. Description, homepage, and topics are empty. The README opens into Node,
   Rust, and `npm ci`. A visitor never sees the 2.1.41 DMG or the Linux
   install lines that exist in `docs/linux.md`.
2. No screenshots or badges on the README. UI evidence exists only under
   `docs/ux-audit/` and `docs/reviews/assets/`.
3. No feature matrix. Changelog 2.1.39 is honest that call chrome does not
   offer Start/Join. Silence will be read as “calls ship.”
4. The README product table says iOS is “Internal TestFlight, then App Store
   release.” v2.1.41 assets contain no iOS build. The repo name is
   `synara-desktop`.
5. `synara/CONTRIBUTING.md` and `synara/CODE_OF_CONDUCT.md` are invisible to
   GitHub’s community profile (it looks at the root, `docs/`, and `.github/`).
   The conduct file has no contact address. `.github/ISSUE_TEMPLATE/config.yml`
   enables blank issues and has no contact links. There is no pull-request
   template.
6. Root `PROVENANCE.md` does not name Cinny. `NOTICE` does. A contributor
   guide should state AGPL-3.0-only, including the network-use source offer,
   and that contributions stay under that license.
7. `devAssets/index.html` is the only tracked file in that directory. It
   describes “fast, secure conversations” and references JS bundles that are
   not in git. Root `package.json` says “high-performance” with no public
   benchmark.
8. `docs/releases/v2.1.41.md` still reads like a pre-tag candidate note on a
   release that has already shipped.
9. Linux packages are amd64 / x86_64 only. Say that next to the install
   instructions.

### CI and supply chain

Already strong: SHA-pinned actions, read-only default token, no
`pull_request_target`, secrets confined to tag and manual signing workflows,
locked Rust and npm installs, Dependabot groups for npm, actions, and cargo,
Synapse images pinned to `postgres:16.9-bookworm` and `matrixdotorg/synapse:v1.161.0`.
`node scripts/check-workflow-policy.mjs` passes.

| Sev | Location | Finding | Remediation |
| --- | --- | --- | --- |
| Medium | `desktop-package-smoke.yml`, `release.yml` | `container: archlinux:base-devel` is a floating tag, and those jobs install Rust from pacman rather than toolchain 1.96. | Pin the image digest and install Rust 1.96. |
| Medium | `ci.yml`, `release.yml` | `brew install xcodegen` is unpinned. | Pin a formula version or a checksummed binary. |
| Medium | `desktop-package-smoke.yml` | Pull-request artifacts include installable `.deb`, Arch packages, and a macOS `.app` on a public repo (7-day retention). | Treat them as public, shorten retention, or upload only from `main` and tags. |
| Low | `ci.yml` | `cargo install cargo-audit --locked` does not pin the cargo-audit version. | Pin the version. |
| Low | `.github/dependabot.yml` | Cargo updates are configured for `/src-tauri` only. The workspace root lockfile (`crates/synara-core`, NSE) is not covered. `synara/package-lock.json` may need its own npm entry. | Add those directories. |
| Low | `.gitignore` | `*.p8` is ignored. `*.p12`, `*.mobileprovision`, and generic `*.pem` are not. The test CA is an intentional exception. | Ignore signing material, with an allowlist for `test_ca.pem`. |
| Info | `ci.yml` | Default pull requests skip long iOS simulator jobs unless labeled `needs-ios` or `needs-ios-ui`. A green check is not full iOS coverage. | Say so in the contributor guide. |

Outside contributors can pass the quality gate on desktop and docs changes
without Apple secrets. Failures they will hit are first-run workflow approval,
macOS runner quota, and path-triggered iOS or Synapse jobs.

## Remediation order on this branch

Do these before any community announcement. Do not rewrite git history in
this pass.

1. **Front door.** README install block (macOS DMG, amd64 APT, x86_64 pacman),
   honest capability table (E2EE messaging, rooms, spaces; no calls, no
   Windows, no Android, no web app; iOS is internal TestFlight only), Cinny
   and AGPL paragraph, Node 24.13.1 and Rust 1.96. Set the GitHub description,
   homepage, and topics when the README is ready. Add disposable-account
   screenshots.
2. **Community files.** Root or `.github/` `SECURITY.md` (contact, what not
   to post), `CONTRIBUTING.md`, and `CODE_OF_CONDUCT.md` with a real contact.
   Issue templates and a PR template. Point contributors at the label-gated
   iOS jobs.
3. **Stop the wrong-product docs.** Rewrite the start-here table in
   `docs/matrix-rust-sdk/README.md` so every row is archive-only. Put a
   one-line present-tense correction at the top of the full-replacement plan,
   the secret-storage “Current State,” the shared-core README, and the iOS
   “current evidence” list. Update ADR 0003’s SDK pin from 0.18 to 0.19.1.
   Replace the `Cargo.toml` workspace header.
4. **Privacy strings.** Replace the private homeserver host and personal MXID in
   tests. Generalize the internal model selector. Delete `commitmsg.txt` and
   gitignore it.
5. **Security decisions, then code.** Recovery-key file location, NSE decrypt
   default, and the main-webview capability split. These are product changes,
   not doc edits. Decide them explicitly before editing.
6. **CI pins.** Arch image digest and Rust 1.96 inside that job, xcodegen,
   cargo-audit, Dependabot directories, signing-file gitignore, artifact
   retention.

## What this review did not do

- Did not run the desktop app, iOS simulator, or a homeserver.
- Did not run `cargo audit` or `npm audit`.
- Did not OCR binary assets.
- Did not read the local untracked `.env` (256 bytes, gitignored).
- Did not rewrite history, delete remote branches, or change GitHub settings.
- Did not start the remediations above.
