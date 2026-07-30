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
| Current integration tip | `151948c8c2329ee6f0b37b8757607b3ac8bb44e7` — V-ROOMS.4 [#246](https://github.com/nepenth/synara-desktop/pull/246) merged from green candidate `c4df9ed`                          |
| Active PRs              | Draft V-ROOMS.2a space parents; [#239](https://github.com/nepenth/synara-desktop/pull/239) V-SEND.2; [#240](https://github.com/nepenth/synara-desktop/pull/240) V-TIMELINE |
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
4. V-ROOMS.3 [#245](https://github.com/nepenth/synara-desktop/pull/245) is merged at integration `efc90d59e6009f45589ce42a29a6f7ebafcf7624` from candidate `a81e026`. Native unread map owns list/nav/platform badges; JS room-list/unread binders deleted; production **194→192**, repository-wide **208→205**. Live badge runtime proof remains **Not confirmed** (not a reopen blocker).
5. CI guardrails harness [#244](https://github.com/nepenth/synara-desktop/pull/244) is merged at `6aa109bf390531e00f79a03a8fac684d3f7b418f`.
6. V-ROOMS.4 [#246](https://github.com/nepenth/synara-desktop/pull/246) is merged at integration `151948c8c2329ee6f0b37b8757607b3ac8bb44e7` from candidate `c4df9ed`. Native typing owns receive/send; production **192→190**, repository-wide **205→203**. Live typing proof remains **Not confirmed** (not a reopen blocker).
7. Start **V-ROOMS.2a** native space parent map: Rust owns joined-space `m.space.child` → `roomToParentsAtom`; delete JS parent-map binder. Candidate inventory production **190→189**, repository-wide **203→202**, allowlist **197→196**. Lobby hierarchy mutations remain **V-ROOMS.2b**.
8. Keep V-SEND.2 [#239](https://github.com/nepenth/synara-desktop/pull/239) draft/ordered; completion and live runtime proof remain unclaimed.
9. Continue V-TIMELINE [#240](https://github.com/nepenth/synara-desktop/pull/240) only through its full-replacement contract. Do not select the presenter or delete `RoomTimeline.tsx` until the full action/media route and runtime proof are complete.
10. Run V-BURN only as the final convergence audit and npm dependency/bootstrap/store cleanup.
11. Resume new media/widgets/notifications/calls verticals only after the residual queue allows.

## Current accounting

- V-ROOMS.4 [#246](https://github.com/nepenth/synara-desktop/pull/246) is merged at `151948c`; production **192→190**, repository-wide **205→203**.
- Repository baseline remains **232 files / 292 direct import lines** referencing
  `matrix-js-sdk`. Each completed vertical must record a negative
  capability-owner/file deletion delta and an honest, non-increasing global
  import delta; the latter may be zero for indirect ownership.
- #221 remains held: zero deleted importers is not completion.

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
