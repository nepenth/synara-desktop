# D0 orchestrator loop (integration branch)

| Field | Value |
| --- | --- |
| Active | **Yes** (2026-07-28) |
| Interval | Every **4 minutes** (scheduler) |
| Integration | `feature/matrix-rust-sdk-full-replacement` only |
| Epic | [d0-dogfood-epic.md](d0-dogfood-epic.md) |
| **Policy** | **[full-vertical-policy.md](full-vertical-policy.md)** — **no dogfood cuts** |
| **Residual queue** | [d0-residual-completion.md](d0-residual-completion.md) — **must drain first** |

## Roles

| Agent | Role |
| --- | --- |
| **Grok** (thin) | Fire this loop; merge green **full-vertical** PRs only; dispatch Codex; never merge main/#39 |
| **Codex** `gpt-5.6-sol` **medium** | Full product rewires (not minima); tip-merge; lightweight review |
| **MiniMax-M3** | Optional free text during waits; never sole implementer |

## Hard policy

- **No dual_backend**
- **No dogfood / minimum / plateau residual** acceptance for product verticals
- Clean-break re-login OK while a full vertical is mid-stack
- **Do not** open new L1-only foundation PRs unless they block residual queue
- Parked L1 PRs stay parked
- **No tokens/secrets** in IPC returns / logs
- Prefer **serial** product merges
- Slim Codex validation: `fmt` + module `cargo test` (+ optional clippy); **no** full suite / package / governance regen
- **Do not merge #221** (or successors) that claim “approved residual plateau” / “0 imports removed is success”

## Priority order (always)

1. **Do not merge** PRs that only plateau residual / dogfood-shell without full vertical acceptance  
2. Merge green **residual-completion / full vertical** product PR if Quality + Desktop package gates success  
3. Merge green **policy/docs** that enforce full-vertical + residual ledger  
4. If no merge: advance **next residual ID** from [d0-residual-completion.md](d0-residual-completion.md) via Codex — default **V-CRYPTO.1** then V-CRYPTO.*  
5. Tip-merge **only** the active residual PR if BEHIND/CONFLICTING  
6. Update PROGRESS after residual lands  
7. Report short status; stop if disk &lt; 5 Gi free  

## Current slice pointer

| Slice | Status | Branch / PR |
| --- | --- | --- |
| D0.1–D0.4 | Merged (partial debt → residual doc) | tip history |
| D0.5 Crypto minimum | Merged #220 — **incomplete under full-vertical** | debt = **V-CRYPTO.*** |
| D0.6 plateau | **#221 HOLD — do not merge as complete** | rework → V-BURN later |
| **Next** | **V-CRYPTO.1** device verification product wire | dispatch on tip |

*Orchestrator must rewrite this table when a residual slice merges.*

## Codex dispatch recipes

### A) Tip-merge active residual PR only

```text
codex exec -m gpt-5.6-sol -c model_reasoning_effort="medium" --ephemeral -
# tip-merge PR N onto origin/feature/matrix-rust-sdk-full-replacement;
# resolve mod.rs by union; fmt; cargo test --lib <module>; push; do not merge
```

### B) Implement next residual full vertical

Use [d0-residual-completion.md](d0-residual-completion.md) + [full-vertical-policy.md](full-vertical-policy.md).  
**Full product rewire** for the ID (start **V-CRYPTO.1**). No “minimum” acceptance. One branch, one PR, residual row closed when done.

### C) Review

Reject dual_backend, secrets, dogfood-minimum scope, empty residual claimed as done.

## Explicitly do NOT do each fire

- Merge #221 plateau as D0.6 complete  
- Open new media/widgets/registry verticals before residual queue allows  
- Open new notify/call L1 polish PRs  
- Merge umbrella #39 or `main`  
- Claim phase-gate acceptance for crypto until V-CRYPTO complete  

## End-of-fire status template

```markdown
## Loop status (full vertical)
- Tip: `sha` — message
- Merged: …
- Residual next: V-CRYPTO.N / …
- Held: #221 …
- Codex: running / done / none
- Next action: …
```

## Package smoke (integration)

Full Desktop package smoke is **off by default** on PRs into this branch.
Gate still reports success. Opt in with label `needs-package`, or run
`workflow_dispatch` when ready for installable artifacts.
