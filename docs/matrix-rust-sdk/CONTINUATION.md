# Continuation — full-vertical Matrix Rust replacement

> **Current continuation card.** Full history remains in
> [PROGRESS.md](PROGRESS.md); original plan inventory and strict gates remain in
> [program-status.md](program-status.md).

<!-- matrix-rust-program-status-link -->

| Field                    | Current value                                                                                            |
| ------------------------ | -------------------------------------------------------------------------------------------------------- |
| Date                     | 2026-07-28 (America/New_York)                                                                            |
| Integration branch       | `feature/matrix-rust-sdk-full-replacement`                                                               |
| Execution model          | Primary Codex + Codex sub-agents, all `gpt-5.6-sol` medium; MiniMax-M3 optional                          |
| Verified integration tip | `fd7c934` — full-vertical policy alignment merged (#228)                                                 |
| Active PR                | [#227](https://github.com/nepenth/synara-desktop/pull/227) — V-CRYPTO.5 closure implementation in review |
| Policy                   | [full-vertical-policy.md](full-vertical-policy.md)                                                       |
| Binding queue            | [d0-residual-completion.md](d0-residual-completion.md)                                                   |
| Main merge               | PR #39 — explicit user approval required                                                                 |

## Active direction

Complete the desktop replacement as serial, product-visible verticals:

```text
React UI → versioned Tauri IPC + Synara DTOs → live Rust matrix-sdk
```

No dogfood minima, no accepted residual plateau, no backend selector, and no
concurrent JS/Rust clients for one session.

**Physical deletion happens per vertical.** A native path beside retained JS
ownership is “wired,” not “done.” Each capability slice must delete its
superseded `matrix-js-sdk` implementation, imports, compatibility branches, and
obsolete tests/types before closure.

## Exact continuation point

1. Validate and land active V-CRYPTO.5 [#227](https://github.com/nepenth/synara-desktop/pull/227): its branch now removes `LegacyLocalBackup`, WebView room-key file crypto/FileSaver behavior, and the old JS owner; closure still requires reviewed-SHA gates and ledger evidence.
2. Drain the already-wired crypto deletion queue serially:
   - V-CRYPTO.1-D — verification;
   - V-CRYPTO.2-D — cross-signing;
   - V-CRYPTO.3-D — backup/recovery;
   - V-CRYPTO.4-D — secret storage.
3. Implement V-CRYPTO.6 UTD/history recovery as a complete wire-plus-delete vertical.
4. Implement V-CRYPTO.7 device list/trust/actions as a complete wire-plus-delete vertical.
5. Continue V-AUTH → V-ROOMS → V-TIMELINE → V-SEND, deleting the superseded JS owner inside each capability slice.
6. Run V-BURN only as the final convergence audit and npm dependency/bootstrap/store cleanup.
7. Resume new media/widgets/notifications/calls verticals only after the residual queue allows.

## Current accounting

- D0.1–D0.5 established native login/session, sync/room list, basic timeline,
  plain-text send, and encrypted-room machine paths.
- V-CRYPTO.1–.4 product wiring merged in #223–#226.
- Those four rows are reopened as **wired / deletion open** because relevant JS
  crypto imports and conditional legacy implementations remain.
- V-CRYPTO.5 is still draft and can be corrected before merge.
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
- secrets in WebView state, IPC returns, diagnostics, logs, or generated docs;
- “wired” relabeled as “done” while the legacy implementation remains;
- final bulk burn-down used to defer deletion owned by an earlier vertical.
