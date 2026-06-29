# macOS and iOS Validation Queue

> Tracks release-gating checks that require a macOS workstation, Xcode, Swift, or an iOS simulator.
> Linux-local gates remain tracked in `PRODUCTION_READINESS_GOAL.md`.

## Execution Model

- Codex in the current Linux environment prepares code changes, exact commands, expected evidence, and review notes.
- A human or a Mac-hosted agent runs the queued commands on the established macOS workstation.
- Results should be pasted back into the session or committed into the relevant living artifacts.
- A queued item is not release-signed-off until pass/fail evidence includes command output, simulator/device target, date, and commit SHA.

## Pending Items

| ID | Priority | Area | Command / Checklist | Required Evidence | Status |
|---|---:|---|---|---|---|
| MAC-IOS-001 | P0 | Timeline Resurrection | Run the iOS unit test target that includes `TimelineServiceTests` from `synara-ios` using the standard Xcode scheme and simulator previously used for this repo. | Commit SHA, Xcode version, simulator name/iOS version, command, pass/fail output for `TimelineServiceTests`. | Pending |
| MAC-IOS-002 | P0 | Timeline Resurrection | Execute `docs/timeline-open-focus-contract.md` smoke checklist on iOS: fully-read room, one-unread room, stale saved history equivalent, jump latest, stale sync state, live appends, and timeline reset/gap cases. | Per-case pass/fail notes with room type, account state, and screen recording or concise reproduction notes for failures. | Pending |
| MAC-IOS-003 | P0 | Link Opening | After the desktop link-opening fix lands, smoke external links on macOS desktop: rich text and Matrix HTML links, normal message links, Hermes action/artifact links, settings/about links, profile/server "Open in Browser", OIDC account-management links, registration terms, feature-check help link, and location links. | Commit SHA, macOS version, app build type, each surface pass/fail, confirmation that links open in the system browser instead of an embedded webview. | Pending |
| MAC-IOS-004 | P1 | Release Operations | Validate macOS signing/notarization and updater prerequisites once updater work starts. | Command output, signing identity used, notarization status, updater metadata verification. | Pending |

## Completed Items

| ID | Date | Evidence |
|---|---|---|
| _None yet_ |  |  |
