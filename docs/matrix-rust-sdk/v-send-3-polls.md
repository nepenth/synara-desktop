# V-SEND.3 — native poll start / response ownership

| Field          | Value                                                              |
| -------------- | ------------------------------------------------------------------ |
| Status         | Candidate — not yet merged or runtime-proven                       |
| Queue position | After V-SEND.1 attachments and V-SEND.2 reactions                  |
| Owner          | Managed Rust client via `matrix_send_poll` / `matrix_poll_respond` |
| JS fallback    | None on desktop native session                                     |

## Scope and deleted owners

This vertical replaces active desktop JS poll writers:

- composer UploadBoard poll create (`RoomInput`);
- `/poll` slash command (`useCommands`);
- poll vote (`PollContent` response send).

Retained JS poll helpers (`polls.ts` parse/normalize/summarize) and
`PollContent` relation-based response **read** remain until a later timeline
poll-projection UI slice. Stickers/GIF/forward/`SendRoomEvent` are out of scope.

Does **not** select `NativeTimelinePresenter` or delete `RoomTimeline.tsx`.

## Operating paths

### Create a poll

```text
composer poll board or /poll command
  → matrix_send_poll
  → normalize + UnstablePollStartEventContent (disclosed)
  → Room::send
```

### Vote

```text
PollContent option click
  → matrix_poll_respond
  → UnstablePollResponseEventContent (m.reference)
  → Room::send
```

## Runtime proof

Authoritative gate: required CI job **Synapse native poll proof**
(`live_native_poll_send_and_respond_against_disposable_synapse_when_configured`)
against disposable Synapse. JS two-client Synapse CI is **not** this proof.

Until that CI job is green on the reviewed SHA, runtime proof remains
**Not confirmed**.

## Inventory

Rebased on integration `90be0f4` (V-SEND.1 merged):

- Production importers **189 → 189**; repository-wide **202 → 202**.
- Scoped method-candidate deletion: poll `sendEvent` writers removed from
  `RoomInput`, `useCommands`, and `PollContent` (sticker/`SendRoomEvent`/call
  widget residual remain).
