# Synara Shared Contracts

Reviewed: 2026-08-24

This directory contains machine-readable contract artifacts for behavior that
must remain compatible across the desktop runtime, macOS/Linux shells, and the
future native iOS app.

Human-readable inventory: [Synara Shared Contract Inventory](../synara-contracts.md).

## Contract Policy

- JSON Schemas describe canonical writer payloads.
- Runtime readers may accept and normalize older or non-canonical payloads when
  the Markdown contract says they should.
- Fixtures are conformance examples for TypeScript and future Swift tests.
- Schemas must not encode desktop-only UI assumptions.
- Schemas must not require decrypted message previews, access tokens, device
  tokens, recovery keys, or platform credentials.

## Artifacts

- `synara-agent-action.schema.json`: canonical bounded agent action payload.
- `fixtures/synara-agent-action.json`: valid, invalid, and normalization
  fixtures for agent actions.
- `synara-agent-approval-action.schema.json`: canonical v1 approve/reject
  result payload for `in.synara.agent.action`.
- `fixtures/synara-agent-approval-action.json`: valid and invalid approval
  result fixtures.
- `synara-agent-card.schema.json`: canonical structured agent-card payload.
- `fixtures/synara-agent-card.json`: valid, invalid, and runtime parsing
  fixtures for agent cards.
- `synara-later-content.schema.json`: canonical v1 `in.synara.later` account
  data payload.
- `fixtures/synara-later-content.json`: valid, invalid, and normalization
  fixtures for Later account data.
- `synara-room-notes-content.schema.json`: canonical v1
  `in.synara.room_notes` account data payload.
- `fixtures/synara-room-notes-content.json`: valid, invalid, and
  normalization fixtures for room notes account data.
- `synara-room-event-anchor.schema.json`: canonical opaque Matrix room/event
  anchor payload.
- `fixtures/synara-room-event-anchor.json`: valid and invalid room/event/thread
  anchor fixtures.
- `synara-spaces-content.schema.json`: canonical `in.synara.spaces` sidebar
  ordering and folder payload.
- `fixtures/synara-spaces-content.json`: valid and invalid space/folder
  fixtures.
- `synara-unread-anchor-content.schema.json`: canonical v1
  `in.synara.unread_anchor` account data payload.
- `fixtures/synara-unread-anchor-content.json`: valid and invalid unread
  anchor fixtures.
- `synara-safe-remote-url.schema.json`: canonical public HTTPS URL policy.
- `fixtures/synara-safe-remote-url.json`: accepted and rejected URL fixtures.
- `synara-shared-settings.schema.json`: canonical platform-neutral settings
  payload.
- `synara-desktop-platform-settings.schema.json`: canonical desktop-only
  platform settings payload.
- `fixtures/synara-settings.json`: valid and invalid settings split fixtures.
- `synara-notification-summary.schema.json`: canonical notification summary
  output counts.
- `fixtures/synara-notification-summary.json`: valid, invalid, and formula
  fixtures for notification summaries.
- `synara-route.schema.json`: canonical app-relative route payload.
- `fixtures/synara-route.json`: valid, invalid, semantic-invalid, and parsed
  destination fixtures for routes.

## Ownership

| Contract                  | Desktop-runtime owner                                                  | iOS owner                        | Compatibility rule                                                                 |
| ------------------------- | ---------------------------------------------------------------------- | -------------------------------- | ---------------------------------------------------------------------------------- |
| Agent action payloads     | `src/app/agents/agentActions.ts`                                       | Future native agent-card service | Writers emit bounded canonical payloads; readers validate and fail closed.         |
| Agent approval results    | Future agent action bridge                                             | Native agent approval service    | Writers emit canonical v1 Matrix room events; readers ignore unsupported versions. |
| Agent cards               | `src/app/utils/hermes.ts`                                              | Future native agent-card service | Explicit structured keys only; safe URLs and bounded sections.                     |
| `in.synara.later`         | `src/app/utils/later.ts`                                               | Future native Later service      | Writers emit canonical v1; readers normalize legacy v1.                            |
| `in.synara.room_notes`    | Shared Core `room_notes_live.rs`; `nativeRoomNotesOwner.ts` projection | `SharedCoreRoomNotesService`     | Writers emit canonical v1; readers normalize malformed or oversized items.         |
| `in.synara.spaces`        | `src/app/hooks/useSidebarItems.ts`                                     | Future native sidebar model      | Folder/ordering account data stays Matrix ID-only and portable.                    |
| `in.synara.unread_anchor` | `src/app/utils/notifications.ts`                                       | Future native unread model       | Private unread anchors store event IDs only; no decrypted previews.                |
| Media/external URL policy | `src/app/utils/remoteContent.ts`                                       | Future native media service      | Public HTTPS only; local/private/credentialed targets fail closed.                 |
| Notification summaries    | `src/app/notifications/badgeSummary.ts`                                | Future native notification model | Writers emit non-negative integer counts; formulas remain cross-platform.          |
| Room/event/thread anchors | Routes, Later, timeline helpers                                        | Future native navigation model   | Anchors contain opaque Matrix IDs only; readers resolve local context after sync.  |
| App-relative route paths  | `src/app/routes/synaraRoutes.ts`                                       | Future native route parser       | Writers emit internal paths; readers reject malformed or unsupported routes.       |
| Settings compatibility    | `src/app/state/settings.ts`                                            | Future native settings store     | Shared settings stay platform-neutral; platform settings stay channel-specific.    |
