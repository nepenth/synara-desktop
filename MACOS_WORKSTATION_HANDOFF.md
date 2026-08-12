# macOS Workstation Handoff

Reviewed: 2026-06-30

Purpose: give a Mac-hosted Codex session or human operator the exact context,
commands, evidence requirements, and prompt needed to execute the remaining
macOS, Xcode, Swift, simulator, and signed-runtime validation work.

## Current Context

- Repository: `nepenth/synara-desktop`.
- Primary Linux-local orchestration file: `PRODUCTION_READINESS_GOAL.md`.
- Mac/iOS queue: `MACOS_IOS_VALIDATION_QUEUE.md`.
- Production smoke checklist: `docs/production-smoke-checklist.md`.
- Timeline contract: `docs/timeline-open-focus-contract.md`.
- iOS status docs:
  - `synara-ios/docs/ios-validation-status.md`
  - `synara-ios/docs/ios-functionality-matrix.md`
  - `synara-ios/docs/e2ee-validation.md`
  - `synara-ios/docs/push-gateway-staging.md`
  - `synara-ios/docs/testflight-readiness.md`

The Linux-local Codex environment cannot run `xcodebuild`, `swift`, an iOS
simulator, macOS desktop runtime smoke, Apple signing, or notarization. Those
checks are release-gating and should be executed on the established macOS
workstation.

## Required Evidence Rules

Do not mark a queue item complete without recording:

- Commit SHA and branch.
- macOS version and hardware class.
- Xcode version.
- Swift version when Swift CLI is used.
- Simulator/device name and OS version for iOS checks.
- Exact command output or a concise attached log path.
- Per-case pass/fail notes.
- Failure reproduction steps, screenshots, screen recordings, or logs when any
  case fails.

Evidence should be added to `MACOS_IOS_VALIDATION_QUEUE.md`,
`docs/production-smoke-checklist.md`, or a linked release issue/PR. Keep secrets
out of committed files.

## Priority Work Items

| ID | Priority | Area | Required Work |
|---|---:|---|---|
| MAC-IOS-003 | P0 | Link Opening | Re-test after the next link-opening fix; current 2026-06-30 smoke says macOS links do not open the browser. |
| MAC-IOS-001 | P0 | Timeline Resurrection | Run iOS unit tests including `TimelineServiceTests`. |
| MAC-IOS-002 | P0 | Timeline Resurrection | Execute Timeline smoke cases from `docs/timeline-open-focus-contract.md` and `docs/production-smoke-checklist.md`. |
| MAC-IOS-005 | P0 | Composer Desktop Parity | Smoke native spellcheck, file drag/drop, screenshot paste, and mixed image+HTML paste on macOS. |
| MAC-IOS-006 | P0 | Native crypto-store recovery | Run the physical-Mac, operator-gated recovery rehearsal below; its restricted evidence rules override the general rules. |
| MAC-IOS-004 | P1 | Release Operations | If updater/signing variables are configured, run strict updater, signing, notarization, and artifact verification. |

## MAC-IOS-006: Native Crypto-Store Recovery Physical Rehearsal

### Gate and evidence boundary

This is a **physical-Mac-only, operator-gated** rehearsal of the native
password-login recovery UI. Run it only in an approved operator environment
with a non-production disposable account and isolated disposable fixtures. Do
not run it on a personal, production, or otherwise user-valued account/store.
Do not manually inspect, edit, delete, rename, or move a store or an OS
credential record. Any fixture setup or integrity check is supplied and run
only by the approved operator environment; this document deliberately gives no
such commands.

For this item, the following is the **only** record (and it overrides the
broader evidence rules above):

1. commit SHA;
2. macOS/Xcode/build-identity category;
3. fixed case ID and pass/fail;
4. static diagnostic ID (or `none` for the fixed success result); and
5. minimal redacted failure class (or `none`).

Do **not** retain or attach raw logs; store contents or paths; homeserver URLs;
account IDs; tokens; passwords; Keychain item, account, or service names;
archive names; or screenshots/screen recordings containing any of those. Do
not display, copy, capture, or record an opaque confirmation capability or any
of its values. Do not substitute terminal, developer-tools, or direct-command
interaction for the native UI.

This rehearsal makes no claim about signing, notarization, packaging, release
readiness, Apple review, or P5 approval. It does not change any existing
release, tag, or issue-policy requirement.

### Physical cases

Treat every case as a separate approved disposable fixture. Record only the
five fields above. A fixture that is unavailable is `Not run — no approved
fixture`, never a pass.

| Case | Native UI / approved-fixture action | Expected privacy-safe result |
|---|---|---|
| MAC-REC-01 | First normal password login, then quit and relaunch the physical Mac app. | The encrypted store has Keychain continuity: relaunch reopens it through the expected native session/login state, with no recovery offer. |
| MAC-REC-02 | Use an existing encrypted-store fixture whose Keychain access is unavailable or locked; attempt normal password login. | The login fails closed with `p3.2-login-store-locked`; recovery is not offered. The approved fixture's non-content integrity result is unchanged: no key creation/replacement and no automatic wipe. |
| MAC-REC-03 | Use separate existing encrypted-store fixtures with a missing Keychain record and a corrupt Keychain record; attempt normal password login. | Each normal login first fails closed with `p3.2-login-store-reset-required`. Before any explicit recovery, the approved fixture reports no key creation/replacement and no automatic wipe. |
| MAC-REC-04 | Check recovery availability with no preceding failed native login and after a non-eligible failed-store result; then check it after each eligible failed-store result. | Recovery is absent until the native login has failed with exactly `p3.2-login-store-reset-required` or `p3.2-login-store-migration-required`; it remains absent for locked/unavailable and generic store-open failure. |
| MAC-REC-05 | With an eligible result, type every text other than exact `ARCHIVE` in the visible native recovery dialog. Never reveal or capture confirmation contents. | The dialog disables or does not submit recovery; record observable UI non-submission and no archive/rebuild, with no diagnostic expected. Invalid, expired, and reused opaque-confirmation rejection is covered only by source/CI internal tests: it is deliberately not a physical UI rehearsal because the UI neither reveals nor permits manipulation or replay of the opaque confirmation. |
| MAC-REC-06 | From an eligible failed-store result, use the visible recovery flow with a valid one-use host confirmation and type exactly `ARCHIVE`; then use the normal native password-login flow again. | The UI reports the fixed privacy-safe result `archived_and_rebuilt`, the local layout is archived and rebuilt, and the subsequent fresh disposable-account login behaves as a fresh native login. |
| MAC-REC-07 | **Only if feasible in an approved, isolated controlled fixture**, exercise symlink/unsafe-layout rejection. This is not a human destructive instruction. | Recovery is refused with `p3.2-login-store-recovery-failed` before archive/rebuild or an external write; the fixture reports only the redacted integrity outcome. |

### Source-fact reconciliation (not physical evidence)

- `src-tauri/src/matrix/auth/product_commands.rs` arms recovery only after the
  two eligible static login diagnostics; locked, generic-open, and I/O failures
  remain fail-closed. Its explicit recovery path consumes the host confirmation
  before archive/rebuild and does not create, replace, rotate, or delete a
  Keychain key.
- `src-tauri/src/matrix/store/paths.rs` and `key_vault.rs` allow new key
  generation only for a genuinely fresh layout; an existing layout with a
  missing/unavailable/corrupt key-vault condition is not silently replaced.
  Managed symlinks are refused.
- `src-tauri/src/matrix/store/revision.rs` makes reset explicit, preserves the
  Keychain key, archives then rebuilds the local layout, and rejects unsafe
  recovery components before writing through them.
  `synara/src/app/pages/auth/login/loginUtil.ts` keeps the password-login
  native UI recovery affordance behind the same static diagnostic allowlist and
  records static diagnostics only.

Source review and CI can establish those expected facts, but neither is
physical-Mac evidence. Only the operator-gated cases above can establish the
physical hardware result.

## Suggested Preflight

From repository root:

```sh
git status --short --branch
git rev-parse --short HEAD
node --version
npm --version
xcodebuild -version
swift --version
npm run check:versions
npm run check:repo-layout
npm run check:matrix-boundaries
npm run check:production-smoke
npm run check:release-updater
npm run typecheck:modernization
npm --prefix synara run check:eslint
npm --prefix synara run check:prettier
```

From `synara-ios`:

```sh
xcodegen generate
xcodebuild -list -project Synara.xcodeproj
RUN_IOS_TESTS=1 IOS_TEST_DESTINATION='platform=iOS Simulator,name=iPhone 16' scripts/ci-build.sh
```

If the previously used simulator differs, use that destination and record it.

## Required macOS Desktop Launch Guardrail

Before executing detailed smoke cases, prove the committed desktop app can launch
with the local disabled-updater config:

```sh
npm run tauri dev
```

Record whether the main window opens and reaches the login/session screen. This
is specifically required because the 2026-06-29 `f938246` fix restored launch by
making updater plugin registration conditional on active `plugins.updater`
configuration. A successful compile is not enough evidence for native-shell
plugin/config changes.

## Mac Codex Prompt

Use this prompt in the macOS workstation Codex session:

```text
You are working on nepenth/synara-desktop on the macOS workstation. Treat the repository filesystem as authoritative. Do not broaden product scope unless asked.

Read these files first:
- MACOS_WORKSTATION_HANDOFF.md
- MACOS_IOS_VALIDATION_QUEUE.md
- PRODUCTION_READINESS_GOAL.md
- docs/production-smoke-checklist.md
- docs/timeline-open-focus-contract.md
- synara-ios/docs/ios-validation-status.md
- synara-ios/docs/ios-functionality-matrix.md
- synara-ios/docs/e2ee-validation.md
- synara-ios/docs/push-gateway-staging.md
- synara-ios/docs/testflight-readiness.md

Execute the queued macOS/iOS validation items with exact command output and evidence:
1. Prove macOS desktop launch with committed disabled-updater config.
2. Run iOS unit tests including TimelineServiceTests.
3. Run Timeline Resurrection smoke cases TL-001 through TL-010 where supported.
4. Re-run macOS desktop smoke for link opening after the Linux/macOS shared fix lands.
5. Run macOS desktop smoke for composer parity.
6. If updater variables/secrets are configured, run strict updater/signing/notarization validation.

Do not mark anything complete without commit SHA, branch, Xcode version, macOS version, simulator/device target, commands run, pass/fail notes, and failure reproduction details. MAC-IOS-006 is the exception: follow its operator-gated native-UI runbook and retain only its five restricted fields—never commands, logs, reproduction data, screenshots, or confirmation contents.

Update MACOS_IOS_VALIDATION_QUEUE.md and docs/production-smoke-checklist.md with evidence. If code changes are required, keep them scoped, run the relevant local gates, update CHANGELOG.md and PRODUCTION_READINESS_GOAL.md, and commit with a descriptive message.
```

## Expected Outputs

The Mac session should return or commit:

- Updated `MACOS_IOS_VALIDATION_QUEUE.md` statuses and evidence.
- Updated `docs/production-smoke-checklist.md` signoff/evidence links.
- Any failing commands with exact output and reproduction notes.
- If all P0 smoke passes, a clear statement that Timeline, link opening, and
  composer parity are evidence-ready for release review.
