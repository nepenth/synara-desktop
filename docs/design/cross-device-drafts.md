# Cross-device draft sync (design)

Status: **approved (option B + A below); Phase 1 implemented.** See
"Implementation notes" at the end for what shipped and what Phase 1 leaves out.

Goal: start typing in a room on one Synara device and keep going on another.
Unsent drafts sync between the user's own Synara clients (desktop and iOS),
end-to-end encrypted. Other Matrix clients ignore them.

Phase 1 (this design): draft text plus reply, thread and edit targets.
Phase 2 (separate PR): attachments (see the end of this doc).

## What exists today

- Desktop keeps per-composer drafts in `localStorage` under
  `in.synara.room_draft.<user>.<draftKey>` as Slate `Descendant[]` JSON
  (`synara/src/app/utils/drafts.ts`). `draftKey` is `roomId`, or
  `roomId#thread:<root>` for a thread composer. The cap is 64 KB.
- iOS keeps a markdown `String` per room in `DraftStore`
  (`synara-ios/Synara/Services/ComposerService.swift`), written after a 0.4 s
  pause.
- The Core already owns the reply draft, with a revision number
  (`nativeComposerDraft` / `SharedCoreComposerReplyDraft`).
- `crates/synara-core/src/app/account_data/` already has codecs and RMW for
  Synara's account data (`in.synara.room_notes`, `in.synara.later`, …). It also
  has a wakeup signal (`NativeAccountDataWakeupKind`) that tells the shells
  when synced account data changed. None of that data is encrypted.
- Sliding sync's `account_data` extension (enabled by `RoomListService`)
  delivers room account data. `room_list/live.rs` already registers an
  `AnyRoomAccountDataEvent` handler.

So the two shells store drafts in different shapes: rich Slate on desktop,
markdown on iOS. The wire format has to carry both (see "Payload").

## Storage

One **room account data** event per room:

```
PUT /_matrix/client/v3/user/{userId}/rooms/{roomId}/account_data/in.synara.draft
```

The event type follows the repo's existing `in.synara.*` namespace, not
`com.synara.*`. Room account data is private to the user and syncs to all
their devices. One event holds every draft slot for that room (the main
composer plus each thread composer), so a room never needs more than one
event type.

Account data can't be deleted on current Synapse (MSC3391 isn't
available), so "no draft" is written as an encrypted tombstone.

### Envelope (what the server sees)

```json
{
  "v": 1,
  "alg": "in.synara.draft.v1.xchacha20poly1305",
  "key_id": "base64(8 bytes)",
  "nonce": "base64(24 bytes)",
  "ciphertext": "base64(...)"
}
```

- **Cipher:** XChaCha20-Poly1305 AEAD (`chacha20poly1305` is already in
  `Cargo.lock`), with a random 192-bit nonce on every write. This is simpler
  than SSSS's AES-CTR + HMAC construction and has no nonce-reuse risk.
- **AAD** = `"in.synara.draft.v1" || user_id || room_id`. A server that moves a
  ciphertext to another room, or to another user, fails authentication.
- **key_id** = the first 8 bytes of `HMAC-SHA256(key, "in.synara.draft.key_id")`.
  It lets a device tell "encrypted with a key I don't have" apart from
  "corrupt".
- **Padding:** the plaintext is padded to a 512-byte bucket, so the server
  learns that a room has a draft and when it changed, but not how long the
  draft is.

### Payload (plaintext, after decryption)

```json
{
  "v": 1,
  "slots": {
    "main": {
      "updated_at": 1730280000123,
      "device_id": "ABCDEFGH",
      "cleared": false,
      "markdown": "hello **world**",
      "desktop_slate": [ ... ],          // optional, desktop only
      "reply_to": "$event",              // optional
      "edit_of": "$event",               // optional
      "truncated": false
    },
    "thread:$root": { ... }
  }
}
```

- `markdown` is the canonical form that every client can read. Desktop adds
  `desktop_slate` so desktop↔desktop keeps the full rich state, and prefers it
  when present. iOS reads `markdown` and ignores `desktop_slate`. Desktop
  turns `markdown` into Slate when only that field exists (the draft was
  written on iOS).
- The thread root is part of the slot key (`thread:<root>`), the same way as
  desktop's `draftKey`.
- `cleared: true` is a tombstone. The text fields are removed, and
  `updated_at`/`device_id` are kept so the merge rule still works.

## Key: creation, delivery, lifecycle

The drafts key is a random 32-byte secret that the user's verified devices
share. Each device caches it locally as a custom value in its own store. The
SQLite store is already passphrase-encrypted, the same protection the Olm
keys have. A copy is kept in secret storage (SSSS) under a custom name,
`in.synara.drafts.v1`, using the SDK's public
`SecretStore::{get_secret, put_secret}` (matrix-sdk 0.19.1).

**Creation (revised in 3.3.1):** draft sync needs a verified device, not the
recovery key. A cross-signing-verified device that has no key asks the user's
other verified devices for it (see "Peer sharing" below). If there are none,
or none answers within 10 seconds, it creates the key itself. Until 3.3.1 the
key was created only when a device opened secret storage, so an account whose
devices were all verified by emoji or QR code never got one and draft sync
stayed off without saying why.

**One key (convergence):** two devices can each create a key, for example
when both are offline from each other. Every place that sees two keys keeps
the same one: the higher *generation* wins, then the lower `key_id`. A
device that holds a key announces it once per session and whenever it
changes. A peer whose key wins answers with it; a peer whose key loses asks
back and gets the winner. Drafts sealed under the losing key read as absent
and are replaced by the next write (each device still has its own local
draft).

**Secret storage:** whenever Synara holds an open secret store (bootstrap,
unlock with the recovery key or passphrase, or reset in
`app/secret_storage/live.rs`), it merges `in.synara.drafts.v1` with the
cached key by the same rule and writes the winner back when the store holds
another key or none. A new device that signs in with the recovery key and
has no peer online therefore starts with the stored key.

**Rotation:** none during normal use. Drafts are short-lived, and the key
shares the trust domain of the recovery key that already protects all message
history.

**Secret-storage reset** (new recovery key): the SDK's `reset_key` re-exports
only its own `KNOWN_SECRETS`, so a custom secret would be lost. Synara's reset
flow therefore re-`put`s the cached drafts key into the new store, so draft
sync keeps working across a reset. A **full identity reset**
(cross-signing reset, which suggests a compromise) generates a new drafts key
on purpose. The resetting device forgets its key and raises its generation
floor to one above the key it held. It never adopts a key below the floor,
and the key it creates next has the higher generation, so it replaces the
old key on the other devices once they are verified again. Drafts under the
old `key_id` become unreadable. They're treated as absent and overwritten on
the next write. Generation 0 keys keep the original stored forms (32 cache
bytes, plain base64 in secret storage); a higher generation is appended
(4 more cache bytes, `<base64>.<generation>` in secret storage).

**Device without the key:** it neither reads nor writes synced drafts. It
keeps using its local drafts exactly as today. It never overwrites remote
drafts, because it can't merge with data it can't read. A verified device is
in this state for at most the 10-second wait; an unverified device stays in
it until it is verified (`keyState: needsRecovery`).

### Decision: devices that never type the recovery key (B + A chosen)

The SDK only gossips its built-in secrets between devices
(`Store::export_secret` returns `None` for any other name, see
`matrix-sdk-crypto` `store/mod.rs`). A phone that was **verified by emoji or QR
code** receives the cross-signing and backup keys, but never opens secret
storage, so it never receives `in.synara.drafts.v1`. That's probably the most
common setup, and draft sync would silently not work there.

Options:

- **A. Prompt once.** A device without the key shows "Enter your recovery key
  to sync drafts" (in Settings, and as a one-time hint). It needs no new
  protocol, but adds friction on exactly the devices that matter.
- **B. Synara peer sharing (recommended).** A device with the key sends it
  over an Olm-encrypted to-device event (`in.synara.drafts.key.share`), and
  only to the user's **own, cross-signing-verified** devices that ask
  (`in.synara.drafts.key.request`). This is the same trust rule the SDK uses
  for gossiping the backup key. It needs the matrix-sdk feature
  `experimental-send-custom-to-device`
  (`Encryption::encrypt_and_send_raw_to_device`) and a handler for decrypted
  custom to-device events. Option A remains the fallback when no other device
  is online.
- **C. Derive the key from the backup decryption key** (HKDF over
  `m.megolm_backup.v1`, which every recovered or verified device already
  holds). That needs no new protocol and gives automatic delivery, but
  matrix-sdk 0.19.1 doesn't expose the backup key publicly
  (`olm_machine()` is `pub(crate)`), and the key would change whenever key
  backup is reset. It isn't viable without patching the SDK.

Recommendation: **B + A** in Phase 1.

**Revised in 3.3.1:** B alone could not start, because it only copies a key
and nothing created one without the recovery key. Verified devices now create
it (see "Creation" above), which makes A unnecessary: the recovery key is
only one of the ways to verify a device.

### Peer sharing (wire format)

Both events are Olm-encrypted to-device events between the user's own
devices. Each side checks that the other is the same account, on a device
cross-signing vouches for.

- `in.synara.drafts.key.request`:
  `{ request_id, requesting_device_id, key_id?, generation? }`. `key_id` and
  `generation` describe the key the sender already holds and are absent when
  it holds none. A receiver without a key ignores it. A receiver whose key
  wins (or any key, when the sender has none) answers with a share. A
  receiver whose key loses sends its own request, at most once a minute.
- `in.synara.drafts.key.share`: `{ request_id, key, generation? }`. It is
  believed only as an answer to a request this device sent. The receiver
  adopts the key when it wins over its own and is not below its generation
  floor. If its own key wins instead, it sends a request so the peer learns
  that.

Synara 2.9.1 to 3.3.0 answers requests and never sends one while it holds a
key, so such a device keeps its key even when a newer device's key wins.
Devices settle once they all run 3.3.1 or later.

## Sync and merge

- **Writes:** debounced 1 s after typing stops, with at most one PUT per room
  every 5 s while typing continues. Writes are flushed immediately on room
  switch, composer blur, desktop window hide/close, and iOS `scenePhase` →
  background (inside a background task). The Core does the work: it merges
  the slot into the latest cached payload for that room, encrypts, and PUTs.
- **On send:** once the send is accepted (local echo queued), write a
  `cleared` tombstone for that slot.
- **Last writer wins,** per slot, ordered by `(updated_at, device_id)`. To
  absorb clock skew, a write uses
  `updated_at = max(now, highest_remote_updated_at_seen_for_slot + 1)`
  (a Lamport-style clock), so a device whose clock runs slow can still win.
- **Applying a remote slot:**
  - Remote is not newer: ignore it. Writes echoed back from this device are
    recognised by `device_id` plus `updated_at`.
  - Composer is idle (not focused, or unchanged since the last apply/save):
    replace its contents silently.
  - Composer is being typed in (dirty since the last save, or focused with a
    keystroke in the last ~3 s): keep the local text and show **"Draft updated
    on another device · Use it / Keep mine"**. "Keep mine" writes the local
    text, which then becomes newest.
  - Remote is a tombstone and the composer is dirty: keep the local text with
    no prompt (another device sent, and this one is typing something new).
  - The local text's age is unknown (iOS keeps no stamp for a draft restored
    from disk until it is first saved or applied): prompt rather than replace,
    and never clear it on a tombstone.
- **Opening a room** reads the cached account data. If the room has been
  outside the sliding-sync window, the Core also does one fresh GET, the same
  way room notes do (`fetch_fresh_room_notes_content`).

## Size limit (64 KB)

The cap is a 46 KB plaintext payload: padded, sealed and base64-encoded it
stays under 64 KB. When the slot being written doesn't fit:

1. drop its `desktop_slate`;
2. prune tombstones older than 30 days;
3. cut its `markdown` at a character boundary and set `truncated: true`. The
   receiving device shows "too long to sync in full" in its prompt.

The writing device keeps its full local draft. Other devices' slots are never
cut to make room; only if they alone overflow (a peer bug) are the oldest
dropped.

## Core API (shared by both shells)

In `crates/synara-core/src/app/drafts/`:

- `mod.rs`: the pure model: payload normalization, last-writer-wins merge,
  Lamport stamp, size cap, and the keep/apply/prompt decision.
- `codec.rs`: the envelope (XChaCha20-Poly1305, AAD, padding, `key_id`).
- `key.rs`: SSSS get/put, the local cache (a state-store custom value), the
  identity-reset hook, and peer sharing (option B).
- `live.rs`: `DraftSyncOwner`: debounced writes, fresh-read RMW, the sync
  handler, the to-device handler and wakeups.

The drafts owner lives inside the account-data owner (`NativeImagePackOwner`),
so it shares that owner's session lifetime without a new entry in the owner
attach order. Typed Core methods (`core/drafts.rs`) back both shells:

| Desktop command / UniFFI method | Purpose |
| --- | --- |
| `matrix_drafts_status` / `draftsStatus()` | `{ enabled, keyState: ready \| requesting \| needsRecovery \| disabled }`; asks peers when the key is missing and creates it when nobody has one. `needsRecovery` means the device is not verified |
| `matrix_drafts_set_enabled` / `draftsSetEnabled` | the setting |
| `matrix_drafts_save` / `draftsSave` | queue (or `flush`) a slot; returns its stamp |
| `matrix_drafts_clear` / `draftsClear` | tombstone a slot now (send or erase) |
| `matrix_drafts_resolve` / `draftsResolve` | compare with the composer's `{ updatedAt, dirty, markdown }` → `keep \| apply \| prompt` |
| `matrix_drafts_delete_all` / `draftsDeleteAll` | tombstone every live slot |
| `matrix_drafts_flush` / `draftsFlush` | write everything queued (background, quit) |

Changes arrive as the `matrix-drafts-updated` Tauri event on desktop and the
`drafts` owner-update family on iOS, each carrying the room ID (or none,
meaning re-read every composer, e.g. when the key arrives). Shells then call
`resolve` for their open composer.

Local drafts stay the source of truth for the composer on each shell. The
Core layers sync on top, so turning sync off changes nothing locally.

## Setting

**"Sync drafts across devices"**, on by default, stored per account on each
device (in Settings → Privacy on both shells). Off means no reads, no writes
and no key requests on that device. Turning it off doesn't delete remote
drafts; a secondary "Delete synced drafts" button tombstones every slot.
Signing out removes the local key cache along with the rest of the store.

## Tests

- **Core unit:** AEAD round trip. Wrong key, wrong room, wrong user and
  tampered ciphertext all fail closed. Unknown `key_id` is reported as "no
  key". LWW, including a skewed clock and the Lamport bump. Tombstone versus
  dirty composer. Echo suppression. Size cap order (drop Slate, then
  truncate). Payload normalization rejects malformed input.
- **Key lifecycle (Core, mocked SSSS):** create on bootstrap, reuse on unlock,
  carried across a secret-storage reset, regenerated on identity reset.
- **Synapse live proof** (the local harness in `integration/`, if it can run
  two clients): device A writes, device B decrypts. A send on A clears B.
  Peer key share to a verified device works, and an unverified device is
  refused.
- **Shells:** desktop tests for the Slate↔markdown fallback and the prompt
  state. An iOS `ComposerService` test for apply versus prompt.

## Phase 2: attachments (outline only)

Pending attachments are uploaded with the existing encrypted-attachment
flow. The slot then holds `attachments: [{ file: EncryptedFile, info, name }]`
inside the already-encrypted payload, so the media key is never visible to
the server. Risks:

- Synapse may delete unreferenced media (`media_retention` /
  unused-media purge). A missing media fetch shows "Attachment no longer
  available" and drops that entry.
- Upload quota. The per-draft cap (10 attachments, 50 MB in total) needs
  confirming.

That work will get its own short design addendum.

## Privacy and security notes

- The server learns which rooms have a draft, when it changed, and its size
  bucket. It never learns content, reply or edit targets, or attachment keys.
- The key never leaves the user's own verified devices and their secret
  storage.
- Nothing about drafts is logged beyond the room ID and the outcome codes
  (the static `&'static str` error style used across `account_data`).

## Implementation notes (Phase 1)

What shipped:

- Core: everything above, with unit tests for the envelope (round trip, fresh
  nonces, padding, wrong key/room/user, tampering, version), normalization,
  merge, Lamport stamps, tombstone pruning, the size cap and every decision
  branch.
- Key: created by a verified device that finds no peer holding one, merged
  with secret storage whenever Synara opens it (bootstrap, unlock,
  recovery-key reset), forgotten on identity reset. Peer sharing runs over
  Olm to-device events. It answers only the user's own
  cross-signing-verified devices, and only adopts an answer to a request this
  device sent. `experimental-send-custom-to-device` was already enabled
  through `experimental-widgets`.
- Desktop: `RoomInput` mirrors the main and thread-pane composers through
  `useSyncedRoomDraft`. Text, the Slate value and the reply target are sent.
  A peer's reply target is applied on desktop. The setting is in
  Settings → General.
- iOS: the room composer mirrors its markdown and reply target through
  `ComposerDraftSync`. It flushes on disappear and background. The setting is
  in Settings → Security & Recovery, because whether it works depends on
  recovery.

Left out of Phase 1, on purpose:

- iOS thread composers don't sync yet, and iOS doesn't apply a peer's reply
  target (that needs the event loaded to build a `ComposerRelationTarget`).
- Edit targets (`edit_of`) are carried by the format, but neither shell
  writes them. An in-progress edit is never synced as a draft.
- Desktop rich formatting reaches iOS as Markdown in the iOS composer's own form
  (`toDraftMarkdown`): mentions as matrix.to Markdown links, underline and spoilers as
  `<u>` and `<span data-mx-spoiler>`, custom emoji as `:shortcode:`. Text typed with
  Markdown off is escaped. iOS syncs its draft with picked mentions resolved to the same
  links. A Markdown draft from iOS is rendered by Core on a desktop with Markdown off.
  Desktop-to-desktop keeps the full Slate value.
- No "Delete synced drafts" button. The Core command exists. Both shells
  show a hint for `keyState` under the setting.
- No Synapse two-client live proof yet. The Core owner is covered by unit
  tests, and its I/O path by the shell builds only. The peer exchange is
  tested through its decision functions (`key_tests.rs`), not over two real
  Olm sessions.
