# Synara Unread Anchor Contract

Reviewed: 2026-05-26

Unread anchors are stored in global Matrix account data under
`in.synara.unread_anchor`. They are a private per-user helper for returning to
the message that Synara considers the unread boundary.

Canonical payloads use version `1`:

```ts
type SynaraUnreadAnchorContent = {
  version: 1;
  anchors: Record<
    string,
    {
      eventId: string;
      ts: number;
    }
  >;
};
```

Keys are room IDs. Values store only an opaque Matrix event ID and timestamp.
Writers must not store message bodies, sender display names, decrypted previews,
or push notification text in this account data.

Unread anchors do not replace Matrix read receipts. Native platforms should use
them only as Synara-local navigation hints after Matrix sync has supplied the
room timeline.

Schema and fixtures:

- `docs/contracts/synara-unread-anchor-content.schema.json`
- `docs/contracts/fixtures/synara-unread-anchor-content.json`
