# Synara Room/Event Anchor Contract

Reviewed: 2026-05-25

Status: initial shared contract with schema and fixtures under
`docs/contracts/`.

## Purpose

Room/event/thread anchors are opaque Matrix identifiers used to move between
Synara surfaces without storing decrypted content in routes, notifications,
Later account data, or agent workflow state. Native platforms should resolve
display names, previews, and decryption state locally after sync.

## Machine-Readable Artifacts

- [synara-room-event-anchor.schema.json](./contracts/synara-room-event-anchor.schema.json)
- [synara-room-event-anchor.json fixtures](./contracts/fixtures/synara-room-event-anchor.json)

## Payload Model

```ts
type SynaraRoomEventAnchor = {
  roomIdOrAlias: string;
  eventId?: string;
  threadRootEventId?: string;
  parentSpaceIdOrAlias?: string;
};
```

## Rules

- `roomIdOrAlias` is required and must be a Matrix room ID or room alias.
- `eventId`, when present, must be an opaque Matrix event ID.
- `threadRootEventId`, when present, must be an opaque Matrix event ID.
- `parentSpaceIdOrAlias`, when present, must be a Matrix room ID or room alias.
- Anchors must not include message bodies, sender display names, room display
  names, homeserver credentials, device tokens, access tokens, APNs tokens, or
  media URLs.
- Platforms may encode anchors into route paths or native navigation state, but
  must preserve the same fields and fail closed on malformed input.

## iOS Notes

- Swift route and Later services should decode anchors before navigation.
- Timeline screens should resolve anchors through the Matrix SDK store after
  sync, not by trusting notification or account-data previews.
- If the event is unavailable, render a recoverable unavailable state in the
  containing room.

## Acceptance Criteria

- Fixtures cover room-only, room/event, threaded event, and parent-space scoped
  anchors.
- Invalid anchors with plaintext previews, missing room anchors, or malformed
  Matrix identifiers are rejected by conformance tests.
