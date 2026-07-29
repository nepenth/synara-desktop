# D0 orchestrator loop (integration branch)

| Field              | Value                                                                                |
| ------------------ | ------------------------------------------------------------------------------------ |
| Active             | **Paused between slices** (2026-07-29); resume as a persistent Codex goal            |
| Interval           | Goal continuation; optional scheduler fires every **4 minutes**                      |
| Integration        | `feature/matrix-rust-sdk-full-replacement` only                                      |
| Orchestrator       | **Codex `gpt-5.6-sol`, medium reasoning**                                            |
| Epic               | [d0-dogfood-epic.md](d0-dogfood-epic.md)                                             |
| **Policy**         | **[full-vertical-policy.md](full-vertical-policy.md)** — **no dogfood cuts**         |
| **Residual queue** | [d0-residual-completion.md](d0-residual-completion.md) — **must drain first**        |
| **Operating path** | [operating-path-contract.md](operating-path-contract.md) — scope and evidence budget |

## Roles

| Agent                                      | Role                                                                                                             |
| ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------- |
| **Codex** `gpt-5.6-sol` **medium**         | Primary orchestrator: fire this loop, own Git/PR state, review and merge green full-vertical PRs, never main/#39 |
| **Codex sub-agents**, same model/reasoning | Implement bounded task packets or independent review in parallel when scopes do not conflict                     |
| **MiniMax-M3**                             | Optional text/research assistance during waits; never sole implementer, reviewer, or acceptance authority        |

Grok is not part of the active execution path while its usage allocation is
unavailable. The primary Codex orchestrator retains acceptance and merge
authority even when implementation is delegated.

## Hard policy

- **No dual_backend**
- **No dogfood / minimum / plateau residual** acceptance for product verticals
- **Physical deletion per vertical** — native wiring plus retained legacy JS is not done
- Clean-break re-login OK while a full vertical is mid-stack
- **Do not** open new L1-only foundation PRs unless they block residual queue
- Parked L1 PRs stay parked
- **No tokens/secrets** in IPC returns / logs
- Prefer **serial** product merges
- Follow the operating-path evidence budget: focused owner tests locally; required PR CI supplies broad integration proof
- Do not add tests, guardrails, retries, or fallbacks without a named confirmed path and concrete boundary they preserve
- **Do not merge #221** (or successors) that claim “approved residual plateau” / “no capability owner deleted is success”

## Priority order (always)

1. **Do not merge** PRs that only plateau residual / dogfood-shell without full vertical acceptance
2. Merge green **residual-completion / full vertical** product PR if Quality + Desktop package gates success
3. Merge green **policy/docs** that enforce full-vertical + residual ledger
4. If no merge: review V-CRYPTO.6; after it lands, advance V-CRYPTO.7 from [d0-residual-completion.md](d0-residual-completion.md)
5. Tip-merge **only** the active residual PR if BEHIND/CONFLICTING
6. Update PROGRESS after residual lands
7. Report short status; stop if disk &lt; 5 Gi free

## Current slice pointer

| Slice         | Status                                   | Branch / PR                                                                  |
| ------------- | ---------------------------------------- | ---------------------------------------------------------------------------- |
| D0.1–D0.4     | Merged (partial debt → residual doc)     | tip history                                                                  |
| V-CRYPTO.1–.3 | **DONE**                                 | Native owners; legacy verification, cross-signing, and backup owners deleted |
| V-CRYPTO.4    | **DONE in V-CRYPTO.4-D candidate**       | Native owner retained; legacy secret-storage owner deleted                   |
| V-CRYPTO.5    | **DONE #227**                            | Rust-only owner; legacy owner/helper deleted; gates green                    |
| V-CRYPTO.6    | **DONE candidate**                       | Automatic native UTD/history recovery; legacy retry/listener owners deleted |
| V-CRYPTO.7    | Queued                                   | Device/trust; wire + delete                                                   |
| D0.6 plateau  | **#221 HOLD — do not merge as complete** | rework → V-BURN later                                                        |
| **Next**      | **Review V-CRYPTO.6, then V-CRYPTO.7**   | Device list/trust follows the UTD recovery candidate                         |

_Orchestrator must rewrite this table when a residual slice merges._

## Codex dispatch recipes

### A) Tip-merge active residual PR only

```text
codex exec -m gpt-5.6-sol -c model_reasoning_effort="medium" --ephemeral -
# tip-merge PR N onto origin/feature/matrix-rust-sdk-full-replacement;
# resolve mod.rs by union; fmt; cargo test --lib <module>; push; do not merge
```

### B) Implement next residual full vertical

Use [d0-residual-completion.md](d0-residual-completion.md) + [full-vertical-policy.md](full-vertical-policy.md) + [operating-path-contract.md](operating-path-contract.md).

**Full product rewire plus physical deletion** for the ID. One
capability-bounded branch/PR; record deleted paths/import delta; close the row
only when both ownership and deletion gates pass.

### C) Review

Reject dual_backend, secrets, dogfood-minimum scope, zero-deletion completion,
or a retained `Legacy*` / native-vs-JS branch for the claimed capability.

## Explicitly do NOT do each fire

- Merge #221 plateau as D0.6 complete
- Open new media/widgets/registry verticals before residual queue allows
- Open new notify/call L1 polish PRs
- Start V-CRYPTO.7 while V-CRYPTO.6 remains open
- Merge umbrella #39 or `main`
- Claim phase-gate acceptance for crypto until V-CRYPTO complete

## End-of-fire status template

```markdown
## Loop status (full vertical)

- Tip: `sha` — message
- Merged: …
- Residual next: V-CRYPTO.N / …
- Import delta: files/statements removed this vertical
- Held: #221 …
- Codex: running / done / none
- Next action: …
```

## Package smoke (integration)

Full Desktop package smoke is **off by default** on PRs into this branch.
Gate still reports success. Opt in with label `needs-package`, or run
`workflow_dispatch` when ready for installable artifacts.
