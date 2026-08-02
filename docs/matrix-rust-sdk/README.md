# Matrix Rust SDK replacement (program docs)

<!-- matrix-rust-program-status-link -->

This directory holds **product and program documentation** for replacing
desktop Matrix client ownership with the Rust Matrix SDK on branch
`feature/matrix-rust-sdk-full-replacement`.

## Start here

| Doc                                                                                        | Role                                                                   |
| ------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------- |
| [program-status.md](program-status.md)                                                     | Generated machine ledger (do not hand-edit)                            |
| [full-vertical-policy.md](full-vertical-policy.md)                                         | Complete replacement acceptance (no dual-backend, no plateau)          |
| [d0-residual-completion.md](d0-residual-completion.md)                                     | Residual capability queue                                              |
| [PROGRESS.md](PROGRESS.md)                                                                 | Human-readable progress log                                            |
| [cutover-operating-model.md](cutover-operating-model.md)                                   | Cutover / operating model                                              |
| [v-burn-importer-taxonomy.md](v-burn-importer-taxonomy.md)                                 | Exhaustive 150-file importer taxonomy and residual overlay (docs only) |
| [../matrix-rust-sdk-full-replacement-plan.md](../matrix-rust-sdk-full-replacement-plan.md) | Authoritative plan                                                     |

## Not published here

Local operator playbooks (Grok/Codex/Cursor skills, hybrid babysit workflows,
session resume packets) stay on developer machines under `~/.grok/` and are
gitignored. Do not re-add session-handoff or orchestrator-loop documents to this
public tree.
