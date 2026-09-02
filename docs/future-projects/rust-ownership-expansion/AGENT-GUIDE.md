# Agent Guide: Investigating a Rust Ownership Workstream

Use this guide when assigning one ROE workstream to an agent. The default task
is **research and planning**, not implementation.

## Required reading

Read these completely before drawing a boundary:

1. [portfolio triage](TRIAGE.md);
2. [project charter](README.md);
3. the assigned file under [`workstreams/`](workstreams/);
4. [ADR 0003](../../adr/0003-shared-native-rust-core.md);
5. [ADR 0004](../../adr/0004-rust-language-boundaries.md);
6. [shared-core implementer playbook](../../shared-native-core/11-implementer-playbook.md);
7. current source and tests for Rust, desktop, and iOS implementations in the
   assigned domain.

If the triage prior for the assigned workstream is **already owned** or
**stay platform-side**, do not produce a full implementation plan unless a
source-linked census overturns that prior. A short memo that confirms or
revises the prior is enough.

Historical plans are evidence, not current truth. Confirm every ownership and
completion claim against the current branch.

## Investigation sequence

1. **Restate the product behavior.** Describe what users observe, not which
   language currently implements it.
2. **Census current owners.** Identify sources, adapters, DTOs, event streams,
   persistence, tests, and platform integrations for all clients.
3. **Locate the earliest divergence.** Distinguish duplicated policy from
   necessary platform rendering and OS behavior.
4. **Challenge the move.** Explain why the behavior should remain where it is,
   then explain the best case for Rust ownership.
5. **Model boundaries.** Identify secrets, bytes, file paths, credentials,
   lifecycle constraints, latency budgets, and compatibility surfaces.
6. **Develop alternatives.** At minimum compare no change, targeted extraction,
   and a broader Core model. Reject a wholesale UI rewrite.
7. **Specify behavior.** Write requirements, invariants, typed models, errors,
   concurrency ownership, persistence, upgrades, and failure behavior.
8. **Design proof.** Cover the test pyramid and live end-to-end evidence needed
   to demonstrate that both clients consume one owner.
9. **Decompose delivery.** Prefer independently reviewable vertical slices,
   with an explicit cutover and removal of superseded owners.
10. **Recommend.** Return one of: already correctly owned, stay platform-side,
    extract a bounded subset, proceed with the proposed Core ownership, or
    requires an ADR decision first.

## Required deliverable

Copy [PLAN-TEMPLATE.md](PLAN-TEMPLATE.md) into a new `plans/` subdirectory using
the workstream ID and a descriptive name. Complete every applicable section.
Do not erase uncomfortable unknowns. Mark them as decision blockers and state
the evidence required to resolve them.

The plan must include:

- source paths and named current owners;
- feature and functional requirements;
- non-functional and security requirements;
- a typed API/DTO sketch when relevant;
- backward/forward compatibility and rollout strategy;
- exhaustive automated and manual validation proposals;
- failure injection, recovery, rollback, and observability;
- dependencies, estimates by slice, and explicit stop conditions;
- review objections and the response to each;
- a final recommendation with confidence and unresolved risks.

## Review protocol

Use at least two perspectives:

- an implementer review for feasibility, completeness, and testability;
- an adversarial boundary review looking for UI leakage, secret/byte transport,
  dual ownership, latency regressions, stale assumptions, and oversized scope.

If the recommendation changes an accepted boundary, draft a new ADR as part of
the proposal. Do not edit or silently reinterpret an accepted ADR.

## Implementation handoff gate

Implementation may begin only after the plan identifies:

- an approved owner and reviewer;
- accepted requirements and exclusions;
- the first bounded vertical slice;
- required CI and live proof;
- rollback and removal criteria;
- whether an ADR is required and, if so, its accepted decision.
