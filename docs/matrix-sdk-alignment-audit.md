# Matrix SDK Alignment Audit

Date: 2026-06-07

> **Historical pre-cutover audit.** The two-backend architecture described
> below has been replaced by the shared Rust core. See
> [the codebase knowledge base](../CODEBASE_KNOWLEDGE_BASE.md) for the current
> implementation. This document remains a record of the findings that led to
> that work.

## Summary

Synara now has two Matrix implementation stacks:

- iOS uses native SwiftUI and Matrix Rust SDK Swift bindings.
- macOS and Linux use the Tauri desktop shell with the React runtime in
  `synara/`, currently backed by `matrix-js-sdk`.

The repo-wide goal is not to force the same language binding everywhere today.
The goal is to make Matrix behavior SDK-first and boundary-driven on every
platform:

- iOS should prefer Matrix Rust SDK APIs over direct Matrix REST.
- Desktop should centralize `matrix-js-sdk` usage and avoid ad hoc Matrix REST
  outside approved runtime boundaries.
- Shared Synara contracts should stay portable across platforms and avoid
  decrypted previews or platform-specific account data for common features.
- Remaining REST exceptions must be named, documented, tested, and removed as
  SDK-backed replacements land.

The iOS-specific audit and implementation plan lives at:

- [`synara-ios/docs/matrix-sdk-alignment-audit.md`](../synara-ios/docs/matrix-sdk-alignment-audit.md)

The macOS/Linux desktop boundary map lives at:

- [`docs/desktop-matrix-sdk-boundaries.md`](desktop-matrix-sdk-boundaries.md)

## Current Platform State

### iOS

Current strong points:

- SDK-backed login/session restore.
- SDK-backed room list and timeline streaming.
- SDK-backed text send and reply send.
- SDK-backed room management, public room search, room profile updates,
  notification mode, crypto status, recovery, and device verification request.
- SDK-backed media thumbnail/upload/send, reaction/redaction/edit, Later reads,
  push pusher registration, avatar media loading, and agent approval custom
  event sends.

Active alignment gaps:

- Rich media download/viewer behavior, upload progress/cancel/retry, and
  encrypted media UX remain.
- Later writes still need SDK-backed mutation.
- Room read-marker lookup is isolated in `MatrixRoomReadMarkerService` and still
  performs direct account-data HTTP pending SDK read-marker/account-data support.
- Device display-name repair remains a small direct HTTP exception pending SDK
  current-device display-name support.

### macOS And Linux Desktop

Current strong points:

- Desktop uses `matrix-js-sdk`, not hand-rolled Matrix sync.
- The Tauri shell does not maintain duplicate Matrix room state.
- Matrix account-data contracts for Synara features are documented under
  `synara/docs/`.
- Media, E2EE, calls, timeline, room state, and settings are mostly implemented
  through the runtime Matrix client.

Active alignment gaps:

- Matrix client access is broad across components and hooks, which makes future
  cross-platform parity audits expensive.
- Some direct Matrix REST exists in approved runtime support paths:
  - `synara/src/sw.ts` injects auth for Matrix media requests.
  - `synara/src/app/cs-api.ts` performs homeserver version discovery.
- `matrix-js-sdk` usage is currently accepted in many UI-facing files; a later
  desktop phase should introduce domain services/hooks that narrow direct client
  access around room list, timeline, media, account data, notifications, and
  device/security features.
- Desktop device naming now goes through `platform/device.ts`.
- Desktop message file, thumbnail, and video media rendering now starts going
  through `app/matrix/media.ts` for authenticated MXC resolution and encrypted
  download policy.
- A Rust-backed desktop Matrix domain core remains a strategic option, but it is
  a major migration and should be evaluated after iOS Matrix Rust SDK behavior
  stabilizes.

## Guardrail

`npm run check:matrix-boundaries` scans tracked production files for direct
Matrix REST/networking usage outside approved exception paths.

The guardrail currently allows known exceptions so the repo stays buildable
while remediation proceeds. Each exception is named in
`scripts/check-matrix-boundaries.mjs` and should be removed as the relevant
phase completes.

CI runs this check in `.github/workflows/ci.yml`.

## Repo-Wide Remediation Sequence

### SDK-1: Boundary Cleanup And Guardrails

Goal: make accidental Matrix REST or duplicate Matrix integration harder to
introduce.

Tasks:

- Add a repo-wide Matrix boundary check.
- Document iOS and desktop Matrix integration boundaries.
- Delete legacy iOS REST services after tests are migrated.
- For desktop, identify high-churn direct `MatrixClient` call sites and group
  them into domain buckets: room list, timeline, media, account data, settings,
  notification, device/security, and calls.

Acceptance criteria:

- CI fails on new unapproved direct Matrix REST usage.
- No new iOS SwiftUI view can add direct Matrix networking without updating the
  exception list and docs.
- Live runtime Matrix boundaries are documented for iOS, macOS, and Linux.
- The exception list becomes the working burn-down list for later phases.

### SDK-2: Timeline Actions

iOS:

- Move reactions to `Timeline.toggleReaction`.
- Move redactions to `Timeline.redactEvent`.
- Implement edits through `Timeline.edit`.
- Let timeline diffs update UI instead of manual local mutation.

Desktop:

- Audit reaction/redaction/edit paths against `matrix-js-sdk` best-practice
  APIs.
- Add contract tests that iOS and desktop emit compatible Matrix event
  semantics for reaction, edit, redaction, reply, and agent custom events.

Acceptance criteria:

- iOS action paths are SDK-backed.
- Desktop action paths are documented and contract-tested.
- Shared event semantics are covered by cross-platform fixtures.

### SDK-3: Profile, Avatar, Account Data

iOS:

- Add SDK-backed profile/avatar/account-data services.
- Move Later account data reads/writes to `Client.accountData` /
  `Client.setAccountData`.
- Remove profile/avatar direct networking from views.

Desktop:

- Keep `matrix-js-sdk` account-data APIs, but centralize Synara account-data
  reads/writes behind contract helpers.
- Confirm desktop and iOS use the same Later, room notes, unread anchor, room
  event anchor, and spaces schemas.

Acceptance criteria:

- iOS view-level Matrix profile/media networking is gone.
- Shared account-data fixtures pass on both runtimes.
- No platform-specific shared-feature account data is introduced.

### SDK-4: Media Pipeline

iOS:

- Move uploads/downloads/thumbnails to Matrix Rust SDK media APIs.
- Use SDK upload handles/progress where exposed.
- Centralize encrypted media policy.

Desktop:

- Audit `matrix-js-sdk` media helpers, service worker media auth, encrypted
  media decrypt path, and file-save/share behavior.
- Document whether desktop media behavior should remain JS SDK/WebView-owned or
  become a future Tauri Rust command boundary.

Acceptance criteria:

- iOS media no longer constructs Matrix media URLs directly.
- Desktop media exceptions are documented and tested.
- Encrypted media behavior is consistent across desktop and iOS where SDK
  support exists.

### SDK-5: Push And Notifications

iOS:

- Move Matrix pusher registration to Matrix Rust SDK `setPusher/deletePusher`.
- Use SDK `NotificationClient` for event-id-only push resolution.

Desktop:

- Audit notification settings, unread anchors, deep links, and native Tauri
  notification bridge.
- Keep OS notification shell behavior platform-native, but keep Matrix
  notification semantics in SDK/domain code.

Acceptance criteria:

- iOS pusher lifecycle is SDK-backed.
- Desktop and iOS route room/event/thread notification anchors consistently.
- Logs remain token-safe.

### SDK-6: Threads, Receipts, Typing, Search

iOS:

- Use SDK thread list service and focused timelines.
- Use SDK read receipt and typing APIs.
- Use SDK room/global search APIs where exposed.

Desktop:

- Audit existing thread, receipt, typing, and search behavior against
  `matrix-js-sdk` supported APIs.
- Add cross-platform fixture/contract coverage for thread anchors, read
  markers, typing state expectations, and search result navigation.

Acceptance criteria:

- iOS true thread behavior matches Matrix semantics.
- Desktop parity expectations are documented and tested.
- Search/read/typing behavior is no longer implemented ad hoc.

### SDK-7: Crypto Verification And Recovery

iOS:

- Complete SAS verification, recovery listeners, backup listeners, and
  encrypted media.

Desktop:

- Audit device/session naming, verification prompts, key backup, recovery, and
  encrypted media UX.
- Ensure device display names remain clear: `Synara iOS`, `Synara macOS`, and
  `Synara Linux`.

Acceptance criteria:

- Verification and recovery flows are release-grade on iOS.
- Desktop device/security UX remains compatible and clear.
- Encrypted rooms behave consistently across platforms.

## Strategic Desktop Decision Point

After SDK-1 through SDK-4 are complete on iOS, revisit whether macOS/Linux
should keep `matrix-js-sdk` long-term or migrate Matrix/domain logic into a
Rust-backed Tauri core using Matrix Rust SDK.

Decision inputs:

- iOS SDK implementation stability.
- Desktop media/E2EE/search/calls complexity.
- Linux packaging and runtime risk.
- Ability to preserve feature parity without rewriting the full React UI.
- Performance evidence from long-history rooms and encrypted media-heavy rooms.
