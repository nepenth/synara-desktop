# Synara Shared Contract Inventory

Reviewed: 2026-08-24

Status: portable contract inventory, updated as shared-core and native iOS
owners land. Individual ownership rows are evidence for those verticals, not a
claim of whole-client parity. Machine fixtures remain the cross-client
conformance boundary.

## Contracts

| Contract                  | Human contract                                      | Machine artifact                                                                                       | Desktop-runtime owner                                                  | iOS owner                        | Compatibility rule                                                                 |
| ------------------------- | --------------------------------------------------- | ------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------- | -------------------------------- | ---------------------------------------------------------------------------------- |
| Agent action payloads     | [Agent Action](./synara-agent-action-contract.md)   | `docs/contracts/synara-agent-action.schema.json`                                                       | `src/app/agents/agentActions.ts`                                       | Future native agent-card service | Writers emit bounded canonical payloads; readers validate and fail closed.         |
| Agent approval results    | [Agent Action](./synara-agent-action-contract.md)   | `docs/contracts/synara-agent-approval-action.schema.json`                                              | Future agent action bridge                                             | Native agent approval service    | Writers emit canonical v1 Matrix room events; readers ignore unsupported versions. |
| Agent cards               | [Agent Card](./synara-agent-card-contract.md)       | `docs/contracts/synara-agent-card.schema.json`                                                         | `src/app/utils/hermes.ts`                                              | Future native agent-card service | Explicit structured keys only; safe URLs and bounded sections.                     |
| Later account data        | [Later](./synara-later-contract.md)                 | `docs/contracts/synara-later-content.schema.json`                                                      | `src/app/utils/later.ts`                                               | Future native Later service      | Writers emit canonical v1; readers normalize legacy v1.                            |
| Media/external URL policy | [Media Policy](./synara-media-policy.md)            | `docs/contracts/synara-safe-remote-url.schema.json`                                                    | `src/app/utils/remoteContent.ts`                                       | Future native media service      | Public HTTPS only; local/private/credentialed targets fail closed.                 |
| Notification summaries    | [Notification](./synara-notification-contract.md)   | `docs/contracts/synara-notification-summary.schema.json`                                               | `src/app/notifications/badgeSummary.ts`                                | Future native notification model | Writers emit non-negative integer counts; formulas remain cross-platform.          |
| Room notes                | [Room Notes](./synara-room-notes-contract.md)       | `docs/contracts/synara-room-notes-content.schema.json`                                                 | Shared Core `room_notes_live.rs`; `nativeRoomNotesOwner.ts` projection | `SharedCoreRoomNotesService`     | Writers emit canonical v1; readers normalize malformed or oversized items.         |
| Room/event/thread anchors | [Anchors](./synara-room-event-anchor-contract.md)   | `docs/contracts/synara-room-event-anchor.schema.json`                                                  | Routes, Later, timeline helpers                                        | Future native navigation model   | Anchors contain opaque Matrix IDs only; readers resolve local context after sync.  |
| Route paths               | [Route](./synara-route-contract.md)                 | `docs/contracts/synara-route.schema.json`                                                              | `src/app/routes/synaraRoutes.ts`                                       | Future native route parser       | Writers emit internal paths; readers reject malformed or unsupported routes.       |
| Settings compatibility    | [Settings](./synara-settings-compatibility.md)      | `docs/contracts/synara-shared-settings.schema.json` and `synara-desktop-platform-settings.schema.json` | `src/app/state/settings.ts`                                            | Future native settings store     | Shared settings stay platform-neutral; platform settings stay channel-specific.    |
| Space folders             | [Spaces](./synara-spaces-contract.md)               | `docs/contracts/synara-spaces-content.schema.json`                                                     | `src/app/hooks/useSidebarItems.ts`                                     | Future native sidebar model      | Folder/ordering account data stays Matrix ID-only and portable.                    |
| Unread anchors            | [Unread Anchor](./synara-unread-anchor-contract.md) | `docs/contracts/synara-unread-anchor-content.schema.json`                                              | `src/app/utils/notifications.ts`                                       | Future native unread model       | Private unread anchors store event IDs only; no decrypted previews.                |

## Forward Compatibility

- Unknown fields in Matrix account data should be ignored by readers unless the
  human contract says to reject them for security.
- Canonical writer schemas are stricter than tolerant runtime readers.
- iOS-specific Matrix account data must not be added for shared features until
  this inventory and `docs/synara-namespaces.md` are updated.
- Generated Swift types may come from these schemas or from manually mirrored
  types, but fixture conformance tests must remain the acceptance gate.

## Baseline and Remaining Validation

Completed locally:

- Public web-client product promise retired in favor of native app channels.
- Desktop APIs abstracted toward platform contracts.
- Desktop bridge compatibility aligned with the parent Tauri shell for
  integration status, tray state, secure credentials, and structured shortcut
  registration results.
- Native credential/session storage direction implemented for desktop.
- Shared contracts documented for routes, notifications, agent actions, agent
  cards, Later, room notes, unread anchors, space folders, anchors, media URL
  policy, and settings compatibility.
- JSON Schema and fixture coverage added for all contracts with current runtime
  behavior.

Remaining validation and migration work:

- Human macOS desktop smoke validation beyond package build.
- Human Linux desktop smoke validation beyond package build.
- Any fixes discovered by those validations.
- Continue moving portable feature ownership into SharedCore and add Swift
  fixture conformance coverage as each vertical lands.
- Human/legal/App Store account tasks when moving into real iOS release work.
