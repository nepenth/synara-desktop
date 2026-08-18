# macOS and iOS Validation Queue

> **Historical validation queue.** This file preserves evidence and open items
> from the 2026-06/07 readiness cycle. Do not treat its pending states as the
> current project status. Use [the 2026-08-17 consolidated proof](docs/shared-native-core/15-2026-08-17-local-proof.md),
> [desktop validation status](docs/desktop-validation-status.md), and
> [iOS validation status](synara-ios/docs/ios-validation-status.md).

> Tracks release-gating checks that require a macOS workstation, Xcode, Swift, or an iOS simulator.
> Linux-local gates remain tracked in `PRODUCTION_READINESS_GOAL.md`.

## Execution Model

- Codex in the current Linux environment prepares code changes, exact commands, expected evidence, and review notes.
- The current Linux environment cannot run `xcodebuild` or `swift`; those checks require the established macOS workstation or a Mac-hosted agent.
- Current host tool check, 2026-06-29: `grok` is available on PATH; `xcodebuild`, `swift`, and `composer-2.5-fast` are not available on PATH.
- A human or a Mac-hosted agent runs the queued commands on the established macOS workstation.
- Results should be pasted back into the session or committed into the relevant living artifacts.
- A queued item is not release-signed-off until pass/fail evidence includes command output, simulator/device target, date, and commit SHA.

## Tool-Bound Buckets

| Tooling | Applies To | Current Owner |
|---|---|---|
| `xcodebuild` | iOS unit/UI tests, simulator runs, macOS signed/notarized release builds | macOS workstation or Mac-hosted agent |
| `swift` | Swift package/test helpers and any local Swift contract validation outside Xcode | macOS workstation or Mac-hosted agent |
| macOS desktop runtime | System-browser link smoke, composer spellcheck/clipboard/drop smoke, signing/notarization | macOS workstation or human tester |

## Pending Items

| ID | Priority | Area | Command / Checklist | Required Evidence | Status |
|---|---:|---|---|---|---|
| MAC-IOS-000 | P0 | macOS Desktop Launch | From the repository root on the macOS workstation, run `npm run tauri dev` with the committed disabled-updater config and verify the main window reaches login/session UI. | Commit SHA, macOS version, command output or log excerpt, confirmation that the main window opens, and any stderr mentioning updater plugin disabled rather than startup failure. | Reported pass 2026-06-30; formal evidence pending |
| MAC-IOS-001 | P0 | Timeline Resurrection | Run the iOS unit test target that includes `TimelineServiceTests` from `synara-ios` using the standard Xcode scheme and simulator previously used for this repo. | Commit SHA, Xcode version, simulator name/iOS version, command, pass/fail output for `TimelineServiceTests`. | Completed 2026-07-07 |
| MAC-IOS-002 | P0 | Timeline Resurrection | Execute `docs/timeline-open-focus-contract.md` smoke checklist on iOS: fully-read room, one-unread room, stale saved history equivalent, jump latest, stale sync state, live appends, and timeline reset/gap cases. | Per-case pass/fail notes with room type, account state, and screen recording or concise reproduction notes for failures. | Desktop behavior much improved by 2026-07-07 human use, but visible initial-load repositioning remains; iOS/formal evidence pending |
| MAC-IOS-003 | P0 | Link Opening | After the packaged-localhost capability fix lands, smoke external links on macOS desktop: rich text and Matrix HTML links, normal message links, Hermes action/artifact links, settings/about links, profile/server "Open in Browser", OIDC account-management links, registration terms, feature-check help link, and location links. | Commit SHA, macOS version, app build type, each surface pass/fail, confirmation that links open in the system browser instead of an embedded webview. | Failed again 2026-07-07 human smoke on macOS/Linux; native IPC capability fix pending packaged re-smoke |
| MAC-IOS-004 | P1 | Release Operations | GitHub updater secrets/variables are configured. When release validation resumes, run the signed/notarized release build path and verify the strict updater gate in the release job. | GitHub Actions run URL, command output, signing identity used, notarization status, updater metadata/signature verification, generated `.sig`/updater archive paths, and confirmation that the release workflow no longer overrides `createUpdaterArtifacts` to `false`. | Key material configured 2026-06-30; signed release proof pending |
| MAC-IOS-005 | P0 | Composer Desktop Parity | Smoke the desktop composer on macOS: native spellcheck in the Slate composer, drag/drop one and multiple files into a room, paste a screenshot/native clipboard image, and paste an image copied from a browser that also advertises HTML/text. | Commit SHA, macOS version, app build type, each surface pass/fail, upload board evidence for dropped/pasted files, and notes for any native spellcheck or paste failures. | Failed 2026-07-07 human smoke on macOS/Linux; packaged-localhost native IPC and spellcheck hint fixes pending packaged re-smoke |
| MAC-IOS-006 | P0 | Native crypto-store recovery | On a physical Mac, run the operator-gated native password-login recovery rehearsal in `MACOS_WORKSTATION_HANDOFF.md` only with its approved disposable account and isolated fixtures. No manual store/Keychain manipulation or locally invented commands. | **Only** commit SHA; macOS/Xcode/build-identity category; fixed case pass/fail; static diagnostic ID; and minimal redacted failure class. No logs, stores/paths, endpoints, account/credential data, Keychain names, archive names, screenshots, or opaque confirmation contents. | Pending physical-Mac operator evidence; source/CI evidence is not physical evidence |

Exact evidence fields and per-surface smoke cases are consolidated in
`docs/production-smoke-checklist.md`.

## Completed Items

| ID | Date | Evidence |
|---|---|---|
| MAC-IOS-001 | 2026-07-07 | Commit `258e2ba`; Xcode 26.5 (17F42); simulator `iPhone 17`, iOS 26.5 (`00000000-0000-0000-0000-000000000000`); XcodeBuildMCP `test_sim` with `CODE_SIGNING_ALLOWED=NO CODE_SIGNING_REQUIRED=NO CODE_SIGN_STYLE=Manual DEVELOPMENT_TEAM= -only-testing:SynaraTests/TimelineServiceTests`; passed 35 tests, failed 0; result bundle `/Users/example/Library/Developer/XcodeBuildMCP/workspaces/synara_project-4f1bff14f6ef/result-bundles/test_sim_2026-07-08T00-56-45-394Z_pid29238_65a5a5a1.xcresult`. |
