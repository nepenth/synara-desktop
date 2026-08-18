# Matrix Rust SDK replacement (program docs)

<!-- matrix-rust-program-status-link -->

> **Completed migration archive.** This directory records the program that
> replaced desktop Matrix ownership with the shared Rust core. Branch names,
> runtime labels, task states, and counts are historical snapshots. Current
> architecture is documented in
> [the codebase knowledge base](../../CODEBASE_KNOWLEDGE_BASE.md).

This directory holds product and program evidence from the replacement work.

> ⚠️ **PUBLIC repository.** Everything in this tree is public. Never commit
> secrets, tokens, keys, credentials, session/recovery material, private
> endpoints, or personal data. See
> [operating-instructions.md](operating-instructions.md) §1 before adding any
> document.

## Start here

| Doc                                                                                        | Role                                                                   |
| ------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------- |
| [program-status.md](program-status.md)                                                     | Generated machine ledger (do not hand-edit)                            |
| [full-vertical-policy.md](full-vertical-policy.md)                                         | Complete replacement acceptance (no dual-backend, no plateau)          |
| [d0-residual-completion.md](d0-residual-completion.md)                                     | Residual capability queue                                              |
| [PROGRESS.md](PROGRESS.md)                                                                 | Human-readable progress log                                            |
| [operating-instructions.md](operating-instructions.md)                                     | **Live operating model** — public hygiene, this harness, UI/UX fidelity |
| [cutover-operating-model.md](cutover-operating-model.md)                                   | Cutover / operating model                                              |
| [v-burn-importer-taxonomy.md](v-burn-importer-taxonomy.md)                                 | Exhaustive 150-file importer taxonomy and residual overlay (docs only) |
| [../matrix-rust-sdk-full-replacement-plan.md](../matrix-rust-sdk-full-replacement-plan.md) | Authoritative plan                                                     |

## Not published here

Session-resume packets, orchestrator loops, and agent-skill content stay local
and gitignored. Execution now runs through **this agent harness with its
locally hosted model** (no external model APIs). Keep orchestrator-only
artifacts out of this public tree. See
[operating-instructions.md](operating-instructions.md).
