# Synara Room Notes Contract

Reviewed: 2026-05-26

Room notes are stored in global Matrix account data under `in.synara.room_notes`.
The account data is private user state and must not be treated as room-visible
content.

Canonical payloads use version `1`:

```ts
type SynaraRoomNotesContent = {
  version: 1;
  rooms: Record<
    string,
    {
      items: Record<string, SynaraRoomNoteItem>;
    }
  >;
};
```

Live Core readers and writers fail closed when the event is missing a version
or carries a version other than `1`. They must not normalize a newer payload to
v1 and write it back. Malformed items inside a recognized v1 payload may still
be discarded or bounded according to the canonical schema.

The complete encoded account-data object is capped at 1 MiB. Oversized reads
and writes fail closed instead of parsing, truncating, or publishing a partial
replacement.

Every mutation validates its target before any server read/modify/write:
Matrix room IDs must use the `!` sigil, contain no whitespace, and be at most
255 UTF-8 bytes; item IDs must be nonempty and at most 256 Unicode scalar
values. These same bounds apply to upserted items and delete, complete, and
move targets. Invalid targets fail closed and are never silently truncated.

Room-note items are one of:

- `note`: a local text note tied to a room.
- `todo`: a local task tied to a room, optionally completed and ordered.
- `message`: an anchor to a Matrix event, with optional bounded display helper
  fields.

Writers must keep note/todo body text bounded and must not store access tokens,
device credentials, media authentication headers, or other platform secrets.
Message items should be treated as anchors first; native clients should resolve
current event state from Matrix sync instead of trusting stored previews.

Core serializes read-modify-write mutations within one running process and
fetches the current server value before each mutation; it does not rely on a
possibly stale `/sync` account-data cache after a write. Read-only snapshots
use the SDK's synchronized local store so desktop polling remains offline-safe
and does not issue a homeserver request every second. Matrix global account
data does not provide a conditional write, so version 1 is explicitly
last-write-wins across devices. A concurrent edit can replace another device's
add, edit, reorder, or deletion (there are no v1 tombstones). Retries are
idempotent only when they resend the same complete replacement. A future
storage version must define append/partition, tombstone, merge, offline retry,
and migration semantics before cross-device convergence can be claimed;
clients must not simulate that guarantee with retries over the same event.

After a successful local mutation, the live Core owner projects the returned
content for at most 30 seconds so a stale SDK account-data cache cannot erase
the acknowledged change in the UI before `/sync` catches up. A normalized sync
event equal to that content acknowledges it immediately. Differing sync content
cannot be ordered reliably against the PUT (the sync request may have already
been in flight), so it remains hidden only for that bounded window and then
surfaces as the account-data last-write-wins result. This favors immediate local
read-your-write behavior while bounding how long a genuine external update can
be delayed; it is not a cross-device merge or convergence guarantee.

Schema and fixtures:

- `docs/contracts/synara-room-notes-content.schema.json`
- `docs/contracts/fixtures/synara-room-notes-content.json`
