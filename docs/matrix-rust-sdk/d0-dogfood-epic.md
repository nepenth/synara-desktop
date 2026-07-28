# D0 — Product replacement epic (full verticals)

| Field | Value |
| --- | --- |
| Status | **Active — full vertical policy** (2026-07-28) |
| Branch | `feature/matrix-rust-sdk-full-replacement` |
| Policy | Clean-break; **no dual-backend**; **[full-vertical-policy.md](full-vertical-policy.md)** — no dogfood minima |
| Residual debt | **[d0-residual-completion.md](d0-residual-completion.md)** — **blocking** before new work |
| Related | [cutover-operating-model.md](cutover-operating-model.md), [delivery-layers.md](delivery-layers.md), [PROGRESS.md](PROGRESS.md) |

## Pivot decision

Stop expanding L1-only harness foundations as the main effort.  
**Execute product replacement:** rip js-sdk ownership capability-by-capability and **fully** re-implement via **UI → Tauri IPC → Rust matrix-sdk**.

L1 modules already on tip are **parts** to wire, not the end product.

## Superseded approach

Earlier D0 wording allowed “crypto **minimum** / usable enough for dogfood” and “approved residual plateau” burn-down. **User directive cancels that.** Incomplete verticals must be finished to full product parity for retained capabilities before starting new verticals.

## Landed slices (historical — partial debt remains)

| ID | Name | Tip status | Debt |
| --- | --- | --- | --- |
| D0.1 | Login + session sole owner | Merged #214 | V-AUTH (SSO/token/UIA/register) open |
| D0.2 | Sync + room list sole owner | Merged #216 | V-ROOMS (invites/spaces/unread) open |
| D0.3 | Timeline read | Merged #218 | V-TIMELINE (virtualization, reactions, rich events, …) open |
| D0.4 | Send text | Merged #219 | V-SEND (attachments, reactions, polls, rich, threads) open |
| D0.5 | Crypto **minimum** | Merged #220 | **V-CRYPTO** full product — **first priority** |
| D0.6 | Burn-down | **#221 HOLD** | Plateau rejected; real **V-BURN** after verticals |

## Current priority (binding)

1. **Close residual queue** in [d0-residual-completion.md](d0-residual-completion.md) — start **V-CRYPTO**.  
2. Do **not** merge plateau / dogfood PRs as complete.  
3. Only after residual queue for a vertical is empty, mark that vertical done.  
4. New verticals (media, widgets, registry, …) only after residual completion order allows.

## Freeze / park

L1-only open PRs (notify polish, call-state, extra media orthogonal, MiniMax helper, etc.) stay **parked** unless they block a full product residual slice.

## Success metrics

1. Residual table rows for active vertical → **0**  
2. Product `matrix-js-sdk` importers for that capability → **0** (or signed types-only)  
3. No PR accepted with dogfood-minimum / plateau residual as done  
4. L1 ledger (`program-status`) stays truthful for harnesses; product ownership tracked here + residual doc + PROGRESS  

## Codex / Grok roles

| Role | Owner |
| --- | --- |
| Implement full vertical residual slices | **Codex** `gpt-5.6-sol` medium (high for crypto edges) |
| Review + tip-merge + focused tests | Codex preferred |
| Dispatch + merge when green | Grok (thin) — **refuse dogfood acceptance** |

## Orchestration

Recurring loop contract: [d0-orchestrator-loop.md](d0-orchestrator-loop.md).
