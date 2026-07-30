# Continuation — full-vertical Matrix Rust replacement

> **Current continuation card.** Full history remains in
> [PROGRESS.md](PROGRESS.md); original plan inventory and strict gates remain in
> [program-status.md](program-status.md).

<!-- matrix-rust-program-status-link -->

| Field                   | Current value                                                                                                                                                                    |
| ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Date                    | 2026-07-30 (America/New_York)                                                                                                                                                    |
| Integration branch      | `feature/matrix-rust-sdk-full-replacement`                                                                                                                                       |
| Execution model         | Primary Codex and every implementation/review sub-agent: `gpt-5.6-sol`, medium reasoning                                                                                         |
| Current integration tip | `2c48fd45a08200a6e3491f100912f086e8458b3b` — V-ROOMS.1 [#241](https://github.com/nepenth/synara-desktop/pull/241) merged from green candidate `7ac2c48`                          |
| Active PRs              | Draft V-ROOMS.3 unread badges; [#239](https://github.com/nepenth/synara-desktop/pull/239) V-SEND.2; [#240](https://github.com/nepenth/synara-desktop/pull/240) V-TIMELINE |
| Policy                  | [full-vertical-policy.md](full-vertical-policy.md)                                                                                                                               |
| Operating path          | [operating-path-contract.md](operating-path-contract.md)                                                                                                                         |
| Binding queue           | [d0-residual-completion.md](d0-residual-completion.md)                                                                                                                           |
| Main merge              | PR #39 — explicit user approval required                                                                                                                                         |

## Active direction

Complete the desktop replacement as serial, product-visible verticals:

```text
React UI → versioned Tauri IPC + Synara DTOs → live Rust matrix-sdk
```

No incomplete/minimum/plateau acceptance, no backend selector, and no concurrent
JS/Rust clients for one session.

**Physical deletion happens per vertical.** A native path beside retained JS
ownership is “wired,” not “done.” Each capability slice must delete its
superseded `matrix-js-sdk` implementation, imports, compatibility branches, and
obsolete tests/types before closure.

## Exact continuation point

1. V-CRYPTO.7 [#236](https://github.com/nepenth/synara-desktop/pull/236) is merged at integration `528a510`; its reviewed, green product/test head was `192be46`. Live multi-session/UI proof remains unclaimed.
2. V-AUTH.1 [#238](https://github.com/nepenth/synara-desktop/pull/238) is merged: all desktop SSO entry points, callback/token-completion ownership, and native SSO UIAA continuation are deleted without a replacement route. Approved inventory: production importers **201→197**.
3. V-ROOMS.1 [#241](https://github.com/nepenth/synara-desktop/pull/241) is merged at integration `2c48fd45a08200a6e3491f100912f086e8458b3b`; candidate `7ac2c48` passed the required scope, Synapse, desktop/runtime, and quality jobs. Its measured deletion delta is production **197→194**, repository-wide **211→208**.
4. Start **V-ROOMS.3** native unread badges: sole `matrix_room_list_snapshot` owner drives `roomToUnreadAtom`; delete JS room-list/unread binders. Candidate inventory production **194→192**, repository-wide **208→205**.
5. Keep V-SEND.2 [#239](https://github.com/nepenth/synara-desktop/pull/239) at `d26db4c` draft and ordered despite green required CI (whole importers **197→197**; direct JS reaction owner candidates `sendEvent` **8→6**, `redactEvent` **5→3**, and `getUnfilteredTimelineSet` **8→6**). Native redaction verifies that the requested event is the selected target/key annotation; completion and live runtime proof remain unclaimed.
6. Continue V-TIMELINE [#240](https://github.com/nepenth/synara-desktop/pull/240) at `5e0c2a5` only through its full-replacement contract. It now provides native stream/session-bound opaque media delivery and image/file/audio/video/sticker presentation, but remains unselected and unaccepted pending fresh full CI and live proof. Do not inherit a green claim from pre-media `7e6a4d2`: run `30553357363` attempt 1 passed Synapse/iOS but hit the host-sensitive 119-row audit RSS cap, while attempt 2 found six React-hook lint errors. The latter are corrected in `5e0c2a5` only by focused local checks. Do not select the presenter or delete `RoomTimeline.tsx` until the full action/media route and runtime proof are complete.
7. Run V-BURN only as the final convergence audit and npm dependency/bootstrap/store cleanup.
8. Resume new media/widgets/notifications/calls verticals only after the residual queue allows.

## Current accounting

- D0.1–D0.5 established native login/session, sync/room list, basic timeline,
  plain-text send, and encrypted-room machine paths.
- V-CRYPTO.1 native verification is complete: its legacy owner/inbox/hooks/helpers
  and JS-only test are deleted, with direct desktop-runtime imports 232/292 → 223/280.
- V-CRYPTO.2 native cross-signing is complete: its legacy setup/status/reset owner,
  account-data fallback, and compatibility types are deleted, with direct
  desktop-runtime imports 223/280 → 222/279.
- V-CRYPTO.3 native backup is complete: its legacy status/restore UI, listeners,
  progress state, and automatic restore listener are deleted, with direct
  desktop-runtime imports 222/279 → 219/276.
- V-CRYPTO.4 native secret storage is complete: browser recovery derivation and
  checking, the account-data compatibility path, dead manual-verification UI,
  JS key cache, and JS-only test are deleted, with direct desktop-runtime imports
  219/276 → 218/275.
- V-CRYPTO.5 merged in #227 and is complete under the per-vertical deletion policy: the retained product path has one Rust IPC owner, and the legacy WebView owner/browser crypto helper are deleted.
- V-CRYPTO.6 [#235](https://github.com/nepenth/synara-desktop/pull/235) integrates P5.10/P8.7 into the managed native timeline,
  relies on SDK-owned pagination insertion/late-key redecryption and adds safe late-decrypt readback, reuses native
  recovery settings, and deletes the JS retry/per-event/listener owners. See
  [v-crypto-6-utd-recovery.md](v-crypto-6-utd-recovery.md).
- V-CRYPTO.7 merged in [#236](https://github.com/nepenth/synara-desktop/pull/236) at integration `528a510` (reviewed, green product/test head `192be46`): native device list/trust, rename, and
  purpose-specific other-device deletion/UIAA replace the device page's JS SDK
  owners. Direct inventory is **212 files / 265 import lines**, production
  importers **201**, and repository-wide importers **215**.
- V-AUTH.1 [#238](https://github.com/nepenth/synara-desktop/pull/238) is merged at integration `08a185e` after required CI passed. It deletes the complete desktop SSO surface with exact AST inventory **212 files / 265 import lines → 208 / 261**, production importers **201→197**, and repository-wide importers **215→211**.
- V-ROOMS.1 [#241](https://github.com/nepenth/synara-desktop/pull/241) is merged at integration `2c48fd45a08200a6e3491f100912f086e8458b3b` from required-CI-green candidate `7ac2c48`. Native invite snapshot/classification/actions/avatar ownership replaces and deletes the active JS invite owners; production importers are **197→194** and repository-wide importers **211→208**.
- Repository baseline remains **232 files / 292 direct import lines** referencing
  `matrix-js-sdk`. Each completed vertical must record a negative
  capability-owner/file deletion delta and an honest, non-increasing global
  import delta; the latter may be zero for indirect ownership.
- #221 remains held: zero deleted importers is not completion.
- L1 foundation inventory is about 74/112, but that number is not product
  replacement completion and 0/15 strict phase gates remain closed.

## Required evidence for every vertical

1. Native product ownership through the managed Rust client.
2. Retained behavior parity or explicit product-approved removal.
3. Passphrases and recovery inputs may cross IPC only as one-way command
   inputs. The Rust-owned command buffer is zeroized after the awaited
   operation. Tokens, keys, passphrases, recovery material, ciphertext, raw
   paths, and raw SDK errors must never appear in IPC responses, events,
   diagnostics, or logs.
4. Physical deletion of the replaced JS implementation/imports.
5. Capability-owner/file deletion and repository-wide direct-import counts
   before and after the slice.
6. Scoped Rust tests, product/helper tests, TypeScript typecheck, formatting,
   guardrails, and required CI on the reviewed SHA.
7. Residual ledger and [PROGRESS.md](PROGRESS.md) updated in the same PR.

## Parked

- #221 D0.6 plateau.
- L1-only notification, call-state, media-boundary/policy, and helper PRs unless
  they directly block the active full vertical.
- Umbrella/main PR #39 until explicit approval and final gates.

## Never

- dual backend or runtime selector;
- fallback to a live JS Matrix client after native ownership is selected;
- raw `/_matrix/` product HTTP outside documented SDK-gap approval;
- persistence or retention of secrets in WebView state; secrets in IPC
  responses, events, diagnostics, logs, or generated docs. Transient
  user-entered recovery material may exist only while the user supplies a
  one-way native command input;
- “wired” relabeled as “done” while the legacy implementation remains;
- final bulk burn-down used to defer deletion owned by an earlier vertical.
