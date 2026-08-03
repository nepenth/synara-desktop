# Pipeline pause handoff — 2026-08-03

## Why paused

Operator request: wrap in-flight work, **stop loops/spawns**, audit tracking, clean disk/PRs, leave resume-ready.

## Policy refresh — 2026-08-03

HUMAN OPERATOR LIVE-PROOF is removed as a completion or merge gate for
residual-empty `matrix-js-sdk` burns on `feature/matrix-rust-sdk-full-replacement`.
For branch purposes, a claimed file is accepted when its code is on the
measured tip, focused unit/CI checks pass, and no `matrix-js-sdk` import remains
in that file. Native product paths that require live Matrix state must fail
closed. Fix-forward and private Beta are accepted.

C3–C5 desktop proof remains optional Beta feedback and may stay **Not
confirmed** without holding a residual-empty burn or merge. R-DEVTOOL may start
without waiting for C3–C5 live confirmation once normal operator scheduling is
re-enabled. V-BURN remains **HOLD**, `dual_backend=false` remains forbidden
forever, and `main` / umbrella #39 remain out of scope.

When a product change changes importer inventory, run
`npm run inventory:matrix-sdk-usage`; ratchet the allowlist `pathCount` and
`paths[]`; update inventory test floors for files, declarations, and buckets;
and update P1.6 guardrail floors. This docs-only policy PR changes no
production importers and carries the tip inventory's 124 files / 124 paths.

## Hard stop (do not violate)

| Control | State |
| ------- | ----- |
| Daytime Grok scheduler | **CANCELLED** (`019fbea5fc75`) |
| Overnight | **OFF** |
| New implementers / bench fill | **FORBIDDEN** until operator re-enables |
| `dual_backend` | **false** forever |
| main / umbrella #39 | **no merge** without explicit approval |
| V-BURN | **HOLD** |

## Tip snapshot (authoritative)

| Field | Value |
| ----- | ----- |
| Branch | `feature/matrix-rust-sdk-full-replacement` |
| Tip SHA (short) | **`abd736b3`** |
| Tip subject | `docs(matrix): pause handoff + tip honesty after #538 (124 importers) (#539)` |
| Production import files | **124** / baseline **220** (**96** removed, ~**43.6%**) |
| Allowlist pathCount | **124** (= `paths[]` length) |
| Inventory source | `docs/matrix-rust-sdk/desktop-sdk-usage.md` **on tip** (never a stale worktree) |
| Burn board | https://kb.whyland.com/go/synara-matrix-burn |
| Open product PRs (base = full-replacement) | **one docs-only policy PR draft** (this branch) |

## What landed during the 2026-08-02 → 2026-08-03 push

### Serial residual verticals (native / residual-empty)

| PR | Scope |
| -- | ----- |
| #514 | Direct power/create readers (native projections) |
| #515 | Presence full residual lifecycle |
| #516 | Power-level tags READ |
| #517 | Room summary micro-slice |
| #518 | Member projection SDK imports |
| #519 | Via-server member importer kill |
| #520 | Room directory visibility native |
| #521 | RoomPublish join-rule **READ** residual-empty |
| #522 | Join-rule presentation DTO freeze |

### Long-tail residual-empty (type / presentation / orphan)

| PR | Scope |
| -- | ----- |
| #523 | RoomAvatar join-rule presentation |
| #524 | JoinRulesSwitcher importer kill |
| #525 | Room/Space settings join-rule presentation types |
| #526 | PollContent type-only |
| #527 | Orphan importers (CapabilitiesLoader, useMemberEventParser, …) |
| #528 | MsgTypeRenderers presentation |
| #529 | polls.ts type-only + pathCount honesty |
| #530 | media.ts type-only MatrixClient |
| #531 | common.ts type-only |
| #532 | history-visibility type residual (JS writer retained) |
| #533 | useMatrixClient type-only |
| #534 | MessageSearch/SearchFilters SearchOrderBy presentation |
| #535 | roomInputDrafts IEventRelation type-only |
| #536 | useAuthMetadata type-only |
| #537 | AccountDataEditor MatrixError type-only |
| #538 | RenderMessageContent MsgType presentation |

**Importer path (historical approximation):** ~153 (early 08-01 scoreboard) → **~145** after join-rule/native stack → **124** after long-tail type burns. The authoritative current count is the committed tip inventory above.

## Inventory honesty rules (resume)

1. Always regenerate inventory on the **product branch** before land.
2. Ratchet **together**: `desktop-sdk-usage.{md,json}`, allowlist `pathCount` **and** `paths[]`, inventory test floors (files + declarations + buckets), p1.6 guardrail floors.
3. Burn board must read inventory from **`origin` tip**, not a residual worktree checkout.

## Remaining hard residuals (not exhaustive)

- **RoomJoinRules.tsx writer** (presentation switcher closed; writer open)
- **useMessageSearch** (still owns SearchOrderBy execution)
- **utils/room.ts**, timeline utils/lifecycle, matrix.ts utilities
- **initMatrix / cryptoStoreContinuity** client lifecycle
- **CallWidget** media config/download + call-status surfaces
- **R-DEVTOOL** SendRoomEvent / StateEventEditor
- **C3–C5** live proofs still **Not confirmed**; optional Beta feedback, not a merge hold

Bucket sketch at tip (desktop-runtime production importers ≈ inventory files): feature ~47, hook ~35, component ~14, utility ~9, page ~7, plugin ~6, state ~4, client-lifecycle ~2.

## Cleanup performed at pause

| Action | Result |
| ------ | ------ |
| Scheduler | Cancelled |
| Live codex agents | 0 |
| Stale docs freezes #502–#512 | **Closed** with audit comments |
| Temp git worktrees under `/tmp/synara-codex*` | **Removed** |
| `ACTIVE_WORK.json` | Cleared to empty agents |
| In-flight product | **#538** merged as final wrap-up |

## Resume checklist

1. Confirm tip + inventory:  
   `git fetch origin feature/matrix-rust-sdk-full-replacement && git rev-parse --short origin/feature/matrix-rust-sdk-full-replacement`  
   `git show origin/…:docs/matrix-rust-sdk/desktop-sdk-usage.md | grep 'Production import files'`
2. Re-enable daytime only if operator asks: recreate 15m scheduler from `/tmp/synara-daytime-pipeline/DAYTIME_PROMPT.md` (or skill `matrix-rust-full-vertical`).
3. Set `LANE_OWNER` to a **single** residual-empty headline.
4. Publish burn board after first spawn / merge (`matrix-burn-dashboard`).
5. Prefer **one** product PR at a time; serial land; no tip-docs thrash.

## Local workspace note

The operator’s main checkout may still be on an old residual branch with **stale local inventory (e.g. 153)**. That is **not** tip. Always measure tip via `origin/feature/matrix-rust-sdk-full-replacement`.
