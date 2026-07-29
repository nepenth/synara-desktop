# V-ROOMS.1 — native invite triage and actions

| Field | Value |
| --- | --- |
| Status | Implementation candidate scoped; no product acceptance claimed |
| Owner | Managed Rust Matrix client and invite DTO boundary |
| Queue | `V-ROOMS.1`; native invite projection and UI |
| Policy | Complete native replacement of this vertical; no JS SDK fallback |

## Retained product contract

The Invite inbox must retain all three existing triage classes and their
actions. A native list that omits classification or bulk safety actions is not a
completed V-ROOMS.1 replacement.

| Class | Current decision | Required native owner |
| --- | --- | --- |
| Known | Inviter shares a joined room | Native joined-member projection |
| Public | No shared joined room | Native joined-member projection |
| Spam | Bad-word match or inviter is banned in any joined room | Native bad-word corpus plus joined-member projection |

The DTO must retain the bounded card data needed for the existing UI: room ID,
name, canonical alias, an opaque native avatar handle, topic, inviter ID/display
name, invite timestamp/reason, direct/space/encryption flags, sender-ignore
state, and triage class. It must not expose tokens, keys, raw state events, raw
SDK errors, arbitrary member lists, or an MXC URI. A raw `mxc:` URI alone is
insufficient: the existing card turns it into a usable image through the JS
Matrix client, so the native owner replaces that media-resolution step too.

## Operating paths

```text
Invite inbox mount or refocus
  → matrix_invites_snapshot
  → active native Client::rooms (invited + joined state-store view)
  → invite DTO/classification + opaque avatar capability
  → SDK-neutral React cards and counts
```

```text
Accept / decline / report / block control
  → one native invitation command
  → Room::join / Room::leave / Room::report_room / Account::ignore_user
  → native invite snapshot readback
```

Accepting a direct invitation also calls the typed native direct-account-data
owner (`Account::mark_as_dm`) after a successful join, preserving the existing
`m.direct` side effect. Batch controls may sequence native commands, but the
webview must never perform the Matrix mutation or inspect the JS SDK state.

```text
Invite card image request
  → synara-media://opaque-handle (main webview GET only)
  → current-generation native capability validation
  → Client::media thumbnail fetch (96 × 96, no durable cache)
  → bounded recognized image bytes in Tauri protocol response
```

The protocol never accepts an MXC URI, filesystem path, arbitrary URL, query,
or a generic media request. Handles are CSPRNG-generated, capped at 256,
revoked after the relevant invite action, and wiped on session retirement.
Image bytes are deliberately not JSON IPC. This narrow thumbnail path does not
close the broader Phase-7 media/cache risk (`R011`): the SDK retrieval API
buffers the thumbnail before the post-fetch size/type check, and no durable
media cache is introduced here.

## Locked SDK evidence

The locked Matrix Rust SDK 0.18 exposes typed APIs for every required write:

- `Room::join` and `Room::leave`;
- `Room::report_room` (valid for an invited room);
- `Account::ignore_user`; and
- `Account::mark_as_dm`.

No raw `/_matrix` request or SDK-gap exception is authorized for this vertical.

## Parity-sensitive implementation decisions

- Classify from the native state store without forcing membership synchronization:
  that matches the current JS product’s local-cache decision surface and avoids
  a hidden network fan-out merely to render the inbox.
- Vendor the exact 450-word, MIT-licensed `badwords-list` 2.0.1-4 corpus
  currently used by the desktop product, plus the two explicit Synara additions
  (`torture` and `t0rture`), with a native boundary matcher. A substitute
  profanity list is not parity evidence.
- Re-read the native invite snapshot after each successful command. A command
  success label alone is not acceptance evidence.
- Mint a per-session, opaque avatar handle at the native boundary. Direct-room
  selection follows the existing local-cache order (eligible heroes, cached
  members, room avatar); it is not reduced to the inviter or a generic fallback.
  Retaining `MatrixClient` merely to turn `mxc:` into an image URL is a
  disqualifying JS owner detour.

Runtime proof remains **Not confirmed** until an authenticated disposable
Synapse/desktop run creates representative Known, Public, and Spam invitations,
executes all actions, and reads the native snapshot after each side effect.

## Current evidence

- `cargo fmt --check`, `cargo check -p synara`, the invite-classifier unit test,
  and the three avatar-capability lifecycle tests pass on this candidate.
- Scoped frontend Prettier, ESLint, and `tsc --noEmit` pass. The Invite page,
  list binding, and obsolete invite-list hooks have no `matrix-js-sdk` import
  or JS room/action listener.
- The generated inventory measures **201 → 198** desktop-runtime production
  import files on this pre-V-AUTH.1 base. This is intentionally a branch-local
  measurement, not an assertion update: after V-AUTH.1 merges, this candidate
  must rebase and regenerate the final combined delta before its CI baseline is
  changed.
