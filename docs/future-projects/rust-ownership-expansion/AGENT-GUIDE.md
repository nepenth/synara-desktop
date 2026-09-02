# Agent Guide: Investigating Residual Rust Ownership

Use this guide only after a human explicitly charters a docs-only research
question. The default deliverable is a short decision memo, not code or an
implementation plan.

## Required reading

Read these completely before drawing a boundary:

1. [project charter and portfolio priors](README.md);
2. the assigned files under [`workstreams/`](workstreams/);
3. the [ADR index and lifecycle rules](../../adr/README.md);
4. [ADR 0001](../../adr/0001-ios-repository-layout.md) for repository ownership;
5. [ADR 0002](../../adr/0002-ios-architecture.md) for the native iOS boundary;
6. [ADR 0003](../../adr/0003-shared-native-rust-core.md);
7. [ADR 0004](../../adr/0004-rust-language-boundaries.md);
8. [ADR 0005](../../adr/0005-native-media-handle-channel.md) for any media or
   byte/path question;
9. [shared-core implementer playbook](../../shared-native-core/11-implementer-playbook.md)
   and the current [goal graph](../../shared-native-core/13-language-boundary-goal-graph.md)
   for sequencing and stop conditions;
10. current source and tests for Rust, desktop, and iOS in the assigned domain.

Accepted decisions and current source supersede historical plans. Do not infer
unfinished work from old command counts, migration prose, or a platform
adapter.

## Research constraints

- Work on at most one deep cluster at a time.
- Research changes are docs-only under `docs/future-projects/**`.
- Do not create Core routes, commands, DTOs, UniFFI methods, tests, or product
  code during research.
- Do not create or update shared-Core phase, scoreboard, release-gate, or
  acceptance state. Read the current goal graph instead of copying historical
  phase identifiers into a memo.
- Treat platform observation and rendering as intentional ownership, not
  duplication.
- Default to staying platform-side unless current evidence proves harmful
  duplicated authority.

## Investigation sequence

1. **Restate observable behavior.** Avoid language-first framing.
2. **Classify it twice.** Mark each concern as Core authority, platform
   observation, or platform rendering. Then mark the applicable constraint as
   a hard invariant, accepted platform boundary, or current technology
   preference.
3. **Census current owners.** Link sources, adapters, DTOs/events, persistence,
   tests, and platform integrations for all clients.
4. **Locate the earliest divergence.** Separate competing policy from
   projection, rendering, and OS behavior.
5. **Check governing boundaries.** Identify secrets, media bytes, paths,
   lifecycle constraints, latency budgets, and accepted ADR decisions. Record
   the ADR review dates used; re-evaluate the memo if a relevant ADR changes.
6. **Compare alternatives.** At minimum: no change, a bounded extraction, and
   a broader Core model. State the strongest case for no ownership change.
7. **Recommend before designing delivery.** Choose: already correctly owned;
   stay platform-side; extract a bounded subset; proceed with Core ownership;
   or requires a product/ADR decision.
8. **Define proof for the recommendation.** For a close/stay decision, specify
   enough regression evidence to keep the boundary stable. For proceed,
   identify the evidence needed before planning.
9. **Stop after the memo.** Do not sketch commands, DTOs, slices, or rollout
   unless the proceed recommendation is explicitly accepted.

## Required first deliverable

Copy [RESEARCH-MEMO-TEMPLATE.md](RESEARCH-MEMO-TEMPLATE.md) to
`memos/ROE-XX-short-name-memo.md`. For a cluster, list the relevant IDs in the
memo and use the lowest ID in the filename. Record the source census commit and
date. Unknowns must be explicit rather than filled with assumptions.

A memo that confirms **already correctly owned** or **stay platform-side** is a
complete and valuable result. Do not inflate it into an implementation plan.

## Review protocol

Use both perspectives:

- an ownership reviewer checks source accuracy and whether a second authority
  actually exists;
- an adversarial boundary reviewer looks for UI leakage, secret/byte
  transport, dual ownership, latency or schema churn, stale assumptions, and
  scope that exceeds the observed product problem.

If a recommendation changes an accepted boundary, it must propose—not silently
edit—an ADR amendment or replacement. Any move of composer/UI state, media
paths/bytes, OS delivery, or a full message AST requires an explicit boundary
decision even if the exact change is not named in an existing brief. A change
to a technology preference still needs a separately chartered product case,
but it must not be misrepresented as breaking a hard security invariant.

## Full-plan handoff gate

Use [PLAN-TEMPLATE.md](PLAN-TEMPLATE.md) only when all are true:

- the memo recommends a bounded Core ownership change;
- the recommendation has human approval;
- any required ADR amendment or replacement is accepted;
- an implementation owner and reviewers are assigned;
- required CI/live proof and the first bounded vertical slice are known.

Store accepted planning work as `plans/ROE-XX-short-name-plan.md`. The plan
must prevent an ambiguous dual-owner interval and define removal and rollback
criteria. Product implementation occurs in a separate, explicitly authorized
branch or task.
