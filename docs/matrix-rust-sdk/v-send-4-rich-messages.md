# V-SEND.4 — native rich composer message ownership

| Field          | Value                                                    |
| -------------- | -------------------------------------------------------- |
| Status         | Draft [#253](https://github.com/nepenth/synara-desktop/pull/253) — runtime proof pending |
| Queue position | After V-SEND.3 polls                                     |
| Owner          | Managed Rust client via extended `matrix_send_text`      |
| JS fallback    | None for composer messages on a desktop native session   |

## Scope and deleted owner

This vertical extends the existing native composer send owner to preserve:

- `m.text`, `m.emote`, and `m.notice` message types;
- `org.matrix.custom.html` formatted bodies;
- explicit user and room mentions; and
- optional reply relations.

`RoomInput` no longer routes emotes or notices to `mx.sendMessage`, and rich
text/mentions are no longer discarded by the native text route. Message edits,
forwards, stickers, GIFs, and threads remain separate residuals.

## Operating path

```text
RoomInput composer submit
  → sendPlainTextWithNativeOwner
  → matrix_send_text
  → validated RoomMessageEventContent type / HTML / mentions / reply
  → Room::send
```

The web and native logged-out routes retain the legacy owner. Once a native
session is logged in, command absence or failure is terminal and never falls
through to the JS Matrix client.

## Runtime proof

Authoritative gate: required CI job **Synapse native rich-message proof**
(`live_native_rich_message_send_against_disposable_synapse_when_configured`).
It sends an HTML emote with user and room mentions through the managed Rust
client, then fetches the event from disposable Synapse and verifies the wire
content.

Until that CI job is green on the reviewed SHA, runtime proof remains
**Not confirmed**.
