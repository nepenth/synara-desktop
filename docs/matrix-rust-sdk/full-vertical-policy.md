# Full vertical replacement policy

<!-- matrix-rust-program-status-link -->

| Field           | Value                                                                                          |
| --------------- | ---------------------------------------------------------------------------------------------- |
| Status          | **Active** (user directive 2026-07-28)                                                         |
| Branch          | `feature/matrix-rust-sdk-full-replacement` only                                                |
| Supersedes      | “Incomplete minimum / usable enough / approved residual plateau” acceptance for product verticals |
| Deletion policy | **Physical deletion happens inside each vertical** (clarified 2026-07-28)                      |

## Directive

**Complete replacement only.** Each product vertical must be **fully implemented / re-implemented** under the native Matrix Rust SDK path (UI → Tauri IPC → live `matrix-sdk`), for capabilities that:

1. Exist in the current Synara client (matrix-js-sdk product surface), **and/or**
2. Are supported by matrix-rust-sdk and are part of product parity for that vertical.

Do **not** ship “minimum”, “usable enough”, “approved non-zero residual plateau”, or stub shells as acceptance for a vertical. Temporary brokenness _between_ sequential full verticals is OK; **declaring a vertical done while product still owns the capability on js-sdk is not**.

## Physical deletion is part of the vertical

Replacing the native happy path is necessary, but it is not the complete slice.
The same vertical must also delete the superseded `matrix-js-sdk` implementation,
imports, compatibility branches, tests that exist only for that implementation,
and obsolete product types for its capability.

Do not retain a `Legacy*` component or `isNative ? rust : js` branch on the theory
that a final burn-down will remove it later. The React runtime is a desktop
implementation detail; Synara does not ship a separate browser product that
requires a second Matrix implementation. Shared UI that remains useful should be
made SDK-neutral and consume Synara-owned DTOs.

For work merged before this clarification, “wired” means the Rust product path
landed; it does **not** mean the vertical is closed. Reopen a named deletion
residual and drain it before advancing.

## Dual backend

Still **forbidden** forever. Full vertical ≠ dual runtime selector.

## Acceptance for a vertical slice

A slice is **done only when** all of the following hold for its capability set:

| Gate          | Requirement                                                                                                                                        |
| ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Ownership** | Product happy path does not use `matrix-js-sdk` live client for that capability                                                                    |
| **Parity**    | Behaviors that existed in product for that capability are re-homed to Rust IPC/SDK (or explicitly deleted with product sign-off — rare)            |
| **Deletion**  | Superseded JS implementation/imports and JS-only compatibility branches for the capability are physically removed in this slice                    |
| **Secrets**   | No tokens, keys, recovery material, ciphertext over IPC or logs                                                                                    |
| **Ledger**    | Residual list for that slice is **empty** or only items that are **other verticals** with named follow-up IDs (not “later / deferred / minimum”)   |
| **Tests**     | Smallest focused evidence set for changed retained behavior and real privacy/authority boundaries; required PR CI supplies broad integration proof |

**Not** acceptance: “core path works; verification/backup/media left as approved residual.”

## Sequencing under this policy

1. **Finish incomplete prior verticals first** — see [d0-residual-completion.md](d0-residual-completion.md).
2. Only then open new verticals (media, widgets, registry, …) with the same full bar.
3. Do **not** merge PRs that document “approved residual plateau” or an incomplete minimum as the done state for D0 / crypto / burn-down.
4. L1 harness PRs remain parked unless they block a full product vertical.
5. Capability-owner/file deletion must be negative and recorded for each completed vertical. Record the repository-wide direct `matrix-js-sdk` import delta too; it must not increase, but it may be zero when the deleted owner reached the SDK indirectly through a shared hook. Do not mix unrelated cleanup into a slice merely to force the global counter down. The final burn-down is a verification and dependency-removal gate, not a warehouse for deferred capability deletion.

## Orchestrator / Codex rules

- Prefer one full vertical in flight (serial product merges).
- Reject prompts that say “minimum”, “partial is enough”, “plateau residual OK”, “0 imports removed is fine”.
- If a PR only documents residual debt without product rewire, treat as **docs debt PR**, not vertical complete.
- If a PR adds a native branch but retains the replaced JS branch, mark it **wired / deletion open**, not done.
- High effort sparingly for crypto/session edges; still full product wire, not stubs.
- Apply [operating-path-contract.md](operating-path-contract.md): incidental
  findings remain separate, and new preservation machinery requires a named
  confirmed path plus a concrete boundary that existing focused evidence and CI
  do not protect.

## Related

- Residual inventory: [d0-residual-completion.md](d0-residual-completion.md)
- Epic (reoriented): [d0-product-replacement-epic.md](d0-product-replacement-epic.md)
- Loop: [README.md](README.md)
- Operating path and evidence budget: [operating-path-contract.md](operating-path-contract.md)
- Migration crypto decisions: [migration-ux-decision.md](migration-ux-decision.md) (`D-KEY-RECOVERY`, etc.)

See [program-status.md](program-status.md).
