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

Room-note items are one of:

- `note`: a local text note tied to a room.
- `todo`: a local task tied to a room, optionally completed and ordered.
- `message`: an anchor to a Matrix event, with optional bounded display helper
  fields.

Writers must keep note/todo body text bounded and must not store access tokens,
device credentials, media authentication headers, or other platform secrets.
Message items should be treated as anchors first; native clients should resolve
current event state from Matrix sync instead of trusting stored previews.

Schema and fixtures:

- `docs/contracts/synara-room-notes-content.schema.json`
- `docs/contracts/fixtures/synara-room-notes-content.json`
