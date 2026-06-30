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
| MAC-IOS-004 | P1 | Release Operations | If updater/signing variables are configured, run strict updater, signing, notarization, and artifact verification. |

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

Do not mark anything complete without commit SHA, branch, Xcode version, macOS version, simulator/device target, commands run, pass/fail notes, and failure reproduction details.

Update MACOS_IOS_VALIDATION_QUEUE.md and docs/production-smoke-checklist.md with evidence. If code changes are required, keep them scoped, run the relevant local gates, update CHANGELOG.md and PRODUCTION_READINESS_GOAL.md, and commit with a descriptive message.
```

## Expected Outputs

The Mac session should return or commit:

- Updated `MACOS_IOS_VALIDATION_QUEUE.md` statuses and evidence.
- Updated `docs/production-smoke-checklist.md` signoff/evidence links.
- Any failing commands with exact output and reproduction notes.
- If all P0 smoke passes, a clear statement that Timeline, link opening, and
  composer parity are evidence-ready for release review.
