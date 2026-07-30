# V-SEND.2 — native reaction ownership

| Field          | Value                                                              |
| -------------- | ------------------------------------------------------------------ |
| Status         | Candidate — not yet merged or runtime-proven                       |
| Queue position | `V-SEND.2`; `V-SEND.1` remains attachments/media upload            |
| Owner          | Managed Rust `NativeTimelineRegistry` and the active native client |
| JS fallback    | None                                                               |

## Scope and deleted owners

This vertical replaces every active desktop JS reaction writer, rather than
adding a parallel reaction route:

- timeline reaction toggle;
- the idempotent approval-card action;
- the native-notification approval action; and
- annotation redaction from the reaction viewer.

The old `MatrixClient.sendEvent` / `redactEvent` relation writes and their local
reaction-aggregation decisions are deleted from those paths. The retained JS
timeline and reaction-rendering code is not a write owner. Generic message/call
redaction is outside this reactions slice and remains explicitly owned by its
future vertical.

The withdrawn `NativeRoomTimeline` flat shell is not a V-SEND.2 target and is
not wired to these commands. Its existence cannot provide a presenter, fallback,
or acceptance route for this vertical.

## Operating paths

### Toggle one reaction

```text
room timeline reaction control
  → matrix_timeline_reaction_toggle
  → NativeTimelineRegistry::toggle_reaction
  → matrix-sdk-ui Timeline::toggle_reaction
  → same native timeline aggregation readback
```

The actor starts with an authenticated native session and a rendered reaction on
a room event. The native registry validates room/event IDs and a bounded
reaction key, opens the managed timeline, and lets `matrix-sdk-ui` select the
single add-or-remove operation. The side effect is exactly one Matrix
annotation send or redaction; the authoritative readback is the aggregation
projected from that same registry. No WebView relation lookup, JS retry, or JS
SDK fallback is permitted.

### Ensure an approval reaction

```text
approval card or native-notification action
  → matrix_reaction_ensure
  → native aggregation readback
  → Client Room::send(ReactionEventContent), only if absent
  → same native aggregation readback
```

`ensure` is deliberately distinct from toggle: if the active user already has
the annotation, it returns `already_present` and performs no write. This keeps
the approval action idempotent without JS inspecting sender lists.

### Redact an annotation

```text
reaction viewer selection
  → matrix_reaction_redact
  → managed native Room::redact(reaction event ID)
  → same native aggregation readback
```

The webview supplies only the selected room, target event, annotation event ID,
and reaction key. Rust validates identifiers and performs the authoritative
room operation. Sender IDs and remote annotation IDs are projected only because
the existing reaction viewer needs them to display/select public room
annotations; no credentials, keys, raw event content, or raw SDK error crosses
the IPC boundary.

## Candidate evidence and limits

- Tauri command registration, capability allow-list, generated permissions, and
  generated ACL schemas agree for all three commands.
- Post-V-ROOMS.1 rebase inventory on integration tip `2c48fd4` records the exact
  candidate delta: desktop runtime production importers **194 → 194** (and
  repository-wide importers **208 → 208**). This capability slice deletes no
  whole JS SDK importer file. Its physical owner deletion is instead measured by
  the scoped JS SDK method-candidate counts: `sendEvent` **8 → 6**,
  `redactEvent` **5 → 3**, and `getUnfilteredTimelineSet` **8 → 6**. The
  direct-import delta is therefore honestly **zero**.
- Focused Rust schema/validation tests (reaction key bounds, event-id
  validation, mutation-readback privacy, annotation binding), injectable
  frontend owner-route tests for toggle/ensure/redact with no JS fallback,
  `cargo check`, `cargo fmt --check`, modernization TypeScript checking, scoped
  ESLint/Prettier, inventory tests, and Matrix Rust guardrails pass.
- Runtime proof authority for merge: required CI job
  **Synapse native reaction proof**
  (`live_native_reaction_paths_against_disposable_synapse_when_configured`)
  against the disposable Synapse harness. It registers/logs in with the managed
  Rust client, then exercises `NativeTimelineRegistry` toggle → ensure
  (idempotent) → redact → ensure re-add and reads native aggregation after each
  mutation. JS two-client Synapse CI is **not** this proof. WebView click-through
  is not required once that owner-route Synapse proof is green on the reviewed
  SHA; IPC command names remain covered by frontend owner-route tests.
- Until that CI job is green on the reviewed SHA, runtime proof remains
  **Not confirmed**. Owner-route unit tests and inventory are preservation
  evidence only.

This ledger was regenerated after rebasing onto integration `2c48fd4` (V-ROOMS.1
#241 merged). Keep #239 draft until the Synapse native reaction proof job is
green on the reviewed SHA.
