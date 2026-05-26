# Synara Notification Contract

Reviewed: 2026-05-25

Status: initial shared contract with runtime summary logic in
`src/app/notifications/badgeSummary.ts` and route payload validation through
`src/app/routes/synaraRoutes.ts`. The canonical summary schema and fixtures
live under `docs/contracts/`.

## Purpose

Synara notification state must be explainable across macOS, Linux, and the
future iOS app without copying UI implementation details. This contract defines
the portable counts and the desktop app-badge formula that native platforms
should preserve until a later version explicitly changes it.

## Count Model

Machine-readable artifacts:

- [synara-notification-summary.schema.json](./contracts/synara-notification-summary.schema.json)
- [synara-notification-summary.json fixtures](./contracts/fixtures/synara-notification-summary.json)

The JSON Schema defines canonical summary output. Runtime formula fixtures
prove how source counts normalize into that summary.

```ts
type NotificationSummary = {
  appBadgeCount: number;
  inboxBadgeCount: number;
  laterActiveCount: number;
  inviteCount: number;
  agentApprovalCount: number;
  highlightCount: number;
  unreadCount: number;
};
```

All counts are non-negative integers. Fractional values are floored. Missing,
invalid, negative, or non-finite values normalize to `0`.

## Source Counts

| Field                | Meaning                                                                                                                          | Current source                                                                           |
| -------------------- | -------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| `highlightCount`     | Sum of room highlight counts. A room with an explicit highlight field contributes highlight count instead of total unread count. | Matrix room unread/highlight state.                                                      |
| `unreadCount`        | Sum of unread totals for rooms that do not provide an explicit highlight count.                                                  | Matrix room unread state.                                                                |
| `laterActiveCount`   | Later items that are not completed.                                                                                              | `in.synara.later` account data.                                                          |
| `inviteCount`        | Pending room invites.                                                                                                            | Matrix invite room list.                                                                 |
| `agentApprovalCount` | Pending agent approvals that require user action.                                                                                | Currently notification-time detection only; durable count source is not implemented yet. |

## Badge Formulas

Desktop app badge:

```text
appBadgeCount = highlightCount + unreadCount + laterActiveCount
```

This preserves existing macOS/Linux behavior. Invites and agent approvals are
defined in the summary contract but are not currently part of the desktop app
badge.

Inbox badge:

```text
inboxBadgeCount = laterActiveCount + inviteCount + agentApprovalCount
```

The current desktop sidebar already shows invites plus Later. Agent approval
count is included in the contract so iOS and future desktop work have a stable
place to add a durable approval inbox count.

## System Notification Payload

`SystemNotificationRequest` is the portable delivery request:

```ts
type SystemNotificationRequest = {
  title: string;
  body?: string;
  route?: string;
  privacy?: 'standard' | 'private';
  sound?: 'default' | 'silent';
};
```

Rules:

- `title` is required, trimmed, and capped at 120 characters.
- `body` is optional, trimmed, and capped at 1,000 characters.
- `route` must pass the [Synara Route Contract](./synara-route-contract.md).
- `privacy` defaults to `standard`.
- `sound` defaults to `default`.
- Payloads must not include access tokens, device tokens, APNs tokens,
  decrypted message bodies in routes, or remote URLs as routes.

## Fixtures

### Active Later And Room Unread

Input:

```json
{
  "unreadCounts": [{ "total": 4, "highlight": 2 }, { "total": 3 }],
  "laterActiveCount": 5,
  "inviteCount": 2,
  "agentApprovalCount": 1
}
```

Expected summary:

```json
{
  "appBadgeCount": 10,
  "inboxBadgeCount": 8,
  "laterActiveCount": 5,
  "inviteCount": 2,
  "agentApprovalCount": 1,
  "highlightCount": 2,
  "unreadCount": 3
}
```

### Clamping

Input:

```json
{
  "unreadCounts": [{ "total": -1, "highlight": -2 }, { "total": 3.9 }],
  "laterActiveCount": 2.8,
  "inviteCount": -1,
  "agentApprovalCount": null
}
```

Expected summary:

```json
{
  "appBadgeCount": 5,
  "inboxBadgeCount": 2,
  "laterActiveCount": 2,
  "inviteCount": 0,
  "agentApprovalCount": 0,
  "highlightCount": 0,
  "unreadCount": 3
}
```

## iOS Notes

- iOS should recompute exact context after sync rather than trusting APNs
  payloads for room names, event text, or decrypted previews.
- APNs badge updates should use the same summary formula once the app has a
  synced local view.
- Generic push payloads should route to the safest destination when an event or
  room anchor is unavailable.

## Acceptance Criteria

- Runtime tests cover the app badge formula, inbox count formula, and clamping.
- Desktop badge behavior remains unchanged.
- Future iOS notification summary tests can use the fixtures above as
  conformance examples.
