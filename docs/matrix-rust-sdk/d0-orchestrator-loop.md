# D0 orchestrator loop (integration branch)

| Field | Value |
| --- | --- |
| Active | **Yes** (2026-07-28) |
| Interval | Every **4 minutes** (scheduler) |
| Integration | `feature/matrix-rust-sdk-full-replacement` only |
| Epic | [d0-dogfood-epic.md](d0-dogfood-epic.md) |

## Roles

| Agent | Role |
| --- | --- |
| **Grok** (thin) | Fire this loop; merge green PRs; dispatch Codex; never merge main/#39 |
| **Codex** `gpt-5.6-sol` **medium** | Implement D0 product rewires, tip-merge, lightweight review |
| **MiniMax-M3** | Optional free text during waits; never sole implementer |

## Hard policy

- **No dual_backend**
- Clean-break re-login OK; branch product may be broken until D0.1–D0.2
- **Do not** open new L1-only foundation PRs unless they block D0
- Parked L1 PRs stay parked
- **No tokens/secrets** in IPC returns / logs
- Prefer **serial** product merges (one package-smoke queue at a time)
- **No** PROGRESS tip-merge into a green product PR mid-package run
- Slim Codex validation: `fmt` + module `cargo test` (+ optional clippy); **no** full suite / package / governance regen

## Priority order (always)

1. Merge green **D0** product PR if Quality + Desktop package gates success  
2. Merge green **docs** that unblock tracking (ledger / D0 epic) if gates allow  
3. If no merge: advance **current D0 slice** via Codex (see below)  
4. Tip-merge **only** the next D0 PR if BEHIND/CONFLICTING  
5. Update PROGRESS only after product land (batch), not between every CI tick  
6. Report short status; stop if disk &lt; 5 Gi free (clean `target` only)

## Current slice pointer

| Slice | Status | Branch / PR |
| --- | --- | --- |
| D0.1 Login/session sole owner | **Implement / land** | `matrix-rust/d0.1-rust-login-owner` |
| D0.2 Sync + room list | Blocked on D0.1 | — |
| D0.3 Timeline read | After D0.2 | — |
| D0.4 Send text | After D0.3 | — |

*Orchestrator must rewrite this table when a slice merges.*

## Codex dispatch recipes

### A) Tip-merge next D0 PR only

```text
codex exec -m gpt-5.6-sol -c model_reasoning_effort="medium" --ephemeral -
# prompt: tip-merge PR N onto origin/feature/matrix-rust-sdk-full-replacement;
# resolve mod.rs by union; fmt; cargo test --lib <module>; push; do not merge
```

### B) Implement next D0 slice (after D0.1 lands)

Use epic acceptance criteria in `d0-dogfood-epic.md`. One branch, one PR, product rip + Rust wire.

### C) Review (optional, save Grok)

Review PR for dual_backend / secrets / scope; write `/tmp/codex-review-N.md` PASS/FAIL.

## Explicitly do NOT do each fire

- Open new notify/call/media L1 polish PRs  
- Cancel random CI unless runner starvation blocks **the** next D0 merge  
- Full-repo cargo test / package builds locally  
- Merge umbrella #39 or `main`  
- Claim phase-gate acceptance  

## End-of-fire status template

```markdown
## D0 loop status
- Tip: `sha` — message
- Merged: …
- D0.1 PR: #N gates …
- Codex: running / done / none
- Next action: …
- Blockers: …
```
