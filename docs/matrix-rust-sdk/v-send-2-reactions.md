# V-SEND.2 — native reaction ownership

| Field | Value |
| --- | --- |
| Status | Candidate — not yet merged or runtime-proven |
| Queue position | `V-SEND.2`; `V-SEND.1` remains attachments/media upload |
| Owner | Managed Rust `NativeTimelineRegistry` and the active native client |
| JS fallback | None |

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
- Post-V-AUTH rebase inventory records the exact candidate delta: desktop
  runtime production importers **197 → 197** (and repository-wide importers
  **211 → 211**). This capability slice deletes no whole JS SDK importer file.
  Its physical owner deletion is instead measured by the scoped JS SDK
  method-candidate counts: `sendEvent` **8 → 6**, `redactEvent` **5 → 3**, and
  `getUnfilteredTimelineSet` **8 → 6**. The direct-import delta is therefore
  honestly **zero**.
- Focused Rust schema/validation tests, `cargo check`, `cargo fmt --check`,
  modernization TypeScript checking, scoped ESLint/Prettier, inventory tests,
  and Matrix Rust guardrails pass.
- Runtime proof is **Not confirmed**: an authenticated desktop/Synapse run must
  exercise each path and read the resulting native aggregation before merge
  acceptance. Compile/test success alone is not that proof.

This ledger was regenerated after rebasing onto integration `a084cbc`, which
includes the accepted V-AUTH merge `08a185e`. Do not treat the rebase or
inventory result as a substitute for runtime proof.
