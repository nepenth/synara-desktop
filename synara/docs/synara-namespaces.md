# Synara Namespaces

Synara uses namespaced `in.synara.*` account data and event-content fields for client workflow features. These fields are private client metadata unless explicitly included in sent event content. The portable cross-platform inventory is [Synara Shared Contract Inventory](./synara-contracts.md).

## Account Data

- `in.synara.later`: per-user Later inbox anchors. Items store only `roomId`, `eventId`, timestamps, kind, due/reminded/completed state, and never plaintext message bodies. See [Synara Later Account Data Contract](./synara-later-contract.md).
- `in.synara.room_notes`: per-user room notes, to-dos, and message anchors. See [Synara Room Notes Contract](./synara-room-notes-contract.md).
- `in.synara.unread_anchor`: per-user message-level unread anchors. This is a private client marker and does not move public read receipts. See [Synara Unread Anchor Contract](./synara-unread-anchor-contract.md).
- `in.synara.spaces`: existing space organization metadata. See [Synara Spaces Contract](./synara-spaces-contract.md).

## Event Content Metadata

- `in.synara.forwarded`: attribution metadata added to forwarded messages. The visible message body also includes attribution so other clients are not required to understand this key.
- `in.synara.gif.*`: GIF metadata for provider/source attribution. GIF media itself is uploaded to the Matrix media repository and sent as `mxc://`.
- `in.synara.agent`: explicit structured agent-card payload key supported by the Hermes/agent card renderer. See [Synara Agent Card Contract](./synara-agent-card-contract.md).
- `in.synara.agent.action`: explicit approve/reject result payload embedded in
  an authenticated Matrix room event after user activation. See
  [Synara Agent Action Contract](./synara-agent-action-contract.md).

## Local-Only Storage

- `in.synara.room_draft.<userId>.<roomId>`: browser `localStorage` key for per-room composer drafts. Drafts are local to the device/browser profile, capped in size, and are not written to Matrix account data.
- `settings.customAccentColor`: browser `localStorage` setting for the optional constrained theme accent. It accepts only full hex colors and is not written to Matrix account data.
- `settings.themeBaseColor`: browser `localStorage` setting for the optional chrome hue tint. It accepts only full `#rrggbb` colors and is not written to Matrix account data. iOS stores the same key in app-group UserDefaults independently; there is no cross-device sync yet.

## Matrix-Native Events

- Polls are sent as `org.matrix.msc3381.poll.start` events with both stable and unstable poll content keys where practical. Votes are sent as `org.matrix.msc3381.poll.response` reference relations.

## Agent Card Config

Deployments can opt into additional explicit structured-card keys with:

```json
{
  "agentCards": {
    "contentKeys": ["org.hermes.agent", "io.hermes.agent", "in.synara.agent", "m.custom.agent"]
  }
}
```

Synara does not render arbitrary JSON messages as structured cards. JSON body rendering still requires an explicit `hermes: true` marker.

Agent payloads can include an optional bounded `actions` array. Each action supports a title/label, optional kind/type, optional safe HTTPS URL, and optional prompt/command fallback. URLs are filtered with the same browser-side remote-content privacy guard used for artifacts.
