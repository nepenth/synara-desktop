# Synara Later Account Data Contract

Reviewed: 2026-05-25

Status: initial shared contract with runtime normalization in
`src/app/utils/later.ts` and account-data type definitions in
`src/types/matrix/accountData.ts`. The canonical writer schema and fixtures
live under `docs/contracts/`.

## Purpose

Later is Synara's per-user saved/reminder inbox. It must sync across macOS,
Linux, and future iOS clients without storing decrypted message previews in
Matrix account data. The durable state is a small set of room/event anchors plus
workflow timestamps.

## Account Data Event

Later state is stored in global Matrix account data:

```text
in.synara.later
```

Account data is server-side per-user metadata. Clients must treat it as private
workflow state, but not as an encrypted message-preview store.

## Payload Model

Machine-readable artifacts:

- [synara-later-content.schema.json](./contracts/synara-later-content.schema.json)
- [synara-later-content.json fixtures](./contracts/fixtures/synara-later-content.json)

The JSON Schema defines the canonical v1 writer payload. Readers still apply
the normalization behavior documented here for older or non-canonical account
data.

```ts
type SynaraLaterContent = {
  version?: 1;
  items?: Record<string, SynaraLaterItem>;
};

type SynaraLaterItemKind = 'saved' | 'reminder';

type SynaraLaterItem = {
  id: string;
  kind: SynaraLaterItemKind;
  roomId: string;
  eventId: string;
  createdAt: number;
  dueTs?: number;
  remindedAt?: number;
  completedAt?: number;
};
```

The current payload version is `1`. Missing or invalid top-level content
normalizes to:

```json
{
  "version": 1,
  "items": {}
}
```

## Stable Item ID

The canonical item ID is:

```text
<roomId>\n<eventId>
```

This gives Synara one Later item per Matrix event in a room. Writers must store
the item under the same key as `item.id`. Readers should tolerate legacy
well-formed items during normalization, but any rewritten item must use the
canonical key.

## Field Rules

| Field         | Required | Rule                                                                          |
| ------------- | -------- | ----------------------------------------------------------------------------- |
| `id`          | yes      | String. Canonical writers use `<roomId>\n<eventId>`.                          |
| `kind`        | yes      | Either `saved` or `reminder`. Unknown values are invalid.                     |
| `roomId`      | yes      | Matrix room ID anchor.                                                        |
| `eventId`     | yes      | Matrix event ID anchor.                                                       |
| `createdAt`   | yes      | Finite Unix epoch milliseconds for when the Later item was created.           |
| `dueTs`       | no       | Finite Unix epoch milliseconds for reminder due time.                         |
| `remindedAt`  | no       | Finite Unix epoch milliseconds for when the current due reminder was emitted. |
| `completedAt` | no       | Finite Unix epoch milliseconds for completion/archive state.                  |

Invalid required fields drop the whole item. Invalid optional timestamp fields
are omitted. Unknown fields are ignored by v1 readers.

## Privacy Rules

Later account data must not store:

- Decrypted message bodies or previews.
- Sender display names or room display names.
- Access tokens, device tokens, APNs tokens, recovery keys, or secret storage
  material.
- Remote URLs copied from message content.
- Rendered HTML or markdown generated from message content.

Clients render Later rows by resolving `roomId` and `eventId` through the local
Matrix SDK store and decrypting locally when available. If the event is not
available or cannot be decrypted, the client should show a generic anchored
state rather than writing a preview into account data.

## Behavior

- `saved` marks an item as saved for later without requiring a due time.
- `reminder` marks an item with reminder semantics; it should normally include
  `dueTs`.
- Setting `completedAt` moves the item out of active Later counts.
- Snoozing a reminder sets a new `dueTs` and clears `remindedAt` and
  `completedAt`.
- Clearing completed items deletes items with `completedAt`.
- Same-client writes must serialize read-modify-write account-data updates so
  concurrent local actions do not overwrite each other.

Sorting for current v1 UI:

1. Active items before completed items.
2. Due/overdue reminders before undated saved items.
3. Earlier due timestamps before later due timestamps.
4. Newer `createdAt` first when due ordering is otherwise equal.

The shared due summary is:

```ts
type LaterDueSummary = {
  active: number;
  completed: number;
  overdue: number;
  dueToday: number;
};
```

## Fixtures

### Saved Item

Input:

```json
{
  "version": 1,
  "items": {
    "!room:example.org\n$event": {
      "id": "!room:example.org\n$event",
      "kind": "saved",
      "roomId": "!room:example.org",
      "eventId": "$event",
      "createdAt": 1770000000000
    }
  }
}
```

Expected normalized content preserves the item unchanged.

### Reminder Item

Input:

```json
{
  "version": 1,
  "items": {
    "!room:example.org\n$reminder": {
      "id": "!room:example.org\n$reminder",
      "kind": "reminder",
      "roomId": "!room:example.org",
      "eventId": "$reminder",
      "createdAt": 1770000000000,
      "dueTs": 1770003600000
    }
  }
}
```

Expected normalized content preserves `dueTs`.

### Completed Item

Input:

```json
{
  "version": 1,
  "items": {
    "!room:example.org\n$done": {
      "id": "!room:example.org\n$done",
      "kind": "saved",
      "roomId": "!room:example.org",
      "eventId": "$done",
      "createdAt": 1770000000000,
      "completedAt": 1770007200000
    }
  }
}
```

Expected normalized content preserves `completedAt`, and notification summaries
do not count this item as active.

### Legacy Plaintext Fields

Input:

```json
{
  "version": 1,
  "items": {
    "legacy": {
      "id": "legacy",
      "kind": "saved",
      "roomId": "!room:example.org",
      "eventId": "$event",
      "createdAt": 1770000000000,
      "body": "do not keep this",
      "sender": "@alice:example.org"
    }
  }
}
```

Expected normalized item:

```json
{
  "id": "legacy",
  "kind": "saved",
  "roomId": "!room:example.org",
  "eventId": "$event",
  "createdAt": 1770000000000
}
```

### Malformed Items

These items must be dropped:

```json
{ "id": "bad-kind", "kind": "todo", "roomId": "!room", "eventId": "$event", "createdAt": 1 }
```

```json
{ "id": "missing-event", "kind": "saved", "roomId": "!room", "createdAt": 1 }
```

```json
{ "id": "bad-created", "kind": "saved", "roomId": "!room", "eventId": "$event", "createdAt": "1" }
```

Non-finite optional timestamps must be omitted, not coerced.

## iOS Notes

- Swift decoding should implement the same fail-closed rules and fixture
  expectations before the iOS UI reads Later account data.
- iOS should route taps through the shared route contract using the room/event
  anchor, then resolve the event locally.
- iOS notifications for Later reminders should mark `remindedAt` only after the
  local notification has been scheduled or delivered according to the platform
  policy.
- Background refresh must not write decrypted previews into account data.

## Acceptance Criteria

- Runtime tests cover stable anchor IDs, no plaintext preview persistence,
  malformed item rejection, optional timestamp normalization, write
  serialization, completion, snoozing, clearing, sorting, and due summaries.
- The iOS project spec references this contract for Later read/write behavior.
- Future Swift conformance tests can use the fixtures above without relying on
  React or Tauri implementation details.
