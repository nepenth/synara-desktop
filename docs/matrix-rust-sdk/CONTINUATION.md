# Continuation — full-vertical Matrix Rust replacement

> **Current continuation card.** Full history remains in
> [PROGRESS.md](PROGRESS.md); original plan inventory and strict gates remain in
> [program-status.md](program-status.md).

<!-- matrix-rust-program-status-link -->

| Field                   | Current value                                                                                                                                                                                         |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Date                    | 2026-07-31 (America/New_York)                                                                                                                                                                         |
| Integration branch      | `feature/matrix-rust-sdk-full-replacement`                                                                                                                                                            |
| Execution model         | Primary Codex and every implementation/review sub-agent: `gpt-5.6-sol`, medium reasoning                                                                                                              |
| Current integration tip | `fe628859836f0b3e8f6dab696f7739074f28311c` — docs handoff [#256](https://github.com/nepenth/synara-desktop/pull/256) after V-SEND.4 [#253](https://github.com/nepenth/synara-desktop/pull/253) |
| Active PRs              | Draft V-ROOMS.2b hierarchy [#254](https://github.com/nepenth/synara-desktop/pull/254); [#240](https://github.com/nepenth/synara-desktop/pull/240) V-TIMELINE **HOLD** |
| Policy                  | [full-vertical-policy.md](full-vertical-policy.md)                                                                                                                                                    |
| Operating path          | [operating-path-contract.md](operating-path-contract.md)                                                                                                                                              |
| Binding queue           | [d0-residual-completion.md](d0-residual-completion.md)                                                                                                                                                |
| Main merge              | PR #39 — explicit user approval required                                                                                                                                                              |

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

1. V-CRYPTO.7 [#236](https://github.com/nepenth/synara-desktop/pull/236) is merged at integration `528a510`; reviewed green product/test head was `192be46`. Live multi-session/UI proof remains unclaimed.
2. V-AUTH.1 [#238](https://github.com/nepenth/synara-desktop/pull/238) is merged: desktop SSO entry points, callback/token-completion ownership, and native SSO UIAA continuation are deleted without a replacement route. Production importers **201→197**.
3. V-ROOMS.1 [#241](https://github.com/nepenth/synara-desktop/pull/241) is merged at `2c48fd4` (candidate `7ac2c48`); production **197→194**, repository-wide **211→208**.
4. V-ROOMS.3 [#245](https://github.com/nepenth/synara-desktop/pull/245) is merged at `efc90d5` (candidate `a81e026`); production **194→192**, repository-wide **208→205**. Live badge proof **Not confirmed** (not a reopen blocker).
5. CI guardrails harness [#244](https://github.com/nepenth/synara-desktop/pull/244) is merged at `6aa109b`.
6. V-ROOMS.4 [#246](https://github.com/nepenth/synara-desktop/pull/246) is merged at `151948c` (candidate `c4df9ed`); production **192→190**, repository-wide **205→203**. Live typing proof **Not confirmed**.
7. V-SEND.2 [#239](https://github.com/nepenth/synara-desktop/pull/239) is merged at `988cdc2`; Synapse native reaction proof Confirmed on reviewed head; whole importers **197→197**.
8. V-ROOMS.2a [#247](https://github.com/nepenth/synara-desktop/pull/247) is merged at `a919689`; production **190→189**, repository-wide **203→202**. Lobby hierarchy mutations remain **V-ROOMS.2b**.
9. V-SEND.1 [#248](https://github.com/nepenth/synara-desktop/pull/248) is merged at `90be0f4`.
10. V-ROOMS.5 read [#249](https://github.com/nepenth/synara-desktop/pull/249) is merged at `d17ab2c` (candidate `708aef7`); production **189→187**, repository-wide **202→200**, allowlist **196→194**.
11. V-ROOMS.5w writers [#251](https://github.com/nepenth/synara-desktop/pull/251) is merged at `0fb0fe4` (candidate `e4e2639`); native `matrix_mdirect_add` / `matrix_mdirect_remove`; JS writer helpers deleted; importers **187→187**.
12. V-ROOMS.5r native `m.direct` user list [#252](https://github.com/nepenth/synara-desktop/pull/252) is merged at `9579ea4`.
13. V-SEND.3 polls [#250](https://github.com/nepenth/synara-desktop/pull/250) is merged (reviewed head `761d2ef`); Synapse native poll proof Confirmed.
14. V-SEND.4 rich composer messages [#253](https://github.com/nepenth/synara-desktop/pull/253) is merged at `b558344`.
15. Complete **V-ROOMS.2b** hierarchy summaries [#254](https://github.com/nepenth/synara-desktop/pull/254). Keep V-TIMELINE [#240](https://github.com/nepenth/synara-desktop/pull/240) **HOLD** — do not select the presenter or delete `RoomTimeline.tsx` until the full action/media route and runtime proof are complete.
16. Run V-BURN only as the final convergence audit and npm dependency/bootstrap/store cleanup.
17. Resume new media/widgets/notifications/calls verticals only after the residual queue allows.

## Current accounting

- Tip `fe62885` merges docs handoff [#256](https://github.com/nepenth/synara-desktop/pull/256); prior product tip `b558344` was V-SEND.4 [#253](https://github.com/nepenth/synara-desktop/pull/253).
- Inventory on tip: production import files **187**, repository-wide **200**.
- Repository baseline remains **232 files / 292 direct import lines** referencing `matrix-js-sdk`. Each completed vertical must record a negative capability-owner/file deletion delta and an honest, non-increasing global import delta; the latter may be zero for indirect ownership.
- #221 remains held: zero deleted importers is not completion.

## Open PR disposition (integration base)

| PR | Disposition | Why |
| --- | --- | --- |
| [#250](https://github.com/nepenth/synara-desktop/pull/250) V-SEND.3 | **Merged** | Reviewed green head `761d2ef`; Synapse poll proof Confirmed |
| [#253](https://github.com/nepenth/synara-desktop/pull/253) V-SEND.4 | **Merged** | Integration `b558344` |
| [#256](https://github.com/nepenth/synara-desktop/pull/256) docs handoff | **Merged** | Integration `fe62885` |
| [#254](https://github.com/nepenth/synara-desktop/pull/254) V-ROOMS.2b | **Active draft** | Hierarchy summaries; tip-merge this PR |
| [#240](https://github.com/nepenth/synara-desktop/pull/240) V-TIMELINE | **HOLD** | Incomplete full-replacement contract; presenter unselected; conflicting vs tip after #251 |
| [#221](https://github.com/nepenth/synara-desktop/pull/221) D0.6 | **HOLD** | Plateau / zero importer deletion |
| [#243](https://github.com/nepenth/synara-desktop/pull/243) docs tracking | **This PR** | Docs/tracking rewrite onto tip; merge when green (tracking-only historically lands) |
| [#193](https://github.com/nepenth/synara-desktop/pull/193)–[#209](https://github.com/nepenth/synara-desktop/pull/209) L1 foundations | **HOLD (parked)** | L1-only notify/call/media/filter/bootstrap; do not merge until residual queue allows |
| [#109](https://github.com/nepenth/synara-desktop/pull/109) MiniMax helper | **HOLD (parked)** | Tooling helper; not on active vertical path |
| [#39](https://github.com/nepenth/synara-desktop/pull/39) umbrella → main | **HOLD** | Never merge without explicit user approval |

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
- #240 V-TIMELINE until contract closable (no presenter selection / no `RoomTimeline` deletion).
- L1-only notification, call-state, media-boundary/policy, filter, crypto-bootstrap, and helper PRs (#109, #193, #196, #198, #199, #201, #203, #204, #207, #208, #209) unless they directly block the active full vertical.
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
